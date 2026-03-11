#![allow(dead_code)]

use crate::models::playbook::{Playbook, PlaybookRule};
use crate::service::ai_service::vector_service::client::VoyagerClient;
use crate::service::ai_service::vector_service::qdrant::QdrantDocumentClient;
use crate::service::analytics_engine::playbook_analytics::PlaybookAnalytics;
use anyhow::{Context, Result};
use std::sync::Arc;

/// Playbook-specific vectorization functions
pub struct PlaybookVectorization {
    voyager_client: Arc<VoyagerClient>,
    qdrant_client: Arc<QdrantDocumentClient>,
}

impl PlaybookVectorization {
    pub fn new(
        voyager_client: Arc<VoyagerClient>,
        qdrant_client: Arc<QdrantDocumentClient>,
    ) -> Self {
        Self {
            voyager_client,
            qdrant_client,
        }
    }

    /// Vectorize a playbook with its rules and analytics
    /// Formats comprehensive content and stores in Qdrant
    pub async fn vectorize_playbook(
        &self,
        user_id: &str,
        playbook: &Playbook,
        rules: &[PlaybookRule],
        analytics: Option<&PlaybookAnalytics>,
    ) -> Result<()> {
        log::info!(
            "Vectorizing playbook - user={}, playbook_id={}, rules_count={}, has_analytics={}",
            user_id,
            playbook.id,
            rules.len(),
            analytics.is_some()
        );

        // Format comprehensive content
        let content = format_playbook_content(playbook, rules, analytics);

        log::debug!(
            "Formatted playbook content - user={}, playbook_id={}, content_length={}",
            user_id,
            playbook.id,
            content.len()
        );

        // Generate embedding
        let embedding = self
            .voyager_client
            .embed_text(&content)
            .await
            .context("Failed to generate embedding for playbook")?;

        log::info!(
            "Embedding generated - user={}, playbook_id={}, embedding_dim={}",
            user_id,
            playbook.id,
            embedding.len()
        );

        // Create vector ID: playbook-{playbook_id}-context
        let vector_id = format!("playbook-{}-context", playbook.id);

        // Store in Qdrant
        self.qdrant_client
            .upsert_playbook_vector(user_id, &vector_id, &content, &embedding)
            .await
            .context("Failed to store playbook vector in Qdrant")?;

        log::info!(
            "Successfully vectorized playbook - user={}, playbook_id={}, vector_id={}",
            user_id,
            playbook.id,
            vector_id
        );

        Ok(())
    }
}

/// Format playbook content for vectorization
/// Combines metadata, rules, and analytics into comprehensive text
fn format_playbook_content(
    playbook: &Playbook,
    rules: &[PlaybookRule],
    analytics: Option<&PlaybookAnalytics>,
) -> String {
    let mut parts = Vec::new();

    // Playbook metadata
    parts.push(format!("Playbook: {}", playbook.name));

    if let Some(description) = &playbook.description && !description.trim().is_empty() {
        parts.push(format!("Description: {}", description));
    }

    // Group rules by type
    let entry_criteria: Vec<&PlaybookRule> = rules
        .iter()
        .filter(|r| {
            matches!(
                r.rule_type,
                crate::models::playbook::RuleType::EntryCriteria
            )
        })
        .collect();

    let exit_criteria: Vec<&PlaybookRule> = rules
        .iter()
        .filter(|r| matches!(r.rule_type, crate::models::playbook::RuleType::ExitCriteria))
        .collect();

    let market_factors: Vec<&PlaybookRule> = rules
        .iter()
        .filter(|r| matches!(r.rule_type, crate::models::playbook::RuleType::MarketFactor))
        .collect();

    // Format entry criteria
    if !entry_criteria.is_empty() {
        parts.push("Entry Criteria:".to_string());
        for rule in entry_criteria {
            parts.push(format!("- {}", rule.title));
            if let Some(desc) = &rule.description && !desc.trim().is_empty() {
                parts.push(format!("  {}", desc));
            }
        }
    }

    // Format exit criteria
    if !exit_criteria.is_empty() {
        parts.push("Exit Criteria:".to_string());
        for rule in exit_criteria {
            parts.push(format!("- {}", rule.title));
            if let Some(desc) = &rule.description && !desc.trim().is_empty() {
                parts.push(format!("  {}", desc));
            }
        }
    }

    // Format market factors
    if !market_factors.is_empty() {
        parts.push("Market Factors:".to_string());
        for rule in market_factors {
            parts.push(format!("- {}", rule.title));
            if let Some(desc) = &rule.description && !desc.trim().is_empty() {
                parts.push(format!("  {}", desc));
            }
        }
    }

    // Add analytics if available
    if let Some(analytics) = analytics {
        parts.push("Performance Summary:".to_string());
        parts.push(format!(
            "- Total Trades: {} ({} stocks, {} options)",
            analytics.total_trades, analytics.stock_trades, analytics.option_trades
        ));
        parts.push(format!("- Win Rate: {:.1}%", analytics.win_rate));
        parts.push(format!("- Net P&L: ${:.2}", analytics.net_pnl));
        parts.push(format!("- Profit Factor: {:.2}", analytics.profit_factor));
        parts.push(format!(
            "- Expectancy: ${:.2} per trade",
            analytics.expectancy
        ));
        parts.push(format!(
            "- Average Winner: ${:.2}",
            analytics.average_winner
        ));
        parts.push(format!("- Average Loser: ${:.2}", analytics.average_loser));

        parts.push("Compliance:".to_string());
        parts.push(format!(
            "- Fully Compliant: {} trades ({:.1}% win rate)",
            analytics.fully_compliant_trades, analytics.fully_compliant_win_rate
        ));
        parts.push(format!(
            "- Partially Compliant: {} trades",
            analytics.partially_compliant_trades
        ));
        parts.push(format!(
            "- Non-Compliant: {} trades",
            analytics.non_compliant_trades
        ));

        parts.push(format!(
            "Missed Opportunities: {} trades",
            analytics.missed_trades
        ));
    }

    parts.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::playbook::RuleType;
    use chrono::Utc;

    #[test]
    fn test_playbook_content_format() {
        let playbook = Playbook {
            id: "test-123".to_string(),
            name: "Test Strategy".to_string(),
            description: Some("A test trading strategy".to_string()),
            icon: None,
            emoji: None,
            color: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let rules = vec![PlaybookRule {
            id: "rule-1".to_string(),
            playbook_id: "test-123".to_string(),
            rule_type: RuleType::EntryCriteria,
            title: "Price breaks above 20-day high".to_string(),
            description: Some("With 2x average volume".to_string()),
            order_position: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }];

        let content = format_playbook_content(&playbook, &rules, None);

        assert!(content.contains("Playbook: Test Strategy"));
        assert!(content.contains("Description: A test trading strategy"));
        assert!(content.contains("Entry Criteria:"));
        assert!(content.contains("Price breaks above 20-day high"));
    }

    #[test]
    fn test_vector_id_format() {
        let playbook_id = "playbook-123";
        let vector_id = format!("playbook-{}-context", playbook_id);
        assert_eq!(vector_id, "playbook-playbook-123-context");
    }
}
