#![allow(dead_code)]

//! Enhanced AI Chat Prompt Templates
//!
//! This module provides a sophisticated prompt template system for the AI chat functionality.
//! It includes query type detection, context formatting, and dynamic prompt selection.
//!
//! ## Features
//!
//! - **Query Type Detection**: Automatically detects the type of query (performance, risk, behavioral, market analysis)
//! - **Context Formatting**: Formats trading data context for AI consumption
//! - **Dynamic Prompts**: Selects appropriate system prompts based on query type
//! - **Configurable Templates**: Allows runtime configuration of prompt templates
//!
//! ## Usage Example
//!
//! ```rust
//! use crate::models::ai::chat_templates::{ChatPromptConfig, ContextFormatter};
//!
//! // Create a chat service with enhanced prompts
//! let mut chat_service = AIChatService::new(
//!     vectorization_service,
//!     gemini_client,
//!     turso_client,
//!     10, // max_context_vectors
//! );
//!
//! // Configure custom prompt templates
//! let mut config = ChatPromptConfig::default();
//! config.context_max_length = 5000;
//! config.include_relevance_scores = true;
//! chat_service.configure_prompts(config);
//!
//! // The service will automatically:
//! // 1. Detect query type from user message
//! // 2. Retrieve relevant context via vector search
//! // 3. Format context with appropriate template
//! // 4. Generate response with enhanced system prompt
//! ```

use serde::{Deserialize, Serialize};

/// System prompt template for chat interactions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemPromptTemplate {
    pub role: String,
    pub instructions: String,
    pub context_format: String,
    pub response_format: String,
    pub examples: Vec<String>,
}

/// Query-specific prompt template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPromptTemplate {
    pub query_patterns: Vec<String>,
    pub system_prompt: String,
    pub context_instructions: String,
    pub response_format: String,
    pub max_tokens: u32,
    pub temperature: f32,
}

/// Chat prompt configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatPromptConfig {
    pub default_template: QueryPromptTemplate,
    pub templates: Vec<QueryPromptTemplate>,
    pub context_max_length: usize,
    pub include_relevance_scores: bool,
}

impl SystemPromptTemplate {
    /// Create a trading assistant system prompt
    pub fn trading_assistant() -> Self {
        Self {
            role: "You are a professional trading analyst and personal finance advisor.".to_string(),
            instructions: "Analyze the user's trading data and provide personalized insights based on their actual trading history.".to_string(),
            context_format: "Based on your trading history: {context_data}".to_string(),
            response_format: "Provide actionable advice with specific references to their trades, include metrics when relevant.".to_string(),
            examples: vec![
                "When analyzing performance, reference specific trades and time periods.".to_string(),
                "Always consider risk management in your recommendations.".to_string(),
                "Use specific numbers and percentages when discussing performance.".to_string(),
                "Provide actionable next steps based on the analysis.".to_string(),
            ],
        }
    }
}

impl QueryPromptTemplate {
    /// Performance analysis template
    pub fn performance_analysis() -> Self {
        Self {
            query_patterns: vec![
                "best trade".to_string(),
                "worst trade".to_string(),
                "performance".to_string(),
                "profit".to_string(),
                "loss".to_string(),
                "win rate".to_string(),
                "return".to_string(),
                "profitability".to_string(),
            ],
            system_prompt: "You are analyzing trading performance. Focus on metrics, patterns, and actionable insights. Always reference specific trades and time periods.".to_string(),
            context_instructions: "Include all trades from the specified time period with P&L data, entry/exit prices, and dates.".to_string(),
            response_format: "Provide specific metrics (win rate, profit factor, average return), identify patterns, and give actionable recommendations.".to_string(),
            max_tokens: 2048,
            temperature: 0.6,
        }
    }

    /// Risk assessment template
    pub fn risk_assessment() -> Self {
        Self {
            query_patterns: vec![
                "risk".to_string(),
                "position size".to_string(),
                "leverage".to_string(),
                "exposure".to_string(),
                "drawdown".to_string(),
                "volatility".to_string(),
                "portfolio".to_string(),
            ],
            system_prompt: "You are a risk management specialist. Analyze position sizing, leverage, and risk exposure. Focus on risk metrics and management strategies.".to_string(),
            context_instructions: "Include all open positions and recent trades with size, leverage, and risk data.".to_string(),
            response_format: "Identify risk factors, calculate exposure metrics, and provide specific risk management recommendations.".to_string(),
            max_tokens: 1536,
            temperature: 0.5,
        }
    }

    /// Behavioral analysis template
    pub fn behavioral_analysis() -> Self {
        Self {
            query_patterns: vec![
                "pattern".to_string(),
                "strategy".to_string(),
                "behavior".to_string(),
                "habit".to_string(),
                "emotion".to_string(),
                "psychology".to_string(),
                "decision".to_string(),
            ],
            system_prompt: "You are a trading psychology expert. Analyze trading behavior, patterns, and decision-making processes. Focus on behavioral insights and psychological factors.".to_string(),
            context_instructions: "Include trade notes, timing patterns, and behavioral indicators from trading history.".to_string(),
            response_format: "Identify behavioral patterns, psychological factors, and provide recommendations for improving trading discipline.".to_string(),
            max_tokens: 2048,
            temperature: 0.8,
        }
    }

    /// Market analysis template
    pub fn market_analysis() -> Self {
        Self {
            query_patterns: vec![
                "market".to_string(),
                "trend".to_string(),
                "analysis".to_string(),
                "sector".to_string(),
                "stock".to_string(),
                "option".to_string(),
                "forecast".to_string(),
            ],
            system_prompt: "You are a market analyst. Analyze market trends, sector performance, and trading opportunities. Focus on market data and technical analysis.".to_string(),
            context_instructions: "Include market data, sector performance, and trading patterns from the user's portfolio.".to_string(),
            response_format: "Provide market analysis, identify trends, and suggest trading opportunities based on current market conditions.".to_string(),
            max_tokens: 2048,
            temperature: 0.7,
        }
    }

    /// General trading assistant template
    pub fn general_trading_assistant() -> Self {
        Self {
            query_patterns: vec!["general".to_string(), "help".to_string(), "advice".to_string()],
            system_prompt: "You are a professional trading assistant. Provide helpful, accurate, and personalized trading advice based on the user's trading history.".to_string(),
            context_instructions: "Include relevant trading data and context based on the user's query.".to_string(),
            response_format: "Provide clear, actionable advice with specific references to their trading data when relevant.".to_string(),
            max_tokens: 1536,
            temperature: 0.7,
        }
    }
}

impl ChatPromptConfig {
    /// Create default chat prompt configuration
    pub fn default() -> Self {
        Self {
            default_template: QueryPromptTemplate::general_trading_assistant(),
            templates: vec![
                QueryPromptTemplate::performance_analysis(),
                QueryPromptTemplate::risk_assessment(),
                QueryPromptTemplate::behavioral_analysis(),
                QueryPromptTemplate::market_analysis(),
                QueryPromptTemplate::general_trading_assistant(),
            ],
            context_max_length: 4000,
            include_relevance_scores: true,
        }
    }

    /// Detect query type and return appropriate template
    pub fn detect_query_type(&self, query: &str) -> &QueryPromptTemplate {
        let query_lower = query.to_lowercase();

        // Find the best matching template based on query patterns
        for template in &self.templates {
            for pattern in &template.query_patterns {
                if query_lower.contains(pattern) {
                    return template;
                }
            }
        }

        // Return default template if no match found
        &self.default_template
    }

    /// Get template by name
    pub fn get_template_by_name(&self, name: &str) -> Option<&QueryPromptTemplate> {
        self.templates.iter().find(|t| match name {
            "performance" => t.query_patterns.contains(&"performance".to_string()),
            "risk" => t.query_patterns.contains(&"risk".to_string()),
            "behavioral" => t.query_patterns.contains(&"pattern".to_string()),
            "market" => t.query_patterns.contains(&"market".to_string()),
            _ => false,
        })
    }
}

/// Context formatting utilities
pub struct ContextFormatter;

impl ContextFormatter {
    /// Format context sources for AI consumption
    pub fn format_context_sources(
        sources: &[crate::models::ai::chat::ContextSource],
        max_length: usize,
        include_scores: bool,
    ) -> String {
        if sources.is_empty() {
            return "No relevant trading data found.".to_string();
        }

        let mut formatted = String::new();
        formatted.push_str("## Relevant Trading Data:\n\n");

        // Group by data type using BTreeMap for consistent ordering
        let mut grouped: std::collections::BTreeMap<
            String,
            Vec<&crate::models::ai::chat::ContextSource>,
        > = std::collections::BTreeMap::new();
        for source in sources {
            grouped
                .entry(source.data_type.clone())
                .or_default()
                .push(source);
        }

        let mut current_length = formatted.len();

        for (data_type, source_items) in grouped {
            let section_header = format!("### {} Data:\n", data_type);
            if current_length + section_header.len() > max_length {
                break;
            }

            formatted.push_str(&section_header);
            current_length += section_header.len();

            for source_item in source_items {
                let score_text = if include_scores {
                    format!(" (Relevance: {:.2})", source_item.similarity_score)
                } else {
                    String::new()
                };

                let source_text = format!(
                    "- {}{}: {}\n",
                    source_item.entity_id, score_text, source_item.snippet
                );

                if current_length + source_text.len() > max_length {
                    formatted.push_str("... (truncated)\n");
                    break;
                }

                formatted.push_str(&source_text);
                current_length += source_text.len();
            }

            formatted.push('\n');
            current_length += 1;
        }

        formatted
    }

    /// Build enhanced system prompt
    pub fn build_system_prompt(template: &QueryPromptTemplate) -> String {
        format!(
            "{}\n\n{}\n\n{}\n\nInstructions:\n- Always reference specific trades when relevant\n- Provide actionable recommendations\n- Use specific numbers and metrics\n- Consider risk management in all advice",
            template.system_prompt, template.context_instructions, template.response_format
        )
    }
}
