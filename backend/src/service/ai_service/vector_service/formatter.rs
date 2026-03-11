#![allow(dead_code)]

use crate::models::notebook::notebook_notes::NotebookNote;
use crate::models::notes::trade_notes::TradeNote;
use crate::models::options::OptionTrade;
use crate::models::playbook::Playbook;
use crate::models::stock::stocks::Stock;
use crate::service::ai_service::vector_service::qdrant::{Document, DocumentMetadata};
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Data type enum for vectorization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataType {
    Stock,
    Option,
    TradeNote,
    NotebookEntry,
    PlaybookStrategy,
}

/// Data formatter for converting trading data to text for embeddings
pub struct DataFormatter;

impl DataFormatter {
    /// Format mistakes and trade notes into combined text for vectorization
    pub fn format_mistakes_and_notes(
        mistakes: Option<String>,
        notes: Vec<TradeNote>,
    ) -> Result<String> {
        let mut parts = Vec::new();

        // Add mistakes section if exists
        if let Some(mistakes_text) = mistakes && !mistakes_text.trim().is_empty() {
            parts.push(format!("Mistakes: {}", mistakes_text));
        }

        // Add trade notes section if exists
        if !notes.is_empty() {
            let notes_text: Vec<String> = notes
                .iter()
                .map(|note| format!("{}: {}", note.name, note.content))
                .collect();
            parts.push(format!("Trade Notes:\n{}", notes_text.join("\n")));
        }

        let combined = parts.join("\n\n");

        // Return empty string if no content
        if combined.trim().is_empty() {
            Ok(String::new())
        } else {
            Ok(combined)
        }
    }

    /// Extract trade content from mistakes and notes
    pub fn extract_trade_content(
        mistakes: Option<String>,
        notes: Vec<TradeNote>,
    ) -> Result<String> {
        Self::format_mistakes_and_notes(mistakes, notes)
    }

    /// Format stock trade for embedding
    pub fn format_stock_for_embedding(stock: &Stock) -> String {
        let entry_date = stock.entry_date.format("%Y-%m-%d").to_string();
        let exit_date = stock.exit_date.format("%Y-%m-%d").to_string();

        // Calculate P&L from entry and exit prices
        let pnl = (stock.exit_price - stock.entry_price) * stock.number_shares - stock.commissions;

        let pnl_percentage = if stock.entry_price > 0.0 {
            (pnl / (stock.entry_price * stock.number_shares)) * 100.0
        } else {
            0.0
        };

        let is_closed =
            stock.exit_price != stock.entry_price || stock.exit_date != stock.entry_date;

        format!(
            "Stock trade: {} {:.2} shares of {} at ${:.2} on {}. Exit: {} at ${:.2} on {}. P&L: ${:.2} ({:.2}% {}). Stop Loss: ${:.2}. Commissions: ${:.2}",
            stock.trade_type,
            stock.number_shares,
            stock.symbol,
            stock.entry_price,
            entry_date,
            if is_closed { "SELL" } else { "HOLD" },
            stock.exit_price,
            exit_date,
            pnl,
            pnl_percentage,
            if pnl >= 0.0 { "gain" } else { "loss" },
            stock.stop_loss,
            stock.commissions
        )
    }

    /// Format option trade for embedding
    pub fn format_option_for_embedding(option: &OptionTrade) -> String {
        let entry_date = option.entry_date.format("%Y-%m-%d").to_string();
        let exit_date = option.exit_date.format("%Y-%m-%d").to_string();

        let quantity = option.total_quantity.unwrap_or(0.0);

        // Calculate P&L from entry and exit prices
        let pnl = (option.exit_price - option.entry_price) * quantity * 100.0;

        let pnl_percentage = if option.entry_price > 0.0 && quantity > 0.0 {
            (pnl / (option.entry_price * quantity * 100.0)) * 100.0
        } else {
            0.0
        };

        let is_closed =
            option.exit_price != option.entry_price || option.exit_date != option.entry_date;

        format!(
            "Option trade: {} {} contracts of {} (Strike: ${:.2}, Expiry: {}) at ${:.2} on {}. Exit: {} at ${:.2} on {}. P&L: ${:.2} ({:.2}% {}). Premium: ${:.2}",
            quantity,
            option.option_type,
            option.symbol,
            option.strike_price,
            option.expiration_date.format("%Y-%m-%d"),
            option.entry_price,
            entry_date,
            if is_closed { "CLOSE" } else { "HOLD" },
            option.exit_price,
            exit_date,
            pnl,
            pnl_percentage,
            if pnl >= 0.0 { "gain" } else { "loss" },
            option.premium
        )
    }

    /// Format trade note for embedding
    pub fn format_trade_note_for_embedding(note: &TradeNote) -> String {
        format!("Trade note: {} - {}", note.name, note.content)
    }

    /// Format notebook entry for embedding
    pub fn format_notebook_for_embedding(notebook: &NotebookNote) -> String {
        format!("Notebook entry: {} - {}", notebook.title, notebook.content)
    }

    /// Format playbook strategy for embedding
    pub fn format_playbook_for_embedding(playbook: &Playbook) -> String {
        format!(
            "Trading strategy: {} - {}",
            playbook.name,
            playbook.description.as_deref().unwrap_or("No description")
        )
    }

    /// Format stock trade for search document
    pub fn format_stock_for_search(stock: &Stock) -> Document {
        let entry_date = stock.entry_date.format("%Y-%m-%d").to_string();
        let exit_date = stock.exit_date.format("%Y-%m-%d").to_string();

        // Calculate P&L from entry and exit prices
        let pnl = (stock.exit_price - stock.entry_price) * stock.number_shares - stock.commissions;

        let pnl_percentage = if stock.entry_price > 0.0 {
            (pnl / (stock.entry_price * stock.number_shares)) * 100.0
        } else {
            0.0
        };

        let is_closed =
            stock.exit_price != stock.entry_price || stock.exit_date != stock.entry_date;

        let mut content = HashMap::new();
        content.insert(
            "title".to_string(),
            format!("{} {} Trade", stock.symbol, stock.trade_type),
        );
        content.insert("description".to_string(), format!(
            "Stock trade: {} {:.2} shares of {} at ${:.2} on {}. Exit: {} at ${:.2} on {}. P&L: ${:.2} ({:.2}% {}). Stop Loss: ${:.2}. Commissions: ${:.2}",
            stock.trade_type,
            stock.number_shares,
            stock.symbol,
            stock.entry_price,
            entry_date,
            if is_closed { "SELL" } else { "HOLD" },
            stock.exit_price,
            exit_date,
            pnl,
            pnl_percentage,
            if pnl >= 0.0 { "gain" } else { "loss" },
            stock.stop_loss,
            stock.commissions
        ));
        content.insert("symbol".to_string(), stock.symbol.clone());
        content.insert("trade_type".to_string(), format!("{:?}", stock.trade_type));
        content.insert("pnl".to_string(), format!("{:.2}", pnl));
        content.insert(
            "pnl_percentage".to_string(),
            format!("{:.2}", pnl_percentage),
        );
        content.insert(
            "entry_price".to_string(),
            format!("{:.2}", stock.entry_price),
        );
        content.insert("exit_price".to_string(), format!("{:.2}", stock.exit_price));
        content.insert("shares".to_string(), format!("{:.2}", stock.number_shares));
        content.insert("stop_loss".to_string(), format!("{:.2}", stock.stop_loss));
        content.insert(
            "commissions".to_string(),
            format!("{:.2}", stock.commissions),
        );

        let metadata = DocumentMetadata {
            user_id: "".to_string(), // Will be set by caller
            data_type: "stock".to_string(),
            entity_id: stock.id.to_string(),
            timestamp: Utc::now(),
            tags: Self::extract_tags(content.get("description").unwrap(), &DataType::Stock),
            content_hash: Self::generate_content_hash(content.get("description").unwrap()),
        };

        Document {
            id: format!("stock_{}", stock.id),
            content,
            metadata,
        }
    }

    /// Format option trade for search document
    pub fn format_option_for_search(option: &OptionTrade) -> Document {
        let entry_date = option.entry_date.format("%Y-%m-%d").to_string();
        let exit_date = option.exit_date.format("%Y-%m-%d").to_string();

        let quantity = option.total_quantity.unwrap_or(0.0);

        // Calculate P&L from entry and exit prices
        let pnl = (option.exit_price - option.entry_price) * quantity * 100.0;

        let pnl_percentage = if option.entry_price > 0.0 && quantity > 0.0 {
            (pnl / (option.entry_price * quantity * 100.0)) * 100.0
        } else {
            0.0
        };

        let is_closed =
            option.exit_price != option.entry_price || option.exit_date != option.entry_date;

        let mut content = HashMap::new();
        content.insert(
            "title".to_string(),
            format!("{} {} Option Trade", option.symbol, option.option_type),
        );
        content.insert("description".to_string(), format!(
            "Option trade: {} {} contracts of {} (Strike: ${:.2}, Expiry: {}) at ${:.2} on {}. Exit: {} at ${:.2} on {}. P&L: ${:.2} ({:.2}% {}). Premium: ${:.2}",
            quantity,
            option.option_type,
            option.symbol,
            option.strike_price,
            option.expiration_date.format("%Y-%m-%d"),
            option.entry_price,
            entry_date,
            if is_closed { "CLOSE" } else { "HOLD" },
            option.exit_price,
            exit_date,
            pnl,
            pnl_percentage,
            if pnl >= 0.0 { "gain" } else { "loss" },
            option.premium
        ));
        content.insert("symbol".to_string(), option.symbol.clone());
        content.insert(
            "option_type".to_string(),
            format!("{:?}", option.option_type),
        );
        content.insert(
            "strike_price".to_string(),
            format!("{:.2}", option.strike_price),
        );
        content.insert(
            "expiration_date".to_string(),
            option.expiration_date.format("%Y-%m-%d").to_string(),
        );
        content.insert("quantity".to_string(), format!("{:.2}", quantity));
        content.insert("pnl".to_string(), format!("{:.2}", pnl));
        content.insert(
            "pnl_percentage".to_string(),
            format!("{:.2}", pnl_percentage),
        );
        content.insert("premium".to_string(), format!("{:.2}", option.premium));

        let metadata = DocumentMetadata {
            user_id: "".to_string(), // Will be set by caller
            data_type: "option".to_string(),
            entity_id: option.id.to_string(),
            timestamp: Utc::now(),
            tags: Self::extract_tags(content.get("description").unwrap(), &DataType::Option),
            content_hash: Self::generate_content_hash(content.get("description").unwrap()),
        };

        Document {
            id: format!("option_{}", option.id),
            content,
            metadata,
        }
    }

    /// Format trade note for search document
    pub fn format_trade_note_for_search(note: &TradeNote) -> Document {
        let mut content = HashMap::new();
        content.insert("title".to_string(), note.name.clone());
        content.insert("description".to_string(), note.content.clone());
        content.insert("content".to_string(), note.content.clone());

        let metadata = DocumentMetadata {
            user_id: "".to_string(), // Will be set by caller
            data_type: "tradenote".to_string(),
            entity_id: note.id.clone(),
            timestamp: Utc::now(),
            tags: Self::extract_tags(&note.content, &DataType::TradeNote),
            content_hash: Self::generate_content_hash(&note.content),
        };

        Document {
            id: format!("note_{}", note.id),
            content,
            metadata,
        }
    }

    /// Format notebook entry for search document
    pub fn format_notebook_for_search(notebook: &NotebookNote) -> Document {
        let mut content = HashMap::new();
        content.insert("title".to_string(), notebook.title.clone());

        // Convert JSON content to string for storage
        let content_str =
            serde_json::to_string(&notebook.content).unwrap_or_else(|_| "".to_string());
        content.insert("description".to_string(), content_str.clone());
        content.insert("content".to_string(), content_str.clone());

        let metadata = DocumentMetadata {
            user_id: "".to_string(), // Will be set by caller
            data_type: "notebookentry".to_string(),
            entity_id: notebook.id.clone(),
            timestamp: Utc::now(),
            tags: Self::extract_tags(&content_str, &DataType::NotebookEntry),
            content_hash: Self::generate_content_hash(&content_str),
        };

        Document {
            id: format!("notebook_{}", notebook.id),
            content,
            metadata,
        }
    }

    /// Format playbook strategy for search document
    pub fn format_playbook_for_search(playbook: &Playbook) -> Document {
        let description = playbook.description.as_deref().unwrap_or("No description");

        let mut content = HashMap::new();
        content.insert("title".to_string(), playbook.name.clone());
        content.insert("description".to_string(), description.to_string());
        content.insert(
            "content".to_string(),
            format!("Trading strategy: {} - {}", playbook.name, description),
        );

        let metadata = DocumentMetadata {
            user_id: "".to_string(), // Will be set by caller
            data_type: "playbookstrategy".to_string(),
            entity_id: playbook.id.clone(),
            timestamp: Utc::now(),
            tags: Self::extract_tags(
                &format!("Trading strategy: {} - {}", playbook.name, description),
                &DataType::PlaybookStrategy,
            ),
            content_hash: Self::generate_content_hash(&format!(
                "Trading strategy: {} - {}",
                playbook.name, description
            )),
        };

        Document {
            id: format!("playbook_{}", playbook.id),
            content,
            metadata,
        }
    }

    /// Format any data type for search document
    pub fn format_for_search_document(
        data_type: DataType,
        data: &serde_json::Value,
    ) -> Result<Document, String> {
        match data_type {
            DataType::Stock => {
                let stock: Stock = serde_json::from_value(data.clone())
                    .map_err(|e| format!("Failed to parse stock data: {}", e))?;
                Ok(Self::format_stock_for_search(&stock))
            }
            DataType::Option => {
                let option: OptionTrade = serde_json::from_value(data.clone())
                    .map_err(|e| format!("Failed to parse option data: {}", e))?;
                Ok(Self::format_option_for_search(&option))
            }
            DataType::TradeNote => {
                let note: TradeNote = serde_json::from_value(data.clone())
                    .map_err(|e| format!("Failed to parse trade note data: {}", e))?;
                Ok(Self::format_trade_note_for_search(&note))
            }
            DataType::NotebookEntry => {
                let notebook: NotebookNote = serde_json::from_value(data.clone())
                    .map_err(|e| format!("Failed to parse notebook data: {}", e))?;
                Ok(Self::format_notebook_for_search(&notebook))
            }
            DataType::PlaybookStrategy => {
                let playbook: Playbook = serde_json::from_value(data.clone())
                    .map_err(|e| format!("Failed to parse playbook data: {}", e))?;
                Ok(Self::format_playbook_for_search(&playbook))
            }
        }
    }

    /// Generate content hash for change detection
    pub fn generate_content_hash(content: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Extract tags from content for better vector organization
    pub fn extract_tags(content: &str, data_type: &DataType) -> Vec<String> {
        let mut tags = Vec::new();

        // Add data type tag
        tags.push(format!("{:?}", data_type).to_lowercase());

        // Extract common trading terms
        let content_lower = content.to_lowercase();

        if content_lower.contains("profit") || content_lower.contains("gain") {
            tags.push("profitable".to_string());
        }
        if content_lower.contains("loss") || content_lower.contains("losing") {
            tags.push("loss".to_string());
        }
        if content_lower.contains("momentum") {
            tags.push("momentum".to_string());
        }
        if content_lower.contains("earnings") {
            tags.push("earnings".to_string());
        }
        if content_lower.contains("swing") {
            tags.push("swing".to_string());
        }
        if content_lower.contains("day trade") {
            tags.push("day_trade".to_string());
        }
        if content_lower.contains("option") {
            tags.push("options".to_string());
        }
        if content_lower.contains("call") {
            tags.push("call_option".to_string());
        }
        if content_lower.contains("put") {
            tags.push("put_option".to_string());
        }
        if content_lower.contains("strategy") {
            tags.push("strategy".to_string());
        }
        if content_lower.contains("risk") {
            tags.push("risk_management".to_string());
        }

        tags
    }

    /// Validate content before vectorization
    pub fn validate_content(content: &str) -> Result<(), String> {
        if content.trim().is_empty() {
            return Err("Content cannot be empty".to_string());
        }

        if content.len() > 32000 {
            return Err("Content too long for embedding (max 32K characters)".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_format_mistakes_and_notes() {
        let mistakes = Some("Entered too early".to_string());
        let notes = vec![TradeNote {
            id: "note1".to_string(),
            name: "Analysis".to_string(),
            content: "My loss on AMD was because...".to_string(),
            trade_type: Some("stock".to_string()),
            stock_trade_id: Some(1),
            option_trade_id: None,
            ai_metadata: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }];

        let result = DataFormatter::format_mistakes_and_notes(mistakes, notes).unwrap();
        assert!(result.contains("Mistakes:"));
        assert!(result.contains("Trade Notes:"));
        assert!(result.contains("Entered too early"));
        assert!(result.contains("My loss on AMD"));
    }

    #[test]
    fn test_format_stock_for_embedding() {
        let stock = Stock {
            id: 1,
            symbol: "AAPL".to_string(),
            trade_type: crate::models::stock::stocks::TradeType::BUY,
            order_type: crate::models::stock::stocks::OrderType::MARKET,
            entry_price: 150.0,
            exit_price: 160.0,
            stop_loss: 140.0,
            commissions: 5.0,
            number_shares: 100.0,
            take_profit: Some(170.0),
            initial_target: None,
            profit_target: None,
            trade_ratings: None,
            entry_date: Utc::now(),
            exit_date: Utc::now(),
            reviewed: false,
            mistakes: None,
            brokerage_name: None,
            trade_group_id: None,
            parent_trade_id: None,
            total_quantity: None,
            transaction_sequence: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let formatted = DataFormatter::format_stock_for_embedding(&stock);
        assert!(formatted.contains("AAPL"));
        assert!(formatted.contains("BUY"));
        assert!(formatted.contains("100.00"));
        assert!(formatted.contains("150.00"));
    }

    #[test]
    fn test_generate_content_hash() {
        let content = "test content";
        let hash1 = DataFormatter::generate_content_hash(content);
        let hash2 = DataFormatter::generate_content_hash(content);
        assert_eq!(hash1, hash2);

        let hash3 = DataFormatter::generate_content_hash("different content");
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_extract_tags() {
        let content = "This is a profitable momentum trade with strong earnings";
        let tags = DataFormatter::extract_tags(content, &DataType::Stock);

        assert!(tags.contains(&"stock".to_string()));
        assert!(tags.contains(&"profitable".to_string()));
        assert!(tags.contains(&"momentum".to_string()));
        assert!(tags.contains(&"earnings".to_string()));
    }

    #[test]
    fn test_validate_content() {
        assert!(DataFormatter::validate_content("valid content").is_ok());
        assert!(DataFormatter::validate_content("").is_err());
        assert!(DataFormatter::validate_content("   ").is_err());
    }
}
