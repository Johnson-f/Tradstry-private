#![allow(dead_code)]

use crate::models::notes::trade_notes::TradeNote;
use crate::models::options::OptionTrade;
use crate::models::stock::stocks::Stock;
use crate::service::ai_service::vector_service::client::VoyagerClient;
use crate::service::ai_service::vector_service::formatter::DataFormatter;
use crate::service::ai_service::vector_service::qdrant::QdrantDocumentClient;
use anyhow::{Context, Result};
use libsql::{Connection, params};
use std::sync::Arc;

/// Service for vectorizing trade mistakes and notes
pub struct TradeVectorService {
    voyager_client: Arc<VoyagerClient>,
    qdrant_client: Arc<QdrantDocumentClient>,
}

impl TradeVectorService {
    pub fn new(
        voyager_client: Arc<VoyagerClient>,
        qdrant_client: Arc<QdrantDocumentClient>,
    ) -> Self {
        Self {
            voyager_client,
            qdrant_client,
        }
    }

    /// Vectorize trade mistakes and associated trade notes
    pub async fn vectorize_trade_mistakes_and_notes(
        &self,
        user_id: &str,
        trade_id: i64,
        trade_type: &str, // "stock" or "option"
        conn: &Connection,
    ) -> Result<()> {
        log::info!(
            "Starting vectorization for trade - user={}, trade_id={}, trade_type={}",
            user_id,
            trade_id,
            trade_type
        );

        // Step 1: Get trade to extract mistakes
        let mistakes = match trade_type {
            "stock" => {
                match Stock::find_by_id(conn, trade_id).await {
                    Ok(Some(stock)) => stock.mistakes,
                    Ok(None) => {
                        log::warn!("Stock trade not found - trade_id={}", trade_id);
                        return Ok(()); // Trade doesn't exist, skip vectorization
                    }
                    Err(e) => {
                        log::error!(
                            "Failed to find stock trade - trade_id={}, error={}",
                            trade_id,
                            e
                        );
                        return Err(anyhow::anyhow!("Failed to find stock trade: {}", e));
                    }
                }
            }
            "option" => {
                match OptionTrade::find_by_id(conn, trade_id).await {
                    Ok(Some(option)) => option.mistakes,
                    Ok(None) => {
                        log::warn!("Option trade not found - trade_id={}", trade_id);
                        return Ok(()); // Trade doesn't exist, skip vectorization
                    }
                    Err(e) => {
                        log::error!(
                            "Failed to find option trade - trade_id={}, error={}",
                            trade_id,
                            e
                        );
                        return Err(anyhow::anyhow!("Failed to find option trade: {}", e));
                    }
                }
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Invalid trade_type: {}. Must be 'stock' or 'option'",
                    trade_type
                ));
            }
        };

        // Step 2: Query all trade notes linked to this trade
        let notes = self.get_trade_notes(conn, trade_id, trade_type).await?;

        // Step 3: Format combined content
        let content = DataFormatter::format_mistakes_and_notes(mistakes.clone(), notes.clone())
            .context("Failed to format mistakes and notes")?;

        log::debug!(
            "Formatted content for vectorization - user={}, trade_id={}, trade_type={}, content_length={}, mistakes_count={}, notes_count={}",
            user_id,
            trade_id,
            trade_type,
            content.len(),
            mistakes.as_ref().map(|m| m.len()).unwrap_or(0),
            notes.len()
        );

        // If no content (no mistakes and no notes), skip vectorization
        if content.trim().is_empty() {
            log::info!(
                "No content to vectorize - user={}, trade_id={}, trade_type={}",
                user_id,
                trade_id,
                trade_type
            );
            return Ok(());
        }

        log::debug!(
            "Content preview (first 200 chars) - user={}, trade_id={}, preview={}",
            user_id,
            trade_id,
            if content.len() > 200 {
                format!("{}...", &content[..200])
            } else {
                content.clone()
            }
        );

        // Step 4: Generate embedding
        log::info!(
            "Generating embedding - user={}, trade_id={}, content_length={}",
            user_id,
            trade_id,
            content.len()
        );
        let embedding = self
            .voyager_client
            .embed_text(&content)
            .await
            .context("Failed to generate embedding")?;

        log::info!(
            "Embedding generated - user={}, trade_id={}, embedding_dim={}",
            user_id,
            trade_id,
            embedding.len()
        );

        // Step 5: Store in Qdrant
        let vector_id = format!("trade-{}-mistakes-notes", trade_id);
        log::debug!(
            "Attempting to store vector in Qdrant - user={}, trade_id={}, vector_id={}, embedding_dim={}",
            user_id,
            trade_id,
            vector_id,
            embedding.len()
        );

        self.qdrant_client
            .upsert_trade_vector(user_id, &vector_id, &content, &embedding)
            .await
            .with_context(|| format!(
                "Failed to store vector in Qdrant - user={}, trade_id={}, vector_id={}, embedding_dim={}",
                user_id, trade_id, vector_id, embedding.len()
            ))?;

        log::info!(
            "Successfully vectorized trade - user={}, trade_id={}, vector_id={}",
            user_id,
            trade_id,
            vector_id
        );

        Ok(())
    }

    /// Get all trade notes linked to a trade
    async fn get_trade_notes(
        &self,
        conn: &Connection,
        trade_id: i64,
        trade_type: &str,
    ) -> Result<Vec<TradeNote>> {
        let sql = match trade_type {
            "stock" => {
                "SELECT id, name, content, trade_type, stock_trade_id, option_trade_id, ai_metadata, created_at, updated_at FROM trade_notes WHERE stock_trade_id = ?"
            }
            "option" => {
                "SELECT id, name, content, trade_type, stock_trade_id, option_trade_id, ai_metadata, created_at, updated_at FROM trade_notes WHERE option_trade_id = ?"
            }
            _ => {
                return Err(anyhow::anyhow!("Invalid trade_type: {}", trade_type));
            }
        };

        let mut rows = conn.prepare(sql).await?.query(params![trade_id]).await?;

        let mut notes = Vec::new();
        while let Some(row) = rows.next().await? {
            match TradeNote::from_row(&row) {
                Ok(note) => notes.push(note),
                Err(e) => {
                    log::warn!("Failed to parse trade note row: {}", e);
                    // Continue processing other notes
                }
            }
        }

        log::debug!(
            "Found {} trade notes for trade_id={}, trade_type={}",
            notes.len(),
            trade_id,
            trade_type
        );

        Ok(notes)
    }

    /// Delete trade vector when trade is deleted
    pub async fn delete_trade_vector(&self, user_id: &str, trade_id: i64) -> Result<()> {
        let vector_id = format!("trade-{}-mistakes-notes", trade_id);

        log::info!(
            "Deleting trade vector - user={}, trade_id={}, vector_id={}",
            user_id,
            trade_id,
            vector_id
        );

        self.qdrant_client
            .delete_trade_vector(user_id, &vector_id)
            .await
            .context("Failed to delete trade vector from Qdrant")?;

        log::info!(
            "Successfully deleted trade vector - user={}, trade_id={}, vector_id={}",
            user_id,
            trade_id,
            vector_id
        );

        Ok(())
    }

    /// Batch vectorize multiple trades
    pub async fn batch_vectorize_trades(
        &self,
        user_id: &str,
        trade_ids: Vec<(i64, String)>, // (trade_id, trade_type)
        conn: &Connection,
    ) -> Result<Vec<(i64, Result<()>)>> {
        let mut results = Vec::new();

        for (trade_id, trade_type) in trade_ids {
            let result = self
                .vectorize_trade_mistakes_and_notes(user_id, trade_id, &trade_type, conn)
                .await;
            results.push((trade_id, result));
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_vector_id_format() {
        let vector_id = format!("trade-{}-mistakes-notes", 123);
        assert_eq!(vector_id, "trade-123-mistakes-notes");
    }
}
