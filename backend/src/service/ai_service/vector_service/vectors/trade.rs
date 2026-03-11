use anyhow::{Context, Result};
use libsql::Connection;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::models::notes::trade_notes::TradeNote;
use crate::models::options::OptionTrade;
use crate::models::stock::stocks::Stock;
use crate::service::ai_service::vector_service::client::VoyagerClient;
use crate::service::ai_service::vector_service::qdrant::QdrantDocumentClient;

/// Analytics data for a trade
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeAnalytics {
    pub net_pnl: f64,
    pub net_pnl_percent: f64,
    pub position_size: f64,
    pub stop_loss_risk_percent: f64,
    pub risk_to_reward: f64,
    pub entry_price: f64,
    pub exit_price: f64,
    pub hold_time_days: f64,
}

/// Service for vectorizing trades with structured Qdrant payload
pub struct TradeVectorization {
    voyager_client: Arc<VoyagerClient>,
    qdrant_client: Arc<QdrantDocumentClient>,
}

impl TradeVectorization {
    pub fn new(
        voyager_client: Arc<VoyagerClient>,
        qdrant_client: Arc<QdrantDocumentClient>,
    ) -> Self {
        Self {
            voyager_client,
            qdrant_client,
        }
    }

    /// Main entry point: Vectorize a trade with structured payload
    pub async fn vectorize_trade(
        &self,
        user_id: &str,
        trade_id: i64,
        trade_type: &str, // "stock" or "option"
        conn: &Connection,
    ) -> Result<()> {
        log::info!(
            "Starting trade vectorization - user={}, trade_id={}, trade_type={}",
            user_id,
            trade_id,
            trade_type
        );

        // Step 1: Fetch trade from database
        let (symbol, analytics, entry_date, mistakes) = match trade_type {
            "stock" => {
                let stock = Stock::find_by_id(conn, trade_id)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to fetch stock trade: {}", e))?
                    .ok_or_else(|| anyhow::anyhow!("Stock trade not found: {}", trade_id))?;

                let analytics = Self::calculate_trade_analytics_from_stock(&stock)?;
                let entry_date = stock.entry_date;
                let mistakes = stock.mistakes;
                (stock.symbol.clone(), analytics, entry_date, mistakes)
            }
            "option" => {
                let option = OptionTrade::find_by_id(conn, trade_id)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to fetch option trade: {}", e))?
                    .ok_or_else(|| anyhow::anyhow!("Option trade not found: {}", trade_id))?;

                let analytics = Self::calculate_trade_analytics_from_option(&option)?;
                let entry_date = option.entry_date;
                let mistakes = option.mistakes;
                (option.symbol.clone(), analytics, entry_date, mistakes)
            }
            _ => {
                return Err(anyhow::anyhow!("Invalid trade_type: {}", trade_type));
            }
        };

        // Step 2: Fetch trade notes (if any)
        let trade_note = match trade_type {
            "stock" => TradeNote::find_by_stock_trade_id(conn, trade_id)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to fetch stock trade notes: {}", e))?,
            "option" => TradeNote::find_by_option_trade_id(conn, trade_id)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to fetch option trade notes: {}", e))?,
            _ => None,
        };

        // Step 4: Format content for embedding
        let embedding_content = Self::format_content_for_embedding(
            &symbol,
            trade_type,
            trade_id,
            &analytics,
            mistakes.as_deref(),
            trade_note.as_ref().map(|n| n.content.as_str()),
        );

        log::debug!(
            "Formatted embedding content - trade_id={}, length={}",
            trade_id,
            embedding_content.len()
        );

        // Step 5: Generate embedding
        let embedding = self
            .voyager_client
            .embed_text(&embedding_content)
            .await
            .context("Failed to generate embedding for trade")?;

        log::info!(
            "Generated embedding for trade - trade_id={}, embedding_dim={}",
            trade_id,
            embedding.len()
        );

        // Step 6: Create structured payload and store in Qdrant
        let vector_id = format!("trade-{}-mistakes-notes", trade_id);
        self.qdrant_client
            .upsert_trade_vector_structured(
                user_id,
                &vector_id,
                &symbol,
                trade_type,
                trade_id,
                analytics,
                mistakes.as_deref(),
                trade_note.as_ref().map(|n| n.content.as_str()),
                &embedding,
                Some(entry_date), // Use trade's entry_date instead of current time
            )
            .await
            .context("Failed to upsert trade vector to Qdrant")?;

        log::info!(
            "Successfully vectorized trade - user={}, trade_id={}, symbol={}",
            user_id,
            trade_id,
            &symbol
        );

        Ok(())
    }

    /// Calculate analytics from a stock trade
    fn calculate_trade_analytics_from_stock(stock: &Stock) -> Result<TradeAnalytics> {
        // Net P&L calculation
        let net_pnl = match stock.trade_type.to_string().as_str() {
            "BUY" => {
                (stock.exit_price - stock.entry_price) * stock.number_shares - stock.commissions
            }
            "SELL" => {
                (stock.entry_price - stock.exit_price) * stock.number_shares - stock.commissions
            }
            _ => 0.0,
        };

        // Net P&L percentage
        let net_pnl_percent = if stock.entry_price > 0.0 {
            (net_pnl / (stock.entry_price * stock.number_shares)) * 100.0
        } else {
            0.0
        };

        // Position size (total value at entry)
        let position_size = stock.entry_price * stock.number_shares;

        // Stop loss risk percentage
        let stop_loss_risk_percent = if stock.entry_price > 0.0 {
            match stock.trade_type.to_string().as_str() {
                "BUY" => ((stock.entry_price - stock.stop_loss) / stock.entry_price) * 100.0,
                "SELL" => ((stock.stop_loss - stock.entry_price) / stock.entry_price) * 100.0,
                _ => 0.0,
            }
        } else {
            0.0
        };

        // Risk to reward ratio
        let risk_amount = match stock.trade_type.to_string().as_str() {
            "BUY" => (stock.entry_price - stock.stop_loss) * stock.number_shares,
            "SELL" => (stock.stop_loss - stock.entry_price) * stock.number_shares,
            _ => 0.0,
        };

        let reward_amount = net_pnl;
        let risk_to_reward = if risk_amount.abs() > 0.0 {
            reward_amount / risk_amount.abs()
        } else {
            0.0
        };

        // Hold time in days
        let hold_time_days =
            (stock.exit_date.timestamp() - stock.entry_date.timestamp()) as f64 / 86400.0; // seconds in a day

        Ok(TradeAnalytics {
            net_pnl: (net_pnl * 100.0).round() / 100.0,
            net_pnl_percent: (net_pnl_percent * 100.0).round() / 100.0,
            position_size: (position_size * 100.0).round() / 100.0,
            stop_loss_risk_percent: (stop_loss_risk_percent * 100.0).round() / 100.0,
            risk_to_reward: (risk_to_reward * 100.0).round() / 100.0,
            entry_price: stock.entry_price,
            exit_price: stock.exit_price,
            hold_time_days: (hold_time_days * 10.0).round() / 10.0,
        })
    }

    /// Calculate analytics from an option trade
    fn calculate_trade_analytics_from_option(option: &OptionTrade) -> Result<TradeAnalytics> {
        // Get number of contracts (total_quantity or default to 1)
        let contracts = option.total_quantity.unwrap_or(1.0);

        // Net P&L calculation for options
        // For calls: profit when exit_price > entry_price
        // For puts: profit when entry_price > exit_price
        // Net P&L = (premium difference) * contracts * 100 (each contract = 100 shares)
        // Note: entry_price and exit_price are premiums per contract
        let premium_diff = match option.option_type {
            crate::models::options::OptionType::Call => {
                (option.exit_price - option.entry_price) * contracts * 100.0
            }
            crate::models::options::OptionType::Put => {
                (option.entry_price - option.exit_price) * contracts * 100.0
            }
        };

        // Note: Options don't have a commissions field in the model
        // If commissions are included in the premium, they're already accounted for
        let net_pnl = premium_diff;

        // Net P&L percentage (based on premium paid)
        let premium_paid = option.entry_price * contracts * 100.0; // Total premium paid
        let net_pnl_percent = if premium_paid > 0.0 {
            (net_pnl / premium_paid) * 100.0
        } else {
            0.0
        };

        // Position size (total premium paid at entry)
        let position_size = premium_paid;

        // Stop loss risk percentage
        // Options don't have stop_loss field, use initial_target or profit_target as proxy
        // If neither exists, set to 0
        let stop_loss_risk_percent = if option.entry_price > 0.0 {
            if let Some(initial_target) = option.initial_target {
                // Risk is the difference between entry and initial target
                let risk = (option.entry_price - initial_target).abs();
                (risk / option.entry_price) * 100.0
            } else {
                0.0
            }
        } else {
            0.0
        };

        // Risk to reward ratio
        let risk_amount = if let Some(initial_target) = option.initial_target {
            (option.entry_price - initial_target).abs() * contracts * 100.0
        } else {
            premium_paid // If no stop loss, risk is the entire premium
        };

        let reward_amount = net_pnl;
        let risk_to_reward = if risk_amount.abs() > 0.0 {
            reward_amount / risk_amount.abs()
        } else {
            0.0
        };

        // Hold time in days
        let hold_time_days =
            (option.exit_date.timestamp() - option.entry_date.timestamp()) as f64 / 86400.0; // seconds in a day

        Ok(TradeAnalytics {
            net_pnl: (net_pnl * 100.0).round() / 100.0,
            net_pnl_percent: (net_pnl_percent * 100.0).round() / 100.0,
            position_size: (position_size * 100.0).round() / 100.0,
            stop_loss_risk_percent: (stop_loss_risk_percent * 100.0).round() / 100.0,
            risk_to_reward: (risk_to_reward * 100.0).round() / 100.0,
            entry_price: option.entry_price,
            exit_price: option.exit_price,
            hold_time_days: (hold_time_days * 10.0).round() / 10.0,
        })
    }

    /// Format content for embedding (human-readable text)
    fn format_content_for_embedding(
        symbol: &str,
        trade_type: &str,
        trade_id: i64,
        analytics: &TradeAnalytics,
        mistakes: Option<&str>,
        trade_notes: Option<&str>,
    ) -> String {
        let mut content = format!("Trade Symbol: {} ({})\n", symbol, trade_type.to_uppercase());
        content.push_str(&format!("Trade ID: {}\n\n", trade_id));

        // Analytics section
        content.push_str("Analytics:\n");
        content.push_str(&format!(
            "- Net P&L: ${:.2} ({:.2}% return)\n",
            analytics.net_pnl, analytics.net_pnl_percent
        ));

        // Format position size differently for stocks vs options
        if trade_type == "stock" {
            content.push_str(&format!(
                "- Position Size: {:.0} shares (${:.2})\n",
                analytics.position_size / analytics.entry_price,
                analytics.position_size
            ));
        } else {
            // For options, position_size is total premium paid
            // Calculate contracts: position_size / (entry_price * 100)
            let contracts = if analytics.entry_price > 0.0 {
                analytics.position_size / (analytics.entry_price * 100.0)
            } else {
                0.0
            };
            content.push_str(&format!(
                "- Position Size: {:.0} contracts (${:.2} premium)\n",
                contracts, analytics.position_size
            ));
        }

        content.push_str(&format!(
            "- Stop Loss Risk: {:.2}%\n",
            analytics.stop_loss_risk_percent
        ));
        content.push_str(&format!(
            "- Risk/Reward Ratio: {:.1}:1\n",
            analytics.risk_to_reward
        ));
        content.push_str(&format!(
            "- Entry: ${:.2}, Exit: ${:.2}\n",
            analytics.entry_price, analytics.exit_price
        ));
        content.push_str(&format!(
            "- Hold Time: {:.1} days\n\n",
            analytics.hold_time_days
        ));

        // Mistakes section
        if let Some(mistakes_text) = mistakes && !mistakes_text.trim().is_empty() {
            content.push_str("Mistakes:\n");
            content.push_str(mistakes_text);
            content.push_str("\n\n");
        }

        // Trade notes section
        if let Some(notes_text) = trade_notes && !notes_text.trim().is_empty() {
            content.push_str("Trade Notes:\n");
            content.push_str(notes_text);
            content.push('\n');
        }

        content
    }
}
