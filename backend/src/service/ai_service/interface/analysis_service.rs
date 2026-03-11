use crate::models::ai::analysis::{
    AnalysisError, AnalysisFocus, AnalysisPeriod, AnalysisRecord, AnalysisResult,
    ComprehensiveAnalysis, FullAnalysis,
};
use crate::service::ai_service::model_connection::ModelSelector;
use crate::service::ai_service::model_connection::openrouter::MessageRole as OpenRouterMessageRole;
use crate::service::ai_service::vector_service::client::VoyagerClient;
use crate::service::ai_service::vector_service::qdrant::{
    QdrantDocumentClient, SearchResult, TradeSearchFilters,
};
use crate::turso::TursoClient;
use crate::websocket::{ConnectionManager, EventType, WsMessage};
use anyhow::{Context, Result};
use chrono::Utc;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

/// Trade Analysis Service for generating real-time trading insights
pub struct TradeAnalysisService {
    qdrant_client: Arc<QdrantDocumentClient>,
    voyager_client: Arc<VoyagerClient>,
    model_selector: Arc<tokio::sync::Mutex<ModelSelector>>,
    turso_client: Arc<TursoClient>,
}

impl TradeAnalysisService {
    const MD_INSTRUCTION: &'static str =
        "Return the answer in GitHub-flavored Markdown with clear headings and bullet lists.";
    /// Minimum trades required for any analysis -> minimum is 20 trades, this is required in order for the 
    /// AI system to give you accurate analysis
    pub const MIN_TRADES_FOR_ANALYSIS: usize = 20;

    /// Minimum trades required for pattern detection -> minium is 20 trades, this is required in order for the 
    /// AI system to give you accurate pattern analysis 
    pub const MIN_TRADES_FOR_PATTERNS: usize = 20;

    pub fn new(
        qdrant_client: Arc<QdrantDocumentClient>,
        voyager_client: Arc<VoyagerClient>,
        model_selector: Arc<tokio::sync::Mutex<ModelSelector>>,
        turso_client: Arc<TursoClient>,
    ) -> Self {
        Self {
            qdrant_client,
            voyager_client,
            model_selector,
            turso_client,
        }
    }

    /// Run full auto analysis (weaknesses, strengths, patterns, comprehensive)
    pub async fn analyze_full(
        &self,
        user_id: &str,
        period: AnalysisPeriod,
        ws_manager: Option<Arc<Mutex<ConnectionManager>>>,
    ) -> Result<FullAnalysis> {
        // helper to broadcast if manager provided
        let ws_manager_ref = ws_manager.clone();
        let send = move |event: EventType, data: serde_json::Value| {
            let ws_manager = ws_manager_ref.clone();
            async move {
                if let Some(manager) = ws_manager {
                    let mgr = manager.lock().await;
                    mgr.broadcast_to_user(user_id, WsMessage::new(event.clone(), data));
                }
            }
        };

        // announce start
        send(
            EventType::AiAnalysisProgress,
            json!({ "stage": "start", "period": period.description() }),
        )
        .await;

        send(
            EventType::AiAnalysisProgress,
            json!({ "stage": "weaknesses", "status": "generating" }),
        )
        .await;
        let weaknesses = self.analyze_weaknesses(user_id, period.clone()).await?;
        send(
            EventType::AiAnalysisSection,
            json!({
                "section": "weaknesses",
                "content": weaknesses.content,
                "trades_analyzed": weaknesses.trades_analyzed,
                "generated_at": weaknesses.generated_at,
                "period": weaknesses.period
            }),
        )
        .await;

        send(
            EventType::AiAnalysisProgress,
            json!({ "stage": "strengths", "status": "generating" }),
        )
        .await;
        let strengths = self.analyze_strengths(user_id, period.clone()).await?;
        send(
            EventType::AiAnalysisSection,
            json!({
                "section": "strengths",
                "content": strengths.content,
                "trades_analyzed": strengths.trades_analyzed,
                "generated_at": strengths.generated_at,
                "period": strengths.period
            }),
        )
        .await;

        send(
            EventType::AiAnalysisProgress,
            json!({ "stage": "patterns", "status": "generating" }),
        )
        .await;
        let patterns = self.analyze_patterns(user_id, period.clone()).await?;
        send(
            EventType::AiAnalysisSection,
            json!({
                "section": "patterns",
                "content": patterns.content,
                "trades_analyzed": patterns.trades_analyzed,
                "generated_at": patterns.generated_at,
                "period": patterns.period
            }),
        )
        .await;

        send(
            EventType::AiAnalysisProgress,
            json!({ "stage": "comprehensive", "status": "generating" }),
        )
        .await;
        let comprehensive = self.analyze_comprehensive(user_id, period.clone()).await?;
        send(
            EventType::AiAnalysisSection,
            json!({
                "section": "comprehensive",
                "content": comprehensive.content,
                "current_trades_count": comprehensive.current_trades_count,
                "previous_trades_count": comprehensive.previous_trades_count,
                "has_comparison": comprehensive.has_comparison,
                "generated_at": comprehensive.generated_at,
                "period": comprehensive.period
            }),
        )
        .await;

        send(
            EventType::AiAnalysisComplete,
            json!({ "stage": "complete", "period": period.description() }),
        )
        .await;

        Ok(FullAnalysis {
            period,
            weaknesses,
            strengths,
            patterns,
            comprehensive,
        })
    }

    /// Analyze trading weaknesses
    pub async fn analyze_weaknesses(
        &self,
        user_id: &str,
        period: AnalysisPeriod,
    ) -> Result<AnalysisResult> {
        let date_range = period.to_date_range();

        // Query for losing trades/mistakes using semantic search
        let query = "trading losses, mistakes, poor decisions, errors, bad entries, failed trades";
        let query_embedding = self
            .voyager_client
            .embed_text(query)
            .await
            .context("Failed to generate query embedding")?;

        let filters = TradeSearchFilters {
            date_range: Some(date_range),
            profit_loss_positive_only: Some(false), // Only losses
            profit_loss_min: None,
            profit_loss_max: None,
            symbol: None,
            trade_type: None,
        };

        let trades = self
            .qdrant_client
            .search_by_embedding_with_filters(user_id, &query_embedding, 20, filters)
            .await
            .map_err(|e| AnalysisError::QdrantUnavailable {
                message: e.to_string(),
            })?;

        // If no losing trades were found, return a friendly "no weaknesses" result
        if trades.is_empty() {
            return Ok(AnalysisResult {
                focus: AnalysisFocus::Weaknesses,
                period,
                content: "No losing trades were detected for this period. Great job managing risk — there are no weaknesses to report right now."
                    .to_string(),
                trades_analyzed: 0,
                generated_at: Utc::now().to_rfc3339(),
            });
        }

        // Validate trade count (allows empty only when min_required == 0)
        self.validate_trade_count(&trades, Self::MIN_TRADES_FOR_ANALYSIS, Some(period.clone()))?;

        // Build prompt and generate analysis
        let prompt = self.build_weakness_prompt(&trades, &period);
        let content = self.generate_analysis(prompt).await?;

        Ok(AnalysisResult {
            focus: AnalysisFocus::Weaknesses,
            period,
            content,
            trades_analyzed: trades.len(),
            generated_at: Utc::now().to_rfc3339(),
        })
    }

    /// Analyze trading strengths
    pub async fn analyze_strengths(
        &self,
        user_id: &str,
        period: AnalysisPeriod,
    ) -> Result<AnalysisResult> {
        let date_range = period.to_date_range();

        // Query for winning trades using semantic search
        let query = "profitable trades, successful decisions, wins, good entries, smart exits, successful strategies";
        let query_embedding = self
            .voyager_client
            .embed_text(query)
            .await
            .context("Failed to generate query embedding")?;

        let filters = TradeSearchFilters {
            date_range: Some(date_range),
            profit_loss_positive_only: Some(true), // Only profits
            profit_loss_min: None,
            profit_loss_max: None,
            symbol: None,
            trade_type: None,
        };

        let trades = self
            .qdrant_client
            .search_by_embedding_with_filters(user_id, &query_embedding, 20, filters)
            .await
            .map_err(|e| AnalysisError::QdrantUnavailable {
                message: e.to_string(),
            })?;

        // Gracefully handle no winning trades
        if trades.is_empty() {
            return Ok(AnalysisResult {
                focus: AnalysisFocus::Strengths,
                period,
                content: "No winning trades were detected for this period. Once you log profitable trades, we’ll highlight your strengths here."
                    .to_string(),
                trades_analyzed: 0,
                generated_at: Utc::now().to_rfc3339(),
            });
        }

        // Validate trade count (allows empty only when min_required == 0)
        self.validate_trade_count(&trades, Self::MIN_TRADES_FOR_ANALYSIS, Some(period.clone()))?;

        // Build prompt and generate analysis
        let prompt = self.build_strength_prompt(&trades, &period);
        let content = self.generate_analysis(prompt).await?;

        Ok(AnalysisResult {
            focus: AnalysisFocus::Strengths,
            period,
            content,
            trades_analyzed: trades.len(),
            generated_at: Utc::now().to_rfc3339(),
        })
    }

    /// Analyze trading patterns using ALL trades (not semantic search)
    pub async fn analyze_patterns(
        &self,
        user_id: &str,
        period: AnalysisPeriod,
    ) -> Result<AnalysisResult> {
        let date_range = period.to_date_range();

        // Get ALL trades in period (no semantic search - complete dataset)
        let trades = self
            .qdrant_client
            .get_all_trades_in_period(user_id, date_range, None, Some(1000))
            .await
            .map_err(|e| AnalysisError::QdrantUnavailable {
                message: e.to_string(),
            })?;

        // If there are no trades, return a graceful message
        if trades.is_empty() {
            return Ok(AnalysisResult {
                focus: AnalysisFocus::Patterns,
                period,
                content:
                    "No trades were found for this period, so there are no patterns to report yet."
                        .to_string(),
                trades_analyzed: 0,
                generated_at: Utc::now().to_rfc3339(),
            });
        }

        // Validate trade count (allows empty only when min_required == 0)
        self.validate_trade_count(&trades, Self::MIN_TRADES_FOR_PATTERNS, Some(period.clone()))?;

        // Build prompt and generate analysis
        let prompt = self.build_pattern_prompt(&trades, &period);
        let content = self.generate_analysis(prompt).await?;

        Ok(AnalysisResult {
            focus: AnalysisFocus::Patterns,
            period,
            content,
            trades_analyzed: trades.len(),
            generated_at: Utc::now().to_rfc3339(),
        })
    }

    /// Comprehensive analysis with period comparison
    pub async fn analyze_comprehensive(
        &self,
        user_id: &str,
        period: AnalysisPeriod,
    ) -> Result<ComprehensiveAnalysis> {
        let date_range = period.to_date_range();

        // Get current period trades
        let current_trades = self
            .qdrant_client
            .get_all_trades_in_period(user_id, date_range, None, Some(1000))
            .await
            .map_err(|e| AnalysisError::QdrantUnavailable {
                message: e.to_string(),
            })?;

        // If there are no trades at all for the period, return a lightweight message
        if current_trades.is_empty() {
            return Ok(ComprehensiveAnalysis {
                period,
                content: "No trades were found for this period, so there is no comprehensive analysis to show yet."
                    .to_string(),
                current_trades_count: 0,
                previous_trades_count: 0,
                has_comparison: false,
                generated_at: Utc::now().to_rfc3339(),
            });
        }

        // Validate trade count for current period (allows empty only when min_required == 0)
        self.validate_trade_count(
            &current_trades,
            Self::MIN_TRADES_FOR_ANALYSIS,
            Some(period.clone()),
        )?;

        // Get previous period trades for comparison
        let previous_period = period.previous();
        let previous_date_range = previous_period.to_date_range();
        let previous_trades = self
            .qdrant_client
            .get_all_trades_in_period(user_id, previous_date_range, None, Some(1000))
            .await
            .map_err(|e| AnalysisError::QdrantUnavailable {
                message: e.to_string(),
            })
            .unwrap_or_else(|_| Vec::new()); // Allow previous period to be empty

        // Build prompt and generate analysis
        let prompt = self.build_comprehensive_prompt(
            &current_trades,
            &previous_trades,
            &period,
            &previous_period,
        );
        let content = self.generate_analysis(prompt).await?;

        Ok(ComprehensiveAnalysis {
            period,
            content,
            current_trades_count: current_trades.len(),
            previous_trades_count: previous_trades.len(),
            has_comparison: !previous_trades.is_empty(),
            generated_at: Utc::now().to_rfc3339(),
        })
    }

    /// Persist a rendered analysis to the user's database for later reporting
    pub async fn save_analysis_record(
        &self,
        user_id: &str,
        time_range: String,
        title: String,
        content: String,
    ) -> Result<String> {
        let conn = self
            .turso_client
            .get_user_database_connection(user_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("User database not found"))?;

        let id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO ai_analysis (id, time_range, title, content) VALUES (?, ?, ?, ?)",
            libsql::params![id.clone(), time_range, title, content],
        )
        .await
        .context("Failed to save analysis record")?;

        Ok(id)
    }

    /// Fetch stored analysis records (newest first)
    pub async fn fetch_analysis_records(
        &self,
        user_id: &str,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<AnalysisRecord>> {
        let conn = self
            .turso_client
            .get_user_database_connection(user_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("User database not found"))?;

        let limit = limit.unwrap_or(50);
        let offset = offset.unwrap_or(0);

        let mut rows = conn
            .prepare(
                "SELECT id, time_range, title, content, created_at
                 FROM ai_analysis
                 ORDER BY created_at DESC
                 LIMIT ? OFFSET ?",
            )
            .await?
            .query(libsql::params![limit, offset])
            .await?;

        let mut records = Vec::new();
        while let Some(row) = rows.next().await? {
            records.push(AnalysisRecord {
                id: row.get(0)?,
                time_range: row.get(1)?,
                title: row.get(2)?,
                content: row.get(3)?,
                created_at: row.get(4)?,
            });
        }

        Ok(records)
    }

    /// Fetch single analysis record by id
    pub async fn fetch_analysis_record_by_id(
        &self,
        user_id: &str,
        id: &str,
    ) -> Result<Option<AnalysisRecord>> {
        let conn = self
            .turso_client
            .get_user_database_connection(user_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("User database not found"))?;

        let mut rows = conn
            .prepare(
                "SELECT id, time_range, title, content, created_at
                 FROM ai_analysis
                 WHERE id = ?",
            )
            .await?
            .query(libsql::params![id])
            .await?;

        if let Some(row) = rows.next().await? {
            Ok(Some(AnalysisRecord {
                id: row.get(0)?,
                time_range: row.get(1)?,
                title: row.get(2)?,
                content: row.get(3)?,
                created_at: row.get(4)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Validate trade count meets minimum requirement
    fn validate_trade_count(
        &self,
        trades: &[SearchResult],
        min_required: usize,
        period: Option<AnalysisPeriod>,
    ) -> Result<()> {
        let count = trades.len();

        // If the caller allows zero (min_required == 0), don't fail when empty.
        if count == 0 && min_required > 0 {
            return Err(AnalysisError::NoTrades {
                period: period.unwrap_or(AnalysisPeriod::Last30Days),
            }
            .into());
        }

        if count < min_required {
            return Err(AnalysisError::InsufficientTrades {
                required: min_required,
                actual: count,
            }
            .into());
        }

        Ok(())
    }

    /// Build prompt for weakness analysis
    fn build_weakness_prompt(&self, trades: &[SearchResult], period: &AnalysisPeriod) -> String {
        let trades_text = self.format_trades_for_prompt(trades);

        format!(
            r#"You are analyzing a trader's weaknesses over the {} period.
{}

TRADES AND MISTAKES:
{}

Provide a detailed weakness analysis:

1. TOP 3 WEAKNESSES
   - For each weakness, provide:
     * Clear description
     * Specific example from the trades above
     * Impact on trading performance

2. ROOT CAUSES
   - Why are these weaknesses occurring?
   - Any psychological patterns?

3. SEVERITY RATING
   - Rate each weakness: Critical / Moderate / Minor

4. PRIORITY ORDER
   - Which weakness to address first and why?

Be specific. Reference actual trades. Use concrete numbers when possible.
Provide actionable insights, not generic advice."#,
            Self::MD_INSTRUCTION,
            period.description(),
            trades_text
        )
    }

    /// Build prompt for strength analysis
    fn build_strength_prompt(&self, trades: &[SearchResult], period: &AnalysisPeriod) -> String {
        let trades_text = self.format_trades_for_prompt(trades);

        format!(
            r#"You are analyzing a trader's strengths over the {} period.
{}

SUCCESSFUL TRADES:
{}

Provide a detailed strength analysis:

1. TOP 3 STRENGTHS
   - For each strength, provide:
     * Clear description
     * Specific example from the trades above
     * Contribution to success

2. CONSISTENCY
   - Are these strengths consistently applied?
   - Which strength is most reliable?

3. LEVERAGE OPPORTUNITIES
   - How can they double down on these strengths?
   - What to keep doing?

Be specific. Celebrate wins. Reference actual successful trades."#,
            Self::MD_INSTRUCTION,
            period.description(),
            trades_text
        )
    }

    /// Build prompt for pattern detection
    fn build_pattern_prompt(&self, trades: &[SearchResult], period: &AnalysisPeriod) -> String {
        let trades_text = self.format_trades_for_prompt(trades);

        format!(
            r#"You are a pattern detection expert analyzing {} trades over {}.
{}

ALL TRADES:
{}

Detect and analyze patterns:

1. BEHAVIORAL PATTERNS
   - Repeated behaviors (good and bad)
   - Timing patterns
   - Decision-making patterns

2. MARKET CONDITION PATTERNS
   - When do they succeed? (market up/down/sideways)
   - When do they struggle?

3. TRADE SETUP PATTERNS
   - What setups work best?
   - What setups lead to losses?

4. EMOTIONAL PATTERNS
   - Evidence of FOMO, fear, greed
   - Discipline vs impulse

5. TEMPORAL PATTERNS
   - Day of week patterns (e.g., loses on Fridays)
   - Time of day patterns
   - Market condition timing

6. FREQUENCY ANALYSIS
   - How often does each pattern occur?
   - Most critical pattern to address

Be specific. Quantify when possible. Reference actual trades as examples."#,
            Self::MD_INSTRUCTION,
            trades.len(),
            period.description(),
            trades_text
        )
    }

    /// Build comprehensive analysis prompt with comparison
    fn build_comprehensive_prompt(
        &self,
        current_trades: &[SearchResult],
        previous_trades: &[SearchResult],
        current_period: &AnalysisPeriod,
        previous_period: &AnalysisPeriod,
    ) -> String {
        let current_text = self.format_trades_for_prompt(current_trades);
        let previous_text = if !previous_trades.is_empty() {
            self.format_trades_for_prompt(previous_trades)
        } else {
            "No data available".to_string()
        };

        let comparison_section = if !previous_trades.is_empty() {
            format!(
                r#"
5. PROGRESS ASSESSMENT
   - Compare {} vs {}
   - What improved?
   - What declined?
   - Overall trajectory: Improving / Declining / Stable
"#,
                current_period.description(),
                previous_period.description()
            )
        } else {
            String::new()
        };

        format!(
            r#"You are providing comprehensive trading analysis.
{}

CURRENT PERIOD ({current_period}):
{current_text}

PREVIOUS PERIOD ({previous_period}):
{previous_text}

Provide comprehensive analysis:

1. TOP 3 WEAKNESSES
   - Specific examples
   - Impact on performance

2. TOP 3 STRENGTHS
   - What's working well
   - Keep doing this

3. RECURRING PATTERNS
   - Both good and bad patterns
   - Frequency and impact

4. ACTIONABLE RECOMMENDATIONS
   - 3-5 specific, implementable actions
   - Prioritized by impact
{comparison_section}

Be specific, actionable, and reference actual trades. Use data when available."#,
            Self::MD_INSTRUCTION,
            current_period = current_period.description(),
            previous_period = previous_period.description(),
            current_text = current_text,
            previous_text = previous_text,
            comparison_section = comparison_section
        )
    }

    /// Format trades for AI prompt
    fn format_trades_for_prompt(&self, trades: &[SearchResult]) -> String {
        trades
            .iter()
            .enumerate()
            .map(|(i, trade)| {
                format!(
                    "Trade {}:\n{}\nRelevance: {:.2}\n",
                    i + 1,
                    trade.content,
                    trade.score
                )
            })
            .collect::<Vec<_>>()
            .join("\n---\n\n")
    }

    /// Generate analysis using AI with retry logic
    async fn generate_analysis(&self, prompt: String) -> Result<String> {
        let messages = vec![
            crate::service::ai_service::model_connection::openrouter::ChatMessage {
                role: OpenRouterMessageRole::User,
                content: prompt,
            },
        ];

        // Retry logic: 3 attempts with exponential backoff
        let mut last_error = None;
        for attempt in 0..3 {
            let mut selector = self.model_selector.lock().await;
            match selector.generate_chat_with_fallback(messages.clone()).await {
                Ok(response) => {
                    if response.trim().is_empty() {
                        if attempt < 2 {
                            let delay_ms = 1000 * 2_u64.pow(attempt);
                            log::warn!(
                                "AI returned empty response, retrying in {}ms (attempt {})",
                                delay_ms,
                                attempt + 1
                            );
                            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                            continue;
                        } else {
                            return Err(AnalysisError::AIGeneration {
                                message: "AI returned empty response after 3 attempts".to_string(),
                            }
                            .into());
                        }
                    }
                    return Ok(response);
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < 2 {
                        let delay_ms = 1000 * 2_u64.pow(attempt);
                        log::warn!(
                            "AI generation failed, retrying in {}ms (attempt {}): {}",
                            delay_ms,
                            attempt + 1,
                            last_error.as_ref().unwrap()
                        );
                        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                    }
                }
            }
        }

        Err(AnalysisError::AIGeneration {
            message: format!(
                "AI generation failed after 3 attempts: {}",
                last_error
                    .map(|e| e.to_string())
                    .unwrap_or_else(|| "Unknown error".to_string())
            ),
        }
        .into())
    }
}
