use anyhow::Result;
use chrono::{DateTime, Utc};
use libsql::{Connection, params};
use serde::{Deserialize, Serialize};

use super::TradeTag;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeTagAssociation {
    pub trade_id: i64,
    pub tag_id: String,
    pub created_at: DateTime<Utc>,
}

impl TradeTagAssociation {
    pub async fn add_tag_to_stock_trade(
        conn: &Connection,
        stock_trade_id: i64,
        tag_id: &str,
    ) -> Result<bool> {
        // Check if association already exists
        let stmt = conn
            .prepare("SELECT 1 FROM stock_trade_tags WHERE stock_trade_id = ? AND tag_id = ?")
            .await?;
        let mut rows = stmt.query(params![stock_trade_id, tag_id]).await?;

        if rows.next().await?.is_some() {
            return Ok(false); // Already exists
        }

        conn.execute(
            "INSERT INTO stock_trade_tags (stock_trade_id, tag_id, created_at) VALUES (?, ?, ?)",
            params![stock_trade_id, tag_id, Utc::now().to_rfc3339()],
        )
        .await?;

        Ok(true)
    }

    pub async fn add_tag_to_option_trade(
        conn: &Connection,
        option_trade_id: i64,
        tag_id: &str,
    ) -> Result<bool> {
        // Check if association already exists
        let stmt = conn
            .prepare("SELECT 1 FROM option_trade_tags WHERE option_trade_id = ? AND tag_id = ?")
            .await?;
        let mut rows = stmt.query(params![option_trade_id, tag_id]).await?;

        if rows.next().await?.is_some() {
            return Ok(false); // Already exists
        }

        conn.execute(
            "INSERT INTO option_trade_tags (option_trade_id, tag_id, created_at) VALUES (?, ?, ?)",
            params![option_trade_id, tag_id, Utc::now().to_rfc3339()],
        )
        .await?;

        Ok(true)
    }

    pub async fn remove_tag_from_stock_trade(
        conn: &Connection,
        stock_trade_id: i64,
        tag_id: &str,
    ) -> Result<bool> {
        let result = conn
            .execute(
                "DELETE FROM stock_trade_tags WHERE stock_trade_id = ? AND tag_id = ?",
                params![stock_trade_id, tag_id],
            )
            .await?;

        Ok(result > 0)
    }

    pub async fn remove_tag_from_option_trade(
        conn: &Connection,
        option_trade_id: i64,
        tag_id: &str,
    ) -> Result<bool> {
        let result = conn
            .execute(
                "DELETE FROM option_trade_tags WHERE option_trade_id = ? AND tag_id = ?",
                params![option_trade_id, tag_id],
            )
            .await?;

        Ok(result > 0)
    }

    pub async fn get_tags_for_stock_trade(
        conn: &Connection,
        stock_trade_id: i64,
    ) -> Result<Vec<TradeTag>> {
        let stmt = conn
            .prepare(
                "SELECT t.id, t.category, t.name, t.color, t.description, t.created_at, t.updated_at
                 FROM trade_tags t
                 INNER JOIN stock_trade_tags stt ON t.id = stt.tag_id
                 WHERE stt.stock_trade_id = ?
                 ORDER BY t.category, t.name",
            )
            .await?;
        let mut rows = stmt.query(params![stock_trade_id]).await?;

        let mut tags = Vec::new();
        while let Some(row) = rows.next().await? {
            let created_at_str: String = row.get(5)?;
            let updated_at_str: String = row.get(6)?;

            tags.push(TradeTag {
                id: row.get(0)?,
                category: row.get(1)?,
                name: row.get(2)?,
                color: row.get(3)?,
                description: row.get(4)?,
                created_at: DateTime::parse_from_rfc3339(&created_at_str)
                    .map_err(|e| anyhow::anyhow!("Failed to parse created_at: {}", e))?
                    .with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&updated_at_str)
                    .map_err(|e| anyhow::anyhow!("Failed to parse updated_at: {}", e))?
                    .with_timezone(&Utc),
            });
        }

        Ok(tags)
    }

    pub async fn get_tags_for_option_trade(
        conn: &Connection,
        option_trade_id: i64,
    ) -> Result<Vec<TradeTag>> {
        let stmt = conn
            .prepare(
                "SELECT t.id, t.category, t.name, t.color, t.description, t.created_at, t.updated_at
                 FROM trade_tags t
                 INNER JOIN option_trade_tags ott ON t.id = ott.tag_id
                 WHERE ott.option_trade_id = ?
                 ORDER BY t.category, t.name",
            )
            .await?;
        let mut rows = stmt.query(params![option_trade_id]).await?;

        let mut tags = Vec::new();
        while let Some(row) = rows.next().await? {
            let created_at_str: String = row.get(5)?;
            let updated_at_str: String = row.get(6)?;

            tags.push(TradeTag {
                id: row.get(0)?,
                category: row.get(1)?,
                name: row.get(2)?,
                color: row.get(3)?,
                description: row.get(4)?,
                created_at: DateTime::parse_from_rfc3339(&created_at_str)
                    .map_err(|e| anyhow::anyhow!("Failed to parse created_at: {}", e))?
                    .with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&updated_at_str)
                    .map_err(|e| anyhow::anyhow!("Failed to parse updated_at: {}", e))?
                    .with_timezone(&Utc),
            });
        }

        Ok(tags)
    }
}

#[derive(Debug, Deserialize)]
pub struct AddTagsToTradeRequest {
    pub tag_ids: Vec<String>,
}
