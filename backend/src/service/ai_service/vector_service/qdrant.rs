use crate::turso::vector_config::QdrantConfig as AppQdrantConfig;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use qdrant_client::{
    Qdrant,
    qdrant::value::Kind,
    qdrant::{
        Condition, CreateCollection, Distance, FieldCondition, Filter, Match, PointId, PointStruct,
        PointsIdsList, PointsSelector, Range, ScrollPoints, SearchPoints, UpsertPoints, Value,
        VectorParams, VectorsConfig, vectors_config::Config,
    },
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub user_id: String,
    pub data_type: String,
    pub entity_id: String,
    pub timestamp: DateTime<Utc>,
    pub tags: Vec<String>,
    pub content_hash: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Document {
    pub id: String,
    pub content: HashMap<String, String>,
    pub metadata: DocumentMetadata,
}

/// Search result from Qdrant semantic search
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: String,
    pub score: f32,
    pub content: String,
    pub r#type: Option<String>, // "chat" or "trade"
    #[allow(dead_code)]
    pub created_at: Option<DateTime<Utc>>, // Timestamp from payload
}

// DateRange is now defined in crate::models::ai::analysis
// Re-export for convenience
pub use crate::models::ai::analysis::DateRange;

/// Filters for trade search with metadata
#[derive(Debug, Clone, Default)]
pub struct TradeSearchFilters {
    pub date_range: Option<DateRange>,
    pub profit_loss_min: Option<f64>,
    pub profit_loss_max: Option<f64>,
    pub profit_loss_positive_only: Option<bool>,
    pub symbol: Option<String>,
    pub trade_type: Option<String>,
}

/// Metadata filters for getting all trades (no semantic search)
#[derive(Debug, Clone, Default)]
pub struct TradeMetadataFilters {
    pub profit_loss_min: Option<f64>,
    pub profit_loss_max: Option<f64>,
    pub profit_loss_positive_only: Option<bool>,
    pub symbol: Option<String>,
    pub trade_type: Option<String>,
}

pub struct QdrantDocumentClient {
    client: Qdrant,
    config: AppQdrantConfig,
}

/// Helper function to check if an error is a timeout error
fn is_timeout_error(error: &dyn std::fmt::Display) -> bool {
    let error_str = format!("{}", error);
    error_str.contains("Timeout")
        || error_str.contains("timeout")
        || error_str.contains("Cancelled")
        || error_str.contains("timeout expired")
        || error_str.contains("operation was cancelled")
}

impl QdrantDocumentClient {
    pub async fn new(config: AppQdrantConfig) -> Result<Self> {
        // Create Qdrant client with proper configuration for self-hosted HTTP instances
        // For self-hosted Qdrant on plain HTTP, we need to use from_url which uses REST API
        let client = if !config.api_key.is_empty() {
            // With API key authentication
            Qdrant::from_url(&config.url)
                .api_key(config.api_key.as_str())
                .timeout(Duration::from_secs(config.timeout_seconds))
                .build()
                .context("Failed to create Qdrant client with API key")?
        } else {
            // Without authentication (for local/self-hosted instances)
            Qdrant::from_url(&config.url)
                .timeout(Duration::from_secs(config.timeout_seconds))
                .build()
                .context("Failed to create Qdrant client")?
        };

        if !config.api_key.is_empty() {
            log::info!(
                "Qdrant client initialized with API key authentication - URL: {}",
                config.url
            );
        } else {
            log::info!(
                "Qdrant client initialized without API key (unauthenticated) - URL: {}",
                config.url
            );
        }

        Ok(Self { client, config })
    }

    pub async fn ensure_collection(&self, user_id: &str) -> Result<()> {
        let collection_name = self.config.get_collection_name(user_id);

        log::debug!(
            "Checking if Qdrant collection exists - user={}, collection={}",
            user_id,
            collection_name
        );

        // Retry logic for list_collections with exponential backoff
        const MAX_RETRIES: u32 = 3;
        let mut last_error = None;

        for attempt in 0..MAX_RETRIES {
            match self.client.list_collections().await {
                Ok(cols) => {
                    if attempt > 0 {
                        log::info!(
                            "Successfully listed Qdrant collections after {} retries",
                            attempt
                        );
                    }
                    log::debug!(
                        "Successfully listed {} Qdrant collections",
                        cols.collections.len()
                    );

                    // Check if collection exists
                    let exists = cols.collections.iter().any(|c| c.name == collection_name);

                    if !exists {
                        log::info!(
                            "Creating Qdrant collection: {} (does not exist)",
                            collection_name
                        );

                        // Retry logic for create_collection
                        let mut create_attempt = 0;
                        loop {
                            match self
                                .client
                                .create_collection(CreateCollection {
                                    collection_name: collection_name.clone(),
                                    vectors_config: Some(VectorsConfig {
                                        config: Some(Config::Params(VectorParams {
                                            size: 1024, // Voyager embeddings are 1024 dimensions
                                            distance: Distance::Cosine.into(),
                                            ..Default::default()
                                        })),
                                    }),
                                    ..Default::default()
                                })
                                .await
                            {
                                Ok(_) => {
                                    if create_attempt > 0 {
                                        log::info!(
                                            "Qdrant collection created after {} retries",
                                            create_attempt
                                        );
                                    }
                                    log::info!(
                                        "Qdrant collection created successfully: {}",
                                        collection_name
                                    );
                                    break;
                                }
                                Err(e) => {
                                    let error_str = format!("{}", e);
                                    if is_timeout_error(&e) && create_attempt < MAX_RETRIES - 1 {
                                        create_attempt += 1;
                                        let delay = Duration::from_millis(
                                            1000 * 2_u64.pow(create_attempt - 1),
                                        );
                                        log::warn!(
                                            "Failed to create Qdrant collection (attempt {}/{}): {}. Retrying in {:?}...",
                                            create_attempt,
                                            MAX_RETRIES,
                                            error_str,
                                            delay
                                        );
                                        tokio::time::sleep(delay).await;
                                        continue;
                                    } else {
                                        log::error!(
                                            "Failed to create Qdrant collection - user={}, collection={}, error={}, error_debug={:?}",
                                            user_id,
                                            collection_name,
                                            e,
                                            e
                                        );
                                        return Err(anyhow::anyhow!(
                                            "Failed to create collection: {}",
                                            e
                                        ));
                                    }
                                }
                            }
                        }
                    } else {
                        log::debug!("Qdrant collection already exists: {}", collection_name);
                    }

                    return Ok(());
                }
                Err(e) => {
                    let error_str = format!("{}", e);
                    last_error = Some(e);

                    if is_timeout_error(&last_error.as_ref().unwrap()) && attempt < MAX_RETRIES - 1
                    {
                        let delay = Duration::from_millis(1000 * 2_u64.pow(attempt));
                        log::warn!(
                            "Failed to list Qdrant collections (attempt {}/{}): {}. Retrying in {:?}...",
                            attempt + 1,
                            MAX_RETRIES,
                            error_str,
                            delay
                        );
                        tokio::time::sleep(delay).await;
                        continue;
                    } else {
                        log::error!(
                            "Failed to list Qdrant collections - user={}, collection={}, error={}, error_debug={:?}",
                            user_id,
                            collection_name,
                            last_error.as_ref().unwrap(),
                            last_error
                        );
                        return Err(anyhow::anyhow!(
                            "Failed to list collections: {}",
                            last_error.unwrap()
                        ));
                    }
                }
            }
        }

        // This should never be reached, but handle it just in case
        let error_msg = last_error
            .map(|e| format!("{}", e))
            .unwrap_or_else(|| "unknown error".to_string());
        Err(anyhow::anyhow!(
            "Failed to list collections after {} retries: {}",
            MAX_RETRIES,
            error_msg
        ))
    }

    /// Upsert a trade vector with the new format: {user_id, id, content, embedding, type: "trade", created_at}
    pub async fn upsert_trade_vector(
        &self,
        user_id: &str,
        vector_id: &str,
        content: &str,
        embedding: &[f32],
    ) -> Result<()> {
        log::debug!(
            "Starting upsert_trade_vector - user={}, vector_id={}, content_length={}, embedding_dim={}",
            user_id,
            vector_id,
            content.len(),
            embedding.len()
        );

        self.ensure_collection(user_id)
            .await
            .context("Failed to ensure collection exists")?;
        let collection_name = self.config.get_collection_name(user_id);

        // Generate a deterministic UUID from the vector_id for Qdrant's point ID
        let qdrant_uuid = Uuid::new_v5(
            &Uuid::NAMESPACE_OID,
            format!("trade-{}-{}", user_id, vector_id).as_bytes(),
        )
        .to_string();

        log::info!(
            "Upserting trade vector to Qdrant - collection={}, qdrant_uuid={}, vector_id={}, content_length={}, embedding_dim={}",
            collection_name,
            qdrant_uuid,
            vector_id,
            content.len(),
            embedding.len()
        );

        let now = Utc::now();

        // Create payload with the specified format
        let mut payload = HashMap::new();
        payload.insert("user_id".to_string(), Value::from(user_id));
        payload.insert("id".to_string(), Value::from(vector_id));
        payload.insert("content".to_string(), Value::from(content));
        payload.insert("type".to_string(), Value::from("trade"));
        payload.insert("created_at".to_string(), Value::from(now.to_rfc3339()));

        log::debug!(
            "Created payload for trade vector - vector_id={}, payload_keys={:?}",
            vector_id,
            payload.keys().collect::<Vec<_>>()
        );

        // Create point with embedding using the generated UUID
        let point_id = PointId {
            point_id_options: Some(qdrant_client::qdrant::point_id::PointIdOptions::Uuid(
                qdrant_uuid.clone(),
            )),
        };

        log::debug!(
            "Creating PointStruct - vector_id={}, qdrant_uuid={}, embedding_dim={}, payload_keys={:?}",
            vector_id,
            qdrant_uuid,
            embedding.len(),
            payload.keys().collect::<Vec<_>>()
        );

        let point = PointStruct {
            id: Some(point_id),
            vectors: Some(embedding.to_vec().into()),
            payload: payload.clone(),
        };

        log::debug!(
            "Created PointStruct - vector_id={}, has_id={}, has_vectors={}, payload_count={}",
            vector_id,
            point.id.is_some(),
            point.vectors.is_some(),
            point.payload.len()
        );

        let upsert_request = UpsertPoints {
            collection_name: collection_name.clone(),
            points: vec![point],
            ..Default::default()
        };

        log::debug!(
            "Calling Qdrant upsert_points - collection={}, points_count=1",
            collection_name
        );

        // Retry logic for upsert_points with exponential backoff
        const MAX_RETRIES: u32 = 3;
        let mut last_error = None;

        for attempt in 0..MAX_RETRIES {
            match self.client.upsert_points(upsert_request.clone()).await {
                Ok(response) => {
                    if attempt > 0 {
                        log::info!(
                            "Successfully upserted trade vector after {} retries",
                            attempt
                        );
                    }
                    log::info!(
                        "Successfully upserted trade vector to Qdrant - qdrant_uuid={}, vector_id={}, collection={}",
                        qdrant_uuid,
                        vector_id,
                        collection_name
                    );
                    log::debug!(
                        "Qdrant upsert response - vector_id={}, result={:?}",
                        vector_id,
                        response.result
                    );
                    return Ok(());
                }
                Err(e) => {
                    let error_string = format!("{}", e);
                    last_error = Some(e);

                    if is_timeout_error(&last_error.as_ref().unwrap()) && attempt < MAX_RETRIES - 1
                    {
                        let delay = Duration::from_millis(1000 * 2_u64.pow(attempt));
                        log::warn!(
                            "Failed to upsert trade vector (attempt {}/{}): {}. Retrying in {:?}...",
                            attempt + 1,
                            MAX_RETRIES,
                            error_string,
                            delay
                        );
                        tokio::time::sleep(delay).await;
                        continue;
                    } else {
                        let error_debug = format!("{:?}", last_error.as_ref().unwrap());
                        log::error!(
                            "Failed to upsert trade vector to Qdrant - user={}, vector_id={}, qdrant_uuid={}, collection={}",
                            user_id,
                            vector_id,
                            qdrant_uuid,
                            collection_name
                        );
                        log::error!("Qdrant error message: {}", error_string);
                        log::error!("Qdrant error debug: {}", error_debug);
                        log::error!(
                            "Qdrant upsert error details - embedding_dim={}, content_length={}, content_preview={}, payload_count={}",
                            embedding.len(),
                            content.len(),
                            if content.len() > 100 {
                                format!("{}...", &content[..100])
                            } else {
                                content.to_string()
                            },
                            payload.len()
                        );

                        return Err(anyhow::anyhow!(
                            "Failed to upsert vector to Qdrant: {} (vector_id: {}, collection: {})",
                            error_string,
                            vector_id,
                            collection_name
                        ));
                    }
                }
            }
        }

        // This should never be reached, but handle it just in case
        let error_msg = last_error
            .map(|e| format!("{}", e))
            .unwrap_or_else(|| "unknown error".to_string());
        Err(anyhow::anyhow!(
            "Failed to upsert vector to Qdrant after {} retries: {} (vector_id: {}, collection: {})",
            MAX_RETRIES,
            error_msg,
            vector_id,
            collection_name
        ))
    }

    /// Upsert a trade vector with structured payload containing symbol, analytics, and content
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_trade_vector_structured(
        &self,
        user_id: &str,
        vector_id: &str,
        symbol: &str,
        trade_type: &str,
        trade_id: i64,
        analytics: crate::service::ai_service::vector_service::vectors::trade::TradeAnalytics,
        mistakes: Option<&str>,
        trade_notes: Option<&str>,
        embedding: &[f32],
        created_at: Option<DateTime<Utc>>,
    ) -> Result<()> {
        log::debug!(
            "Starting upsert_trade_vector_structured - user={}, vector_id={}, symbol={}, trade_type={}, trade_id={}, embedding_dim={}",
            user_id,
            vector_id,
            symbol,
            trade_type,
            trade_id,
            embedding.len()
        );

        self.ensure_collection(user_id)
            .await
            .context("Failed to ensure collection exists")?;
        let collection_name = self.config.get_collection_name(user_id);

        // Generate a deterministic UUID from the vector_id for Qdrant's point ID
        let qdrant_uuid = Uuid::new_v5(
            &Uuid::NAMESPACE_OID,
            format!("trade-{}-{}", user_id, vector_id).as_bytes(),
        )
        .to_string();

        log::info!(
            "Upserting structured trade vector to Qdrant - collection={}, qdrant_uuid={}, vector_id={}, symbol={}, trade_id={}",
            collection_name,
            qdrant_uuid,
            vector_id,
            symbol,
            trade_id
        );

        // Use provided created_at timestamp or fallback to current time
        let created_at_timestamp = created_at.unwrap_or_else(Utc::now);

        // Create structured payload
        let mut payload = HashMap::new();
        payload.insert("user_id".to_string(), Value::from(user_id));
        payload.insert("id".to_string(), Value::from(vector_id));
        payload.insert("type".to_string(), Value::from("trade"));
        payload.insert(
            "created_at".to_string(),
            Value::from(created_at_timestamp.to_rfc3339()),
        );
        // Add timestamp as f64 for efficient range queries
        payload.insert(
            "created_at_timestamp".to_string(),
            Value::from(created_at_timestamp.timestamp() as f64),
        );
        payload.insert("symbol".to_string(), Value::from(symbol));
        payload.insert("trade_type".to_string(), Value::from(trade_type));
        payload.insert("trade_id".to_string(), Value::from(trade_id));

        // Add analytics as nested fields
        payload.insert("net_pnl".to_string(), Value::from(analytics.net_pnl));
        payload.insert("profit_loss".to_string(), Value::from(analytics.net_pnl)); // For filtering by profit/loss
        payload.insert(
            "net_pnl_percent".to_string(),
            Value::from(analytics.net_pnl_percent),
        );
        payload.insert(
            "position_size".to_string(),
            Value::from(analytics.position_size),
        );
        payload.insert(
            "stop_loss_risk_percent".to_string(),
            Value::from(analytics.stop_loss_risk_percent),
        );
        payload.insert(
            "risk_to_reward".to_string(),
            Value::from(analytics.risk_to_reward),
        );
        payload.insert(
            "entry_price".to_string(),
            Value::from(analytics.entry_price),
        );
        payload.insert("exit_price".to_string(), Value::from(analytics.exit_price));
        payload.insert(
            "hold_time_days".to_string(),
            Value::from(analytics.hold_time_days),
        );

        // Add content fields
        if let Some(mistakes_text) = mistakes && !mistakes_text.trim().is_empty() {
            payload.insert("mistakes".to_string(), Value::from(mistakes_text));
        }

        if let Some(notes_text) = trade_notes && !notes_text.trim().is_empty() {
            payload.insert("trade_notes".to_string(), Value::from(notes_text));
        }

        log::debug!(
            "Created structured payload for trade vector - vector_id={}, payload_keys={:?}",
            vector_id,
            payload.keys().collect::<Vec<_>>()
        );

        // Create point with embedding using the generated UUID
        let point_id = PointId {
            point_id_options: Some(qdrant_client::qdrant::point_id::PointIdOptions::Uuid(
                qdrant_uuid.clone(),
            )),
        };

        let point = PointStruct {
            id: Some(point_id),
            vectors: Some(embedding.to_vec().into()),
            payload: payload.clone(),
        };

        let upsert_request = UpsertPoints {
            collection_name: collection_name.clone(),
            points: vec![point],
            ..Default::default()
        };

        log::debug!(
            "Calling Qdrant upsert_points - collection={}, points_count=1",
            collection_name
        );

        match self.client.upsert_points(upsert_request).await {
            Ok(response) => {
                log::info!(
                    "Successfully upserted structured trade vector to Qdrant - qdrant_uuid={}, vector_id={}, symbol={}, collection={}",
                    qdrant_uuid,
                    vector_id,
                    symbol,
                    collection_name
                );
                log::debug!(
                    "Qdrant upsert response - vector_id={}, result={:?}",
                    vector_id,
                    response.result
                );
                Ok(())
            }
            Err(e) => {
                let error_string = format!("{}", e);
                let error_debug = format!("{:?}", e);

                log::error!(
                    "Failed to upsert structured trade vector to Qdrant - user={}, vector_id={}, qdrant_uuid={}, collection={}",
                    user_id,
                    vector_id,
                    qdrant_uuid,
                    collection_name
                );
                log::error!("Qdrant error message: {}", error_string);
                log::error!("Qdrant error debug: {}", error_debug);
                log::error!(
                    "Qdrant upsert error details - embedding_dim={}, payload_count={}",
                    embedding.len(),
                    payload.len()
                );

                Err(anyhow::anyhow!(
                    "Failed to upsert structured trade vector to Qdrant: {} (vector_id: {}, collection: {})",
                    error_string,
                    vector_id,
                    collection_name
                ))
            }
        }
    }

    #[allow(dead_code)]
    pub async fn upsert_documents(&self, user_id: &str, documents: Vec<Document>) -> Result<()> {
        if documents.is_empty() {
            return Ok(());
        }

        self.ensure_collection(user_id).await?;
        let collection_name = self.config.get_collection_name(user_id);

        log::info!(
            "Upserting {} documents to Qdrant collection: {}",
            documents.len(),
            collection_name
        );

        let points: Vec<PointStruct> = documents
            .into_iter()
            .map(|doc| {
                let mut payload = HashMap::new();

                // Add metadata
                payload.insert("user_id".to_string(), Value::from(doc.metadata.user_id));
                payload.insert("data_type".to_string(), Value::from(doc.metadata.data_type));
                payload.insert("entity_id".to_string(), Value::from(doc.metadata.entity_id));
                payload.insert(
                    "timestamp".to_string(),
                    Value::from(doc.metadata.timestamp.to_rfc3339()),
                );
                payload.insert(
                    "content_hash".to_string(),
                    Value::from(doc.metadata.content_hash),
                );
                payload.insert("original_id".to_string(), Value::from(doc.id.clone()));

                // Add content fields
                for (key, value) in doc.content {
                    payload.insert(key, Value::from(value));
                }

                // Add tags
                let tags: Vec<Value> = doc.metadata.tags.into_iter().map(Value::from).collect();
                payload.insert("tags".to_string(), Value::from(tags));

                // Generate a proper UUID for the document ID
                let document_uuid = Uuid::new_v4().to_string();

                PointStruct {
                    id: Some(PointId {
                        point_id_options: Some(
                            qdrant_client::qdrant::point_id::PointIdOptions::Uuid(document_uuid),
                        ),
                    }),
                    vectors: Some(vec![0.0].into()), // Dummy vector
                    payload,
                }
            })
            .collect();

        self.client
            .upsert_points(UpsertPoints {
                collection_name: collection_name.clone(),
                points,
                ..Default::default()
            })
            .await?;

        log::info!("Successfully upserted documents to Qdrant");
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn search_by_keyword(
        &self,
        user_id: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<String>> {
        let collection_name = self.config.get_collection_name(user_id);

        // Build filter for keyword search in content field
        let filter = Filter {
            must: vec![Condition {
                condition_one_of: Some(qdrant_client::qdrant::condition::ConditionOneOf::Field(
                    FieldCondition {
                        key: "content".to_string(),
                        r#match: Some(Match {
                            match_value: Some(qdrant_client::qdrant::r#match::MatchValue::Text(
                                query.to_string(),
                            )),
                        }),
                        ..Default::default()
                    },
                )),
            }],
            ..Default::default()
        };

        let scroll_request = ScrollPoints {
            collection_name: collection_name.clone(),
            filter: Some(filter),
            limit: Some(limit as u32),
            ..Default::default()
        };

        let search_result = self.client.scroll(scroll_request).await?;

        let ids: Vec<String> = search_result
            .result
            .into_iter()
            .filter_map(|point| {
                point.id.map(|id| match id {
                    PointId {
                        point_id_options: Some(point_id_options),
                    } => match point_id_options {
                        qdrant_client::qdrant::point_id::PointIdOptions::Num(n) => n.to_string(),
                        qdrant_client::qdrant::point_id::PointIdOptions::Uuid(u) => u,
                    },
                    _ => "unknown".to_string(),
                })
            })
            .collect();

        Ok(ids)
    }

    /// Delete a trade vector by ID
    pub async fn delete_trade_vector(&self, user_id: &str, vector_id: &str) -> Result<()> {
        let collection_name = self.config.get_collection_name(user_id);

        // Build filter to find the vector by ID
        let filter = Filter {
            must: vec![Condition {
                condition_one_of: Some(qdrant_client::qdrant::condition::ConditionOneOf::Field(
                    FieldCondition {
                        key: "id".to_string(),
                        r#match: Some(Match {
                            match_value: Some(qdrant_client::qdrant::r#match::MatchValue::Text(
                                vector_id.to_string(),
                            )),
                        }),
                        ..Default::default()
                    },
                )),
            }],
            ..Default::default()
        };

        // Find the point to get its Qdrant UUID
        let scroll_request = ScrollPoints {
            collection_name: collection_name.clone(),
            filter: Some(filter),
            limit: Some(1),
            ..Default::default()
        };

        let search_result = self.client.scroll(scroll_request).await?;

        if let Some(point) = search_result.result.into_iter().next() {
            if let Some(qdrant_id) = point.id {
                let points_selector = PointsSelector {
                    points_selector_one_of: Some(
                        qdrant_client::qdrant::points_selector::PointsSelectorOneOf::Points(
                            PointsIdsList {
                                ids: vec![qdrant_id],
                            },
                        ),
                    ),
                };

                self.client
                    .delete_points(qdrant_client::qdrant::DeletePoints {
                        collection_name: collection_name.clone(),
                        points: Some(points_selector),
                        ..Default::default()
                    })
                    .await?;

                log::info!("Deleted trade vector from Qdrant - vector_id={}", vector_id);
            }
        } else {
            log::warn!("Trade vector not found in Qdrant - vector_id={}", vector_id);
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn delete_documents(&self, user_id: &str, document_ids: &[String]) -> Result<()> {
        if document_ids.is_empty() {
            return Ok(());
        }

        let collection_name = self.config.get_collection_name(user_id);

        // Build filter to find documents by their original_id
        let mut conditions = Vec::new();
        for doc_id in document_ids {
            conditions.push(Condition {
                condition_one_of: Some(qdrant_client::qdrant::condition::ConditionOneOf::Field(
                    FieldCondition {
                        key: "original_id".to_string(),
                        r#match: Some(Match {
                            match_value: Some(qdrant_client::qdrant::r#match::MatchValue::Text(
                                doc_id.clone(),
                            )),
                        }),
                        ..Default::default()
                    },
                )),
            });
        }

        let filter = Filter {
            should: conditions,
            ..Default::default()
        };

        // First, find the documents to get their Qdrant UUIDs
        let scroll_request = ScrollPoints {
            collection_name: collection_name.clone(),
            filter: Some(filter),
            limit: Some(document_ids.len() as u32),
            ..Default::default()
        };

        let search_result = self.client.scroll(scroll_request).await?;

        // Extract the Qdrant UUIDs
        let qdrant_ids: Vec<PointId> = search_result
            .result
            .into_iter()
            .filter_map(|point| point.id)
            .collect();

        if qdrant_ids.is_empty() {
            log::warn!("No documents found to delete for IDs: {:?}", document_ids);
            return Ok(());
        }

        let points_selector = PointsSelector {
            points_selector_one_of: Some(
                qdrant_client::qdrant::points_selector::PointsSelectorOneOf::Points(
                    PointsIdsList { ids: qdrant_ids },
                ),
            ),
        };

        self.client
            .delete_points(qdrant_client::qdrant::DeletePoints {
                collection_name: collection_name.clone(),
                points: Some(points_selector),
                ..Default::default()
            })
            .await?;

        log::info!("Deleted {} documents from Qdrant", document_ids.len());
        Ok(())
    }

    /// Upsert a chat vector with format: {user_id, id, content, embedding, type: "chat", created_at}
    pub async fn upsert_chat_vector(
        &self,
        user_id: &str,
        qdrant_uuid: &str,   // The UUID for Qdrant's point ID
        meaningful_id: &str, // Your "chat-{session_id}-qa" ID for payload
        content: &str,
        embedding: &[f32],
    ) -> Result<()> {
        self.ensure_collection(user_id).await?;
        let collection_name = self.config.get_collection_name(user_id);

        log::info!(
            "Upserting chat vector to Qdrant - collection={}, qdrant_uuid={}, meaningful_id={}, content_length={}, embedding_dim={}",
            collection_name,
            qdrant_uuid,
            meaningful_id,
            content.len(),
            embedding.len()
        );

        let now = Utc::now();

        // Create payload with the specified format
        let mut payload = HashMap::new();
        payload.insert("user_id".to_string(), Value::from(user_id));
        payload.insert("id".to_string(), Value::from(meaningful_id)); // Use meaningful_id in payload
        payload.insert("content".to_string(), Value::from(content));
        payload.insert("type".to_string(), Value::from("chat"));
        payload.insert("created_at".to_string(), Value::from(now.to_rfc3339()));

        // Create point with embedding
        let point = PointStruct {
            id: Some(PointId {
                point_id_options: Some(qdrant_client::qdrant::point_id::PointIdOptions::Uuid(
                    qdrant_uuid.to_string(), // Use the generated UUID for Qdrant's point ID
                )),
            }),
            vectors: Some(embedding.to_vec().into()),
            payload,
        };

        match self
            .client
            .upsert_points(UpsertPoints {
                collection_name: collection_name.clone(),
                points: vec![point],
                ..Default::default()
            })
            .await
        {
            Ok(_) => {
                log::info!(
                    "Successfully upserted chat vector to Qdrant - qdrant_uuid={}, meaningful_id={}",
                    qdrant_uuid,
                    meaningful_id
                );
                Ok(())
            }
            Err(e) => {
                log::error!(
                    "Failed to upsert chat vector to Qdrant - collection={}, qdrant_uuid={}, meaningful_id={}, error={}, error_debug={:?}",
                    collection_name,
                    qdrant_uuid,
                    meaningful_id,
                    e,
                    e
                );
                Err(anyhow::anyhow!("Failed to upsert chat vector: {}", e))
            }
        }
    }

    /// Upsert a playbook vector with format: {user_id, id, content, embedding, type: "playbook", created_at}
    pub async fn upsert_playbook_vector(
        &self,
        user_id: &str,
        vector_id: &str,
        content: &str,
        embedding: &[f32],
    ) -> Result<()> {
        self.ensure_collection(user_id).await?;
        let collection_name = self.config.get_collection_name(user_id);

        // Generate a deterministic UUID from the vector_id for Qdrant's point ID
        let qdrant_uuid = Uuid::new_v5(
            &Uuid::NAMESPACE_OID,
            format!("playbook-{}-{}", user_id, vector_id).as_bytes(),
        )
        .to_string();

        log::info!(
            "Upserting playbook vector to Qdrant - collection={}, qdrant_uuid={}, vector_id={}, content_length={}, embedding_dim={}",
            collection_name,
            qdrant_uuid,
            vector_id,
            content.len(),
            embedding.len()
        );

        let now = Utc::now();

        // Create payload with the specified format
        let mut payload = HashMap::new();
        payload.insert("user_id".to_string(), Value::from(user_id));
        payload.insert("id".to_string(), Value::from(vector_id));
        payload.insert("content".to_string(), Value::from(content));
        payload.insert("type".to_string(), Value::from("playbook"));
        payload.insert("created_at".to_string(), Value::from(now.to_rfc3339()));

        // Create point with embedding using the generated UUID
        let point = PointStruct {
            id: Some(PointId {
                point_id_options: Some(qdrant_client::qdrant::point_id::PointIdOptions::Uuid(
                    qdrant_uuid.clone(),
                )),
            }),
            vectors: Some(embedding.to_vec().into()),
            payload,
        };

        match self
            .client
            .upsert_points(UpsertPoints {
                collection_name: collection_name.clone(),
                points: vec![point],
                ..Default::default()
            })
            .await
        {
            Ok(_) => {
                log::info!(
                    "Successfully upserted playbook vector to Qdrant - qdrant_uuid={}, vector_id={}",
                    qdrant_uuid,
                    vector_id
                );
                Ok(())
            }
            Err(e) => {
                log::error!(
                    "Failed to upsert playbook vector to Qdrant - collection={}, qdrant_uuid={}, vector_id={}, error={}, error_debug={:?}",
                    collection_name,
                    qdrant_uuid,
                    vector_id,
                    e,
                    e
                );
                Err(anyhow::anyhow!("Failed to upsert playbook vector: {}", e))
            }
        }
    }

    /// Upsert a notebook vector with format: {user_id, id, content, embedding, type: "notebook", created_at}
    pub async fn upsert_notebook_vector(
        &self,
        user_id: &str,
        vector_id: &str,
        content: &str,
        embedding: &[f32],
    ) -> Result<()> {
        self.ensure_collection(user_id).await?;
        let collection_name = self.config.get_collection_name(user_id);

        // Generate a deterministic UUID from the vector_id for Qdrant's point ID
        let qdrant_uuid = Uuid::new_v5(
            &Uuid::NAMESPACE_OID,
            format!("notebook-{}-{}", user_id, vector_id).as_bytes(),
        )
        .to_string();

        log::info!(
            "Upserting notebook vector to Qdrant - collection={}, qdrant_uuid={}, vector_id={}, content_length={}, embedding_dim={}",
            collection_name,
            qdrant_uuid,
            vector_id,
            content.len(),
            embedding.len()
        );

        let now = Utc::now();

        // Create payload with the specified format
        let mut payload = HashMap::new();
        payload.insert("user_id".to_string(), Value::from(user_id));
        payload.insert("id".to_string(), Value::from(vector_id));
        payload.insert("content".to_string(), Value::from(content));
        payload.insert("type".to_string(), Value::from("notebook"));
        payload.insert("created_at".to_string(), Value::from(now.to_rfc3339()));

        // Create point with embedding using the generated UUID
        let point = PointStruct {
            id: Some(PointId {
                point_id_options: Some(qdrant_client::qdrant::point_id::PointIdOptions::Uuid(
                    qdrant_uuid.clone(),
                )),
            }),
            vectors: Some(embedding.to_vec().into()),
            payload,
        };

        match self
            .client
            .upsert_points(UpsertPoints {
                collection_name: collection_name.clone(),
                points: vec![point],
                ..Default::default()
            })
            .await
        {
            Ok(_) => {
                log::info!(
                    "Successfully upserted notebook vector to Qdrant - qdrant_uuid={}, vector_id={}",
                    qdrant_uuid,
                    vector_id
                );
                Ok(())
            }
            Err(e) => {
                log::error!(
                    "Failed to upsert notebook vector to Qdrant - collection={}, qdrant_uuid={}, vector_id={}, error={}, error_debug={:?}",
                    collection_name,
                    qdrant_uuid,
                    vector_id,
                    e,
                    e
                );
                Err(anyhow::anyhow!("Failed to upsert notebook vector: {}", e))
            }
        }
    }

    /// Delete a notebook vector by ID
    pub async fn delete_notebook_vector(&self, user_id: &str, vector_id: &str) -> Result<()> {
        let collection_name = self.config.get_collection_name(user_id);

        // Build filter to find the vector by ID
        let filter = Filter {
            must: vec![Condition {
                condition_one_of: Some(qdrant_client::qdrant::condition::ConditionOneOf::Field(
                    FieldCondition {
                        key: "id".to_string(),
                        r#match: Some(Match {
                            match_value: Some(qdrant_client::qdrant::r#match::MatchValue::Text(
                                vector_id.to_string(),
                            )),
                        }),
                        ..Default::default()
                    },
                )),
            }],
            ..Default::default()
        };

        // Find the point to get its Qdrant UUID
        let scroll_request = ScrollPoints {
            collection_name: collection_name.clone(),
            filter: Some(filter),
            limit: Some(1),
            ..Default::default()
        };

        let search_result = self.client.scroll(scroll_request).await?;

        if let Some(point) = search_result.result.into_iter().next() {
            if let Some(qdrant_id) = point.id {
                let points_selector = PointsSelector {
                    points_selector_one_of: Some(
                        qdrant_client::qdrant::points_selector::PointsSelectorOneOf::Points(
                            PointsIdsList {
                                ids: vec![qdrant_id],
                            },
                        ),
                    ),
                };

                self.client
                    .delete_points(qdrant_client::qdrant::DeletePoints {
                        collection_name: collection_name.clone(),
                        points: Some(points_selector),
                        ..Default::default()
                    })
                    .await?;

                log::info!(
                    "Deleted notebook vector from Qdrant - vector_id={}",
                    vector_id
                );
            }
        } else {
            log::warn!(
                "Notebook vector not found in Qdrant - vector_id={}",
                vector_id
            );
        }

        Ok(())
    }

    /// Search by embedding using semantic similarity
    /// Returns top N results sorted by cosine similarity score
    pub async fn search_by_embedding(
        &self,
        user_id: &str,
        query_embedding: &[f32],
        limit: usize,
        type_filter: Option<&str>,
    ) -> Result<Vec<SearchResult>> {
        let collection_name = self.config.get_collection_name(user_id);

        log::info!(
            "Searching Qdrant by embedding - collection={}, limit={}, type_filter={:?}",
            collection_name,
            limit,
            type_filter
        );

        // Build filter for user_id (required) and optionally by type
        let mut must_conditions = vec![Condition {
            condition_one_of: Some(qdrant_client::qdrant::condition::ConditionOneOf::Field(
                FieldCondition {
                    key: "user_id".to_string(),
                    r#match: Some(Match {
                        match_value: Some(qdrant_client::qdrant::r#match::MatchValue::Text(
                            user_id.to_string(),
                        )),
                    }),
                    ..Default::default()
                },
            )),
        }];

        // Add type filter if specified
        if let Some(type_val) = type_filter {
            must_conditions.push(Condition {
                condition_one_of: Some(qdrant_client::qdrant::condition::ConditionOneOf::Field(
                    FieldCondition {
                        key: "type".to_string(),
                        r#match: Some(Match {
                            match_value: Some(qdrant_client::qdrant::r#match::MatchValue::Text(
                                type_val.to_string(),
                            )),
                        }),
                        ..Default::default()
                    },
                )),
            });
        }

        let filter = Filter {
            must: must_conditions,
            ..Default::default()
        };

        // Perform semantic search
        let search_request = SearchPoints {
            collection_name: collection_name.clone(),
            vector: query_embedding.to_vec(),
            limit: limit as u64,
            filter: Some(filter),
            with_payload: Some(true.into()),
            ..Default::default()
        };

        let search_result = self.client.search_points(search_request).await?;

        // Convert results to SearchResult
        let results: Vec<SearchResult> =
            search_result
                .result
                .into_iter()
                .map(|scored_point| {
                    let id = scored_point
                        .payload
                        .get("id")
                        .and_then(|v| match &v.kind {
                            Some(Kind::StringValue(s)) => Some(s.clone()),
                            _ => None,
                        })
                        .unwrap_or_else(|| {
                            // Fallback to Qdrant point ID if payload id not found
                            match &scored_point.id {
                                Some(PointId {
                                    point_id_options: Some(point_id_options),
                                }) => match point_id_options {
                                    qdrant_client::qdrant::point_id::PointIdOptions::Uuid(u) => {
                                        u.clone()
                                    }
                                    qdrant_client::qdrant::point_id::PointIdOptions::Num(n) => {
                                        n.to_string()
                                    }
                                },
                                _ => "unknown".to_string(),
                            }
                        });

                    let content = scored_point
                        .payload
                        .get("content")
                        .and_then(|v| match &v.kind {
                            Some(Kind::StringValue(s)) => Some(s.clone()),
                            _ => None,
                        })
                        .unwrap_or_default();

                    let r#type = scored_point
                        .payload
                        .get("type")
                        .and_then(|v| match &v.kind {
                            Some(Kind::StringValue(s)) => Some(s.clone()),
                            _ => None,
                        });

                    let created_at = scored_point.payload.get("created_at").and_then(|v| match &v
                        .kind
                    {
                        Some(Kind::StringValue(s)) => DateTime::parse_from_rfc3339(s)
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc)),
                        _ => None,
                    });

                    SearchResult {
                        id,
                        score: scored_point.score,
                        content,
                        r#type,
                        created_at,
                    }
                })
                .collect();

        log::info!(
            "Semantic search completed - collection={}, results={}",
            collection_name,
            results.len()
        );

        if results.is_empty() {
            log::warn!(
                "Semantic search returned no results - collection={}, user_id={}, limit={}, type_filter={:?}",
                collection_name,
                user_id,
                limit,
                type_filter
            );
        }

        Ok(results)
    }

    /// Search by embedding with metadata filters (date range, profit/loss, symbol, trade_type)
    pub async fn search_by_embedding_with_filters(
        &self,
        user_id: &str,
        query_embedding: &[f32],
        limit: usize,
        filters: TradeSearchFilters,
    ) -> Result<Vec<SearchResult>> {
        let collection_name = self.config.get_collection_name(user_id);

        log::info!(
            "Searching Qdrant by embedding with filters - collection={}, limit={}, filters={:?}",
            collection_name,
            limit,
            filters
        );

        // Build filter for user_id (required) and type="trade"
        let mut must_conditions = vec![
            Condition {
                condition_one_of: Some(qdrant_client::qdrant::condition::ConditionOneOf::Field(
                    FieldCondition {
                        key: "user_id".to_string(),
                        r#match: Some(Match {
                            match_value: Some(qdrant_client::qdrant::r#match::MatchValue::Text(
                                user_id.to_string(),
                            )),
                        }),
                        ..Default::default()
                    },
                )),
            },
            Condition {
                condition_one_of: Some(qdrant_client::qdrant::condition::ConditionOneOf::Field(
                    FieldCondition {
                        key: "type".to_string(),
                        r#match: Some(Match {
                            match_value: Some(qdrant_client::qdrant::r#match::MatchValue::Text(
                                "trade".to_string(),
                            )),
                        }),
                        ..Default::default()
                    },
                )),
            },
        ];

        // Add date range filter using timestamp field
        // We'll add created_at_timestamp to payload for efficient numeric range queries
        if let Some(date_range) = &filters.date_range {
            let start_ts = date_range.start.timestamp() as f64;
            let end_ts = date_range.end.timestamp() as f64;

            must_conditions.push(Condition {
                condition_one_of: Some(qdrant_client::qdrant::condition::ConditionOneOf::Field(
                    FieldCondition {
                        key: "created_at_timestamp".to_string(),
                        range: Some(Range {
                            lt: None,
                            gt: None,
                            gte: Some(start_ts),
                            lte: Some(end_ts),
                        }),
                        ..Default::default()
                    },
                )),
            });
        }

        // Add profit/loss filters
        if let Some(positive_only) = filters.profit_loss_positive_only {
            if positive_only {
                // Only profits (positive values)
                must_conditions.push(Condition {
                    condition_one_of: Some(
                        qdrant_client::qdrant::condition::ConditionOneOf::Field(FieldCondition {
                            key: "profit_loss".to_string(),
                            range: Some(Range {
                                lt: None,
                                gt: Some(0.0),
                                gte: None,
                                lte: None,
                            }),
                            ..Default::default()
                        }),
                    ),
                });
            } else {
                // Only losses (negative values)
                must_conditions.push(Condition {
                    condition_one_of: Some(
                        qdrant_client::qdrant::condition::ConditionOneOf::Field(FieldCondition {
                            key: "profit_loss".to_string(),
                            range: Some(Range {
                                lt: Some(0.0),
                                gt: None,
                                gte: None,
                                lte: None,
                            }),
                            ..Default::default()
                        }),
                    ),
                });
            }
        } else {
            // Use min/max if specified
            if filters.profit_loss_min.is_some() || filters.profit_loss_max.is_some() {
                must_conditions.push(Condition {
                    condition_one_of: Some(
                        qdrant_client::qdrant::condition::ConditionOneOf::Field(FieldCondition {
                            key: "profit_loss".to_string(),
                            range: Some(Range {
                                lt: filters.profit_loss_max,
                                gt: filters.profit_loss_min,
                                gte: None,
                                lte: None,
                            }),
                            ..Default::default()
                        }),
                    ),
                });
            }
        }

        // Add symbol filter
        if let Some(symbol) = &filters.symbol {
            must_conditions.push(Condition {
                condition_one_of: Some(qdrant_client::qdrant::condition::ConditionOneOf::Field(
                    FieldCondition {
                        key: "symbol".to_string(),
                        r#match: Some(Match {
                            match_value: Some(qdrant_client::qdrant::r#match::MatchValue::Text(
                                symbol.clone(),
                            )),
                        }),
                        ..Default::default()
                    },
                )),
            });
        }

        // Add trade_type filter
        if let Some(trade_type) = &filters.trade_type {
            must_conditions.push(Condition {
                condition_one_of: Some(qdrant_client::qdrant::condition::ConditionOneOf::Field(
                    FieldCondition {
                        key: "trade_type".to_string(),
                        r#match: Some(Match {
                            match_value: Some(qdrant_client::qdrant::r#match::MatchValue::Text(
                                trade_type.clone(),
                            )),
                        }),
                        ..Default::default()
                    },
                )),
            });
        }

        let filter = Filter {
            must: must_conditions,
            ..Default::default()
        };

        // Perform semantic search
        let search_request = SearchPoints {
            collection_name: collection_name.clone(),
            vector: query_embedding.to_vec(),
            limit: limit as u64,
            filter: Some(filter),
            with_payload: Some(true.into()),
            ..Default::default()
        };

        let search_result = self.client.search_points(search_request).await?;

        // Convert results to SearchResult (same as search_by_embedding)
        let results: Vec<SearchResult> = search_result
            .result
            .into_iter()
            .map(|scored_point| {
                let id = scored_point
                    .payload
                    .get("id")
                    .and_then(|v| match &v.kind {
                        Some(Kind::StringValue(s)) => Some(s.clone()),
                        _ => None,
                    })
                    .unwrap_or_else(|| match &scored_point.id {
                        Some(PointId {
                            point_id_options: Some(point_id_options),
                        }) => match point_id_options {
                            qdrant_client::qdrant::point_id::PointIdOptions::Uuid(u) => u.clone(),
                            qdrant_client::qdrant::point_id::PointIdOptions::Num(n) => {
                                n.to_string()
                            }
                        },
                        _ => "unknown".to_string(),
                    });

                let content = scored_point
                    .payload
                    .get("content")
                    .and_then(|v| match &v.kind {
                        Some(Kind::StringValue(s)) => Some(s.clone()),
                        _ => None,
                    })
                    .unwrap_or_default();

                let r#type = scored_point
                    .payload
                    .get("type")
                    .and_then(|v| match &v.kind {
                        Some(Kind::StringValue(s)) => Some(s.clone()),
                        _ => None,
                    });

                let created_at =
                    scored_point
                        .payload
                        .get("created_at")
                        .and_then(|v| match &v.kind {
                            Some(Kind::StringValue(s)) => DateTime::parse_from_rfc3339(s)
                                .ok()
                                .map(|dt| dt.with_timezone(&Utc)),
                            _ => None,
                        });

                SearchResult {
                    id,
                    score: scored_point.score,
                    content,
                    r#type,
                    created_at,
                }
            })
            .collect();

        log::info!(
            "Semantic search with filters completed - collection={}, results={}",
            collection_name,
            results.len()
        );

        Ok(results)
    }

    /// Get all trades in a period (no semantic search, complete dataset)
    pub async fn get_all_trades_in_period(
        &self,
        user_id: &str,
        date_range: DateRange,
        filters: Option<TradeMetadataFilters>,
        limit: Option<usize>,
    ) -> Result<Vec<SearchResult>> {
        let collection_name = self.config.get_collection_name(user_id);
        let limit = limit.unwrap_or(1000);

        log::info!(
            "Getting all trades in period - collection={}, date_range={:?} to {:?}, limit={}",
            collection_name,
            date_range.start,
            date_range.end,
            limit
        );

        // Build filter for user_id (required) and type="trade"
        let mut must_conditions = vec![
            Condition {
                condition_one_of: Some(qdrant_client::qdrant::condition::ConditionOneOf::Field(
                    FieldCondition {
                        key: "user_id".to_string(),
                        r#match: Some(Match {
                            match_value: Some(qdrant_client::qdrant::r#match::MatchValue::Text(
                                user_id.to_string(),
                            )),
                        }),
                        ..Default::default()
                    },
                )),
            },
            Condition {
                condition_one_of: Some(qdrant_client::qdrant::condition::ConditionOneOf::Field(
                    FieldCondition {
                        key: "type".to_string(),
                        r#match: Some(Match {
                            match_value: Some(qdrant_client::qdrant::r#match::MatchValue::Text(
                                "trade".to_string(),
                            )),
                        }),
                        ..Default::default()
                    },
                )),
            },
        ];

        // Add date range filter using timestamp field
        let start_ts = date_range.start.timestamp() as f64;
        let end_ts = date_range.end.timestamp() as f64;

        must_conditions.push(Condition {
            condition_one_of: Some(qdrant_client::qdrant::condition::ConditionOneOf::Field(
                FieldCondition {
                    key: "created_at_timestamp".to_string(),
                    range: Some(Range {
                        lt: None,
                        gt: None,
                        gte: Some(start_ts),
                        lte: Some(end_ts),
                    }),
                    ..Default::default()
                },
            )),
        });

        // Add metadata filters if provided
        if let Some(meta_filters) = filters {
            // Profit/loss filters
            if let Some(positive_only) = meta_filters.profit_loss_positive_only {
                if positive_only {
                    must_conditions.push(Condition {
                        condition_one_of: Some(
                            qdrant_client::qdrant::condition::ConditionOneOf::Field(
                                FieldCondition {
                                    key: "profit_loss".to_string(),
                                    range: Some(Range {
                                        lt: None,
                                        gt: Some(0.0),
                                        gte: None,
                                        lte: None,
                                    }),
                                    ..Default::default()
                                },
                            ),
                        ),
                    });
                } else {
                    must_conditions.push(Condition {
                        condition_one_of: Some(
                            qdrant_client::qdrant::condition::ConditionOneOf::Field(
                                FieldCondition {
                                    key: "profit_loss".to_string(),
                                    range: Some(Range {
                                        lt: Some(0.0),
                                        gt: None,
                                        gte: None,
                                        lte: None,
                                    }),
                                    ..Default::default()
                                },
                            ),
                        ),
                    });
                }
            } else if meta_filters.profit_loss_min.is_some()
                || meta_filters.profit_loss_max.is_some()
            {
                must_conditions.push(Condition {
                    condition_one_of: Some(
                        qdrant_client::qdrant::condition::ConditionOneOf::Field(FieldCondition {
                            key: "profit_loss".to_string(),
                            range: Some(Range {
                                lt: meta_filters.profit_loss_max,
                                gt: meta_filters.profit_loss_min,
                                gte: None,
                                lte: None,
                            }),
                            ..Default::default()
                        }),
                    ),
                });
            }

            // Symbol filter
            if let Some(symbol) = &meta_filters.symbol {
                must_conditions.push(Condition {
                    condition_one_of: Some(
                        qdrant_client::qdrant::condition::ConditionOneOf::Field(FieldCondition {
                            key: "symbol".to_string(),
                            r#match: Some(Match {
                                match_value: Some(
                                    qdrant_client::qdrant::r#match::MatchValue::Text(
                                        symbol.clone(),
                                    ),
                                ),
                            }),
                            ..Default::default()
                        }),
                    ),
                });
            }

            // Trade type filter
            if let Some(trade_type) = &meta_filters.trade_type {
                must_conditions.push(Condition {
                    condition_one_of: Some(
                        qdrant_client::qdrant::condition::ConditionOneOf::Field(FieldCondition {
                            key: "trade_type".to_string(),
                            r#match: Some(Match {
                                match_value: Some(
                                    qdrant_client::qdrant::r#match::MatchValue::Text(
                                        trade_type.clone(),
                                    ),
                                ),
                            }),
                            ..Default::default()
                        }),
                    ),
                });
            }
        }

        let filter = Filter {
            must: must_conditions,
            ..Default::default()
        };

        // Use ScrollPoints to get all matching points (no semantic search)
        let scroll_request = ScrollPoints {
            collection_name: collection_name.clone(),
            filter: Some(filter),
            limit: Some(limit as u32),
            with_payload: Some(true.into()),
            with_vectors: Some(false.into()),
            ..Default::default()
        };

        let scroll_result = self.client.scroll(scroll_request).await?;

        // Convert results to SearchResult
        let results: Vec<SearchResult> = scroll_result
            .result
            .into_iter()
            .map(|point| {
                let id = point
                    .payload
                    .get("id")
                    .and_then(|v| match &v.kind {
                        Some(Kind::StringValue(s)) => Some(s.clone()),
                        _ => None,
                    })
                    .unwrap_or_else(|| match &point.id {
                        Some(PointId {
                            point_id_options: Some(point_id_options),
                        }) => match point_id_options {
                            qdrant_client::qdrant::point_id::PointIdOptions::Uuid(u) => u.clone(),
                            qdrant_client::qdrant::point_id::PointIdOptions::Num(n) => {
                                n.to_string()
                            }
                        },
                        _ => "unknown".to_string(),
                    });

                let content = point
                    .payload
                    .get("content")
                    .and_then(|v| match &v.kind {
                        Some(Kind::StringValue(s)) => Some(s.clone()),
                        _ => None,
                    })
                    .unwrap_or_default();

                let r#type = point.payload.get("type").and_then(|v| match &v.kind {
                    Some(Kind::StringValue(s)) => Some(s.clone()),
                    _ => None,
                });

                let created_at = point.payload.get("created_at").and_then(|v| match &v.kind {
                    Some(Kind::StringValue(s)) => DateTime::parse_from_rfc3339(s)
                        .ok()
                        .map(|dt| dt.with_timezone(&Utc)),
                    _ => None,
                });

                SearchResult {
                    id,
                    score: 1.0, // No semantic score for scroll results
                    content,
                    r#type,
                    created_at,
                }
            })
            .collect();

        log::info!(
            "Retrieved all trades in period - collection={}, results={}",
            collection_name,
            results.len()
        );

        Ok(results)
    }

    /// Delete entire user collection from Qdrant
    pub async fn delete_user_collection(&self, user_id: &str) -> Result<()> {
        let collection_name = self.config.get_collection_name(user_id);

        log::info!("Deleting Qdrant collection: {}", collection_name);

        // Check if collection exists
        let collections = self.client.list_collections().await?;
        let exists = collections
            .collections
            .iter()
            .any(|c| c.name == collection_name);

        if !exists {
            log::info!(
                "Collection {} does not exist, skipping deletion",
                collection_name
            );
            return Ok(());
        }

        // Delete the collection
        self.client
            .delete_collection(collection_name.clone())
            .await
            .context("Failed to delete Qdrant collection")?;

        log::info!(
            "Successfully deleted Qdrant collection: {}",
            collection_name
        );
        Ok(())
    }

    /// Health check to verify Qdrant connection
    /// Returns Ok(()) if connection is healthy, Err otherwise
    pub async fn health_check(&self) -> Result<()> {
        log::info!("Performing Qdrant health check - URL: {}", self.config.url);

        // Retry logic for health check (network might be slow)
        let mut last_error = None;
        for attempt in 1..=self.config.max_retries {
            match self.client.list_collections().await {
                Ok(collections) => {
                    log::info!(
                        "Qdrant health check passed - connection is healthy. Found {} collections (attempt {})",
                        collections.collections.len(),
                        attempt
                    );
                    return Ok(());
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < self.config.max_retries {
                        log::warn!(
                            "Qdrant health check attempt {} failed, retrying... Error: {}",
                            attempt,
                            last_error.as_ref().unwrap()
                        );
                        tokio::time::sleep(tokio::time::Duration::from_millis(
                            500 * attempt as u64,
                        ))
                        .await;
                    }
                }
            }
        }

        let error_msg = format!(
            "Qdrant health check failed after {} attempts - unable to list collections. Last error: {}",
            self.config.max_retries,
            last_error
                .map(|e| e.to_string())
                .unwrap_or_else(|| "Unknown error".to_string())
        );
        log::error!("{}", error_msg);
        Err(anyhow::anyhow!(error_msg))
    }
}
