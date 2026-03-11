<!-- 38234a73-892d-4699-b74e-9a139604e3b9 5d1f13e2-cc64-442f-92ee-fe4953114fd9 -->
# AI Chat System Enhancement Plan

## Overview

Transform the AI chat system into an intelligent trading assistant with superior response quality, optimized performance, comprehensive data access, and enhanced streaming capabilities. This plan balances free and paid services strategically for maximum value.

## Current System Analysis

### Strengths

- **Solid Foundation**: RAG architecture with Upstash Vector + Voyager finance-specific embeddings
- **Multi-tenant**: Isolated user databases with Turso
- **Streaming Support**: Real-time response streaming with OpenRouter
- **Context-Aware**: Dynamic prompt templates based on query type detection
- **Modern Stack**: Rust backend (performance) + Next.js frontend (UX)

### Key Issues Identified

1. **Context Retrieval**: Basic vector similarity without reranking or hybrid search
2. **Limited Data Access**: Only vectorized trading data, missing real-time market context
3. **Streaming Token Management**: No token counting, context window optimization, or cost tracking
4. **Cache Strategy**: Minimal caching for repeated queries or similar contexts
5. **Prompt Engineering**: Good templates but missing few-shot examples and chain-of-thought
6. **Performance**: No request batching, connection pooling needs optimization
7. **Error Recovery**: Basic fallbacks without graceful degradation strategies

## Phase 1: Enhanced RAG & Context Retrieval (Priority: HIGH)

### 1.1 Implement Hybrid Search with Reranking

**Backend: Add Cohere Reranker Integration** (`backend/src/service/reranker_service.rs`)

```rust
// Free tier: 100 rerank requests/day is sufficient for most users
// Paid: $1.00 per 1000 searches for premium users
pub struct CohereReranker {
    api_key: String,
    model: String, // "rerank-english-v3.0" or "rerank-multilingual-v3.0"
    max_documents: usize,
}

impl CohereReranker {
    pub async fn rerank(
        &self,
        query: &str,
        documents: Vec<String>,
        top_n: usize,
    ) -> Result<Vec<RankedDocument>> {
        // POST to https://api.cohere.ai/v1/rerank
        // Returns documents with relevance scores
    }
}
```

**Backend: Enhance VectorizationService** (`backend/src/service/vectorization_service.rs`)

```rust
// Add hybrid search: vector similarity + keyword matching + reranking
pub async fn hybrid_search(
    &self,
    user_id: &str,
    query: &str,
    max_results: usize,
    enable_reranking: bool, // Toggle based on user tier
) -> Result<Vec<ContextSource>> {
    // 1. Vector similarity search (Upstash Vector)
    let vector_results = self.query_similar_vectors(...).await?;
    
    // 2. Keyword search on metadata (use Turso FTS if available)
    let keyword_results = self.keyword_search(user_id, query).await?;
    
    // 3. Merge and deduplicate
    let merged = self.merge_results(vector_results, keyword_results);
    
    // 4. Rerank with Cohere (if enabled)
    if enable_reranking {
        return self.cohere_reranker.rerank(query, merged, max_results).await;
    }
    
    Ok(merged.into_iter().take(max_results).collect())
}
```

### 1.2 Intelligent Context Window Management

**Backend: Token Counter & Context Manager** (`backend/src/service/context_manager.rs`)

```rust
pub struct ContextManager {
    model_limits: HashMap<String, ModelLimits>, // e.g., deepseek: 32k, gpt-4: 128k
    token_counter: Arc<TokenCounter>,
}

impl ContextManager {
    pub fn optimize_context(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
        context_sources: Vec<ContextSource>,
        reserved_for_response: usize, // e.g., 2048 tokens
    ) -> OptimizedContext {
        // 1. Count tokens in conversation history
        let history_tokens = self.count_message_tokens(&messages);
        
        // 2. Calculate available space for context
        let model_limit = self.model_limits.get(model).unwrap().max_tokens;
        let available = model_limit - history_tokens - reserved_for_response;
        
        // 3. Intelligently truncate context sources by relevance
        let optimized_sources = self.fit_context_to_budget(
            context_sources,
            available,
        );
        
        // 4. Apply sliding window for long conversations
        let optimized_messages = self.apply_sliding_window(
            messages,
            model_limit / 2, // Keep recent 50% of history
        );
        
        OptimizedContext {
            messages: optimized_messages,
            sources: optimized_sources,
            total_tokens: history_tokens,
            available_tokens: available,
        }
    }
}
```

### 1.3 Advanced Prompt Engineering

**Backend: Enhanced Prompt Templates** (`backend/src/models/ai/chat_templates.rs`)

Add:

- **Few-shot examples**: 3-5 high-quality Q&A pairs per query type
- **Chain-of-thought prompting**: Guide the model to explain reasoning
- **Structured output**: JSON schema for actionable insights
- **Context utilization hints**: Explicit instructions to reference sources
```rust
impl QueryPromptTemplate {
    pub fn performance_analysis_v2() -> Self {
        Self {
            query_patterns: vec!["performance", "return", "profit", "loss", "pnl"],
            system_prompt: r#"You are a quantitative trading analyst with expertise in performance attribution.

APPROACH:
1. Analyze the provided trading data systematically
2. Calculate key metrics (win rate, profit factor, Sharpe ratio)
3. Identify patterns and trends
4. Provide actionable recommendations

CONTEXT USAGE:
- Always cite specific trades when making points
- Reference date ranges and symbols explicitly
- Quantify observations with percentages and dollar amounts

FEW-SHOT EXAMPLES:
Q: "How's my performance this month?"
A: "Based on your 15 trades this month (Jan 2024):
- Total P&L: +$2,450 (+12.3% return)
- Win Rate: 66.7% (10 wins, 5 losses)
- Best trade: AAPL call on Jan 15 (+$680)
- Concern: Your average loss ($245) is higher than average win ($195), suggesting you're cutting winners too early.
Recommendation: Consider letting winners run longer - your winning trades averaged only 2.3 days vs 4.1 days for losses."

"#.to_string(),
            context_instructions: "Prioritize recent trades and aggregate statistics.",
            response_format: "Structure: Summary → Metrics → Analysis → Recommendations",
            max_tokens: 1024,
            temperature: 0.3, // Lower for analytical queries
        }
    }
}
```


## Phase 2: Performance Optimization (Priority: HIGH)

### 2.1 Multi-Layer Caching Strategy

**Backend: Intelligent Cache Layer** (`backend/src/service/cache_service.rs`)

```rust
pub struct CacheService {
    redis: Arc<RedisClient>,
    ttl_strategy: CacheTTLStrategy,
}

pub enum CacheLayer {
    // L1: In-memory LRU (ultra-fast, small)
    Memory { capacity: usize },
    // L2: Redis (fast, distributed)
    Redis { ttl: Duration },
    // L3: Turso (persistent, queryable)
    Database,
}

impl CacheService {
    // Cache similar queries (fuzzy matching)
    pub async fn get_or_compute_response(
        &self,
        user_id: &str,
        query: &str,
        compute_fn: impl Future<Output = Result<String>>,
    ) -> Result<CachedResponse> {
        // 1. Generate semantic hash of query
        let query_hash = self.semantic_hash(query);
        
        // 2. Check L1 cache (memory)
        if let Some(cached) = self.memory_cache.get(&query_hash) {
            return Ok(cached);
        }
        
        // 3. Check L2 cache (Redis)
        let cache_key = format!("chat:{}:{}", user_id, query_hash);
        if let Some(cached) = self.redis.get(&cache_key).await? {
            // Promote to L1
            self.memory_cache.put(query_hash, cached.clone());
            return Ok(cached);
        }
        
        // 4. Compute response
        let response = compute_fn.await?;
        
        // 5. Store in L2 (async, don't block)
        tokio::spawn(async move {
            let _ = self.redis.setex(&cache_key, 3600, &response).await;
        });
        
        Ok(response)
    }
    
    // Cache vector search results
    pub async fn cache_context_retrieval(
        &self,
        user_id: &str,
        query: &str,
        sources: Vec<ContextSource>,
    ) -> Result<()> {
        // Cache for 1 hour - trade data doesn't change frequently
        let key = format!("context:{}:{}", user_id, self.semantic_hash(query));
        self.redis.setex(&key, 3600, &sources).await
    }
}
```

### 2.2 Request Batching & Connection Pooling

**Backend: Batch Processor** (`backend/src/service/batch_processor.rs`)

```rust
pub struct BatchProcessor {
    voyager_client: Arc<VoyagerClient>,
    batch_window: Duration, // e.g., 100ms
    max_batch_size: usize, // e.g., 10 requests
}

impl BatchProcessor {
    // Collect multiple embedding requests and batch them
    pub async fn batch_embeddings(
        &self,
        requests: Vec<EmbeddingRequest>,
    ) -> Result<Vec<EmbeddingResponse>> {
        // Voyager API supports batch embeddings (up to 10 texts per request)
        let batches = requests.chunks(self.max_batch_size);
        
        let mut results = Vec::new();
        for batch in batches {
            let texts: Vec<String> = batch.iter().map(|r| r.text.clone()).collect();
            let embeddings = self.voyager_client.embed_batch(&texts).await?;
            results.extend(embeddings);
        }
        
        Ok(results)
    }
}
```

**Backend: Optimize Database Connections** (`backend/src/turso/client.rs`)

```rust
// Add connection pooling with deadpool or bb8
pub struct TursoConnectionPool {
    pools: DashMap<String, Pool<TursoConnectionManager>>,
    config: PoolConfig,
}

impl TursoConnectionPool {
    pub async fn get_connection(&self, user_id: &str) -> Result<PooledConnection> {
        // Get or create pool for user's database
        let pool = self.pools.entry(user_id.to_string())
            .or_insert_with(|| {
                Pool::builder()
                    .max_size(10) // 10 connections per user
                    .build(TursoConnectionManager::new(user_db_url))
            });
        
        pool.get().await
    }
}
```

### 2.3 Streaming Optimizations

**Backend: Enhanced Streaming with Token Management** (`backend/src/service/streaming_manager.rs`)

```rust
pub struct StreamingManager {
    token_counter: Arc<TokenCounter>,
    cost_tracker: Arc<CostTracker>,
}

impl StreamingManager {
    pub async fn stream_with_monitoring(
        &self,
        user_id: &str,
        model: &str,
        stream: mpsc::Receiver<String>,
    ) -> mpsc::Receiver<EnhancedStreamChunk> {
        let (tx, rx) = mpsc::channel(100);
        
        tokio::spawn(async move {
            let mut accumulated = String::new();
            let mut token_count = 0;
            let start_time = Instant::now();
            
            while let Some(chunk) = stream.recv().await {
                accumulated.push_str(&chunk);
                token_count += self.estimate_tokens(&chunk);
                
                // Send enhanced chunk with metadata
                let _ = tx.send(EnhancedStreamChunk {
                    content: chunk,
                    tokens_so_far: token_count,
                    elapsed_ms: start_time.elapsed().as_millis() as u64,
                    estimated_cost: self.calculate_cost(model, token_count),
                }).await;
            }
            
            // Log usage for analytics
            self.cost_tracker.record_usage(user_id, model, token_count).await;
        });
        
        rx
    }
}
```

## Phase 3: Expanded Context & Data Access (Priority: MEDIUM)

### 3.1 Real-Time Market Data Integration

**Backend: Market Data Context Provider** (`backend/src/service/market_context_provider.rs`)

```rust
pub struct MarketContextProvider {
    finnhub_client: Arc<FinnhubClient>, // Free tier: 60 calls/min
    alpha_vantage: Arc<AlphaVantageClient>, // Free tier: 25 calls/day
    cache: Arc<CacheService>,
}

impl MarketContextProvider {
    pub async fn enrich_query_with_market_data(
        &self,
        query: &str,
        user_trades: Vec<ContextSource>,
    ) -> Result<Vec<ContextSource>> {
        // 1. Extract symbols from query and user trades
        let symbols = self.extract_symbols(query, &user_trades);
        
        // 2. Fetch relevant market data (cached for 15 mins)
        let mut enriched = user_trades;
        
        for symbol in symbols {
            // Check cache first
            let cache_key = format!("market:{}:{}", symbol, Utc::now().format("%Y%m%d%H%M"));
            
            if let Some(cached) = self.cache.get(&cache_key).await? {
                enriched.push(cached);
                continue;
            }
            
            // Fetch fresh data (rate-limited)
            let quote = self.finnhub_client.get_quote(&symbol).await?;
            let news = self.finnhub_client.get_company_news(&symbol, 1).await?; // Last 24h
            
            let context = ContextSource {
                vector_id: format!("market_{}", symbol),
                entity_id: symbol.clone(),
                data_type: "market_data".to_string(),
                snippet: format!(
                    "{}: ${:.2} ({:+.2}%), Volume: {}, Latest news: {}",
                    symbol, quote.current, quote.change_percent,
                    quote.volume, news.first().map(|n| &n.headline).unwrap_or(&"None")
                ),
                similarity_score: 1.0, // Always relevant
            };
            
            // Cache for 15 minutes
            self.cache.set(&cache_key, &context, 900).await?;
            enriched.push(context);
        }
        
        Ok(enriched)
    }
}
```

### 3.2 User Preference & Psychology Integration

**Backend: User Profile Context** (`backend/src/models/user_profile.rs`)

```rust
#[derive(Serialize, Deserialize)]
pub struct TradingProfile {
    pub user_id: String,
    pub risk_tolerance: RiskLevel,
    pub preferred_strategies: Vec<String>,
    pub trading_psychology: PsychologyProfile,
    pub performance_history: PerformanceMetrics,
}

#[derive(Serialize, Deserialize)]
pub struct PsychologyProfile {
    pub emotional_patterns: Vec<EmotionalPattern>,
    pub bias_indicators: Vec<String>, // e.g., "overtrading", "revenge_trading"
    pub discipline_score: f32,
}

impl TradingProfile {
    pub fn to_context_string(&self) -> String {
        format!(
            "User Profile: Risk tolerance: {:?}, Strategies: {}, \
            Common challenges: {}, Historical win rate: {:.1}%",
            self.risk_tolerance,
            self.preferred_strategies.join(", "),
            self.trading_psychology.bias_indicators.join(", "),
            self.performance_history.win_rate * 100.0
        )
    }
}
```

**Backend: Integrate Profile into Context Retrieval**

```rust
// In ai_chat_service.rs
async fn retrieve_context_with_profile(
    &self,
    user_id: &str,
    query: &str,
    max_vectors: usize,
) -> Result<Vec<ContextSource>> {
    // 1. Get standard context (trades, notes)
    let mut context = self.hybrid_search(user_id, query, max_vectors).await?;
    
    // 2. Add market data for mentioned symbols
    context = self.market_context_provider
        .enrich_query_with_market_data(query, context).await?;
    
    // 3. Add user profile context
    if let Ok(profile) = self.get_user_profile(user_id).await {
        context.push(ContextSource {
            vector_id: format!("profile_{}", user_id),
            entity_id: user_id.to_string(),
            data_type: "user_profile".to_string(),
            snippet: profile.to_context_string(),
            similarity_score: 0.95, // Always highly relevant
        });
    }
    
    Ok(context)
}
```

### 3.3 Intelligent Data Filtering

**Backend: Context Filter Service** (`backend/src/service/context_filter.rs`)

```rust
pub struct ContextFilter {
    relevance_threshold: f32,
    diversity_factor: f32,
}

impl ContextFilter {
    pub fn filter_and_rank(
        &self,
        query: &str,
        sources: Vec<ContextSource>,
        max_results: usize,
    ) -> Vec<ContextSource> {
        // 1. Remove low-relevance sources
        let filtered: Vec<_> = sources.into_iter()
            .filter(|s| s.similarity_score >= self.relevance_threshold)
            .collect();
        
        // 2. Apply MMR (Maximal Marginal Relevance) for diversity
        let diverse = self.apply_mmr(filtered, self.diversity_factor);
        
        // 3. Rank by freshness + relevance
        let mut ranked = diverse;
        ranked.sort_by(|a, b| {
            let score_a = self.compute_ranking_score(a, query);
            let score_b = self.compute_ranking_score(b, query);
            score_b.partial_cmp(&score_a).unwrap()
        });
        
        ranked.into_iter().take(max_results).collect()
    }
    
    fn compute_ranking_score(&self, source: &ContextSource, query: &str) -> f32 {
        // Combine relevance, recency, and data type importance
        let relevance = source.similarity_score;
        let recency = self.calculate_recency_score(&source.entity_id);
        let type_weight = self.get_data_type_weight(&source.data_type, query);
        
        relevance * 0.5 + recency * 0.3 + type_weight * 0.2
    }
}
```

## Phase 4: Frontend Enhancements (Priority: MEDIUM)

### 4.1 Better Streaming UI with Token Display

**Frontend: Enhanced Chat Interface** (`app/app/chat/[chatId]/page.tsx`)

```typescript
// Add streaming metadata display
interface StreamingMetadata {
  tokensUsed: number;
  estimatedCost: number;
  elapsedMs: number;
}

// Update sendStreamingMessage to handle metadata
const [streamingMetadata, setStreamingMetadata] = useState<StreamingMetadata | null>(null);

// Display in UI
{isStreaming && streamingMetadata && (
  <div className="text-xs text-muted-foreground flex items-center gap-4 px-4 py-2">
    <span>Tokens: {streamingMetadata.tokensUsed}</span>
    <span>Cost: ${streamingMetadata.estimatedCost.toFixed(4)}</span>
    <span>Time: {(streamingMetadata.elapsedMs / 1000).toFixed(1)}s</span>
  </div>
)}
```

### 4.2 Context Source Display

```typescript
// Show which sources the AI used
interface ContextSourceDisplay {
  id: string;
  type: 'trade' | 'note' | 'market' | 'profile';
  label: string;
  relevance: number;
}

// Add to message component
{message.context_sources && (
  <div className="mt-2 flex flex-wrap gap-2">
    {message.context_sources.map(source => (
      <Badge key={source.id} variant="outline" className="text-xs">
        <Tooltip>
          <TooltipTrigger>{source.label}</TooltipTrigger>
          <TooltipContent>
            Type: {source.type} | Relevance: {(source.relevance * 100).toFixed(0)}%
          </TooltipContent>
        </Tooltip>
      </Badge>
    ))}
  </div>
)}
```

## Phase 5: Model Strategy & Cost Optimization (Priority: MEDIUM)

### 5.1 Tiered Model Selection

**Backend: Model Router** (`backend/src/service/model_router.rs`)

```rust
pub struct ModelRouter {
    user_tier_service: Arc<UserTierService>,
}

pub enum ModelTier {
    Free,      // deepseek-chat-v3.1:free, llama-3.1-8b:free
    Standard,  // gpt-3.5-turbo ($0.002/1K), claude-3-haiku ($0.00025/1K)
    Premium,   // gpt-4-turbo ($0.01/1K), claude-3.5-sonnet ($0.003/1K)
}

impl ModelRouter {
    pub async fn select_model(
        &self,
        user_id: &str,
        query_complexity: QueryComplexity,
        estimated_tokens: usize,
    ) -> Result<ModelConfig> {
        let user_tier = self.user_tier_service.get_tier(user_id).await?;
        
        match (user_tier, query_complexity) {
            // Free users: always use free models
            (UserTier::Free, _) => Ok(ModelConfig {
                provider: "openrouter",
                model: "deepseek/deepseek-chat-v3.1:free",
                max_tokens: 4096,
            }),
            
            // Standard users: use paid for complex queries
            (UserTier::Standard, QueryComplexity::Complex) => Ok(ModelConfig {
                provider: "anthropic",
                model: "claude-3-haiku-20240307",
                max_tokens: 4096,
            }),
            (UserTier::Standard, _) => Ok(ModelConfig {
                provider: "openrouter",
                model: "deepseek/deepseek-chat-v3.1:free",
                max_tokens: 4096,
            }),
            
            // Premium users: best models always
            (UserTier::Premium, QueryComplexity::Complex) => Ok(ModelConfig {
                provider: "anthropic",
                model: "claude-3-5-sonnet-20240620",
                max_tokens: 8192,
            }),
            (UserTier::Premium, _) => Ok(ModelConfig {
                provider: "openai",
                model: "gpt-3.5-turbo",
                max_tokens: 4096,
            }),
        }
    }
}
```

### 5.2 Cost Tracking & Alerts

```rust
pub struct CostTracker {
    redis: Arc<RedisClient>,
    alert_thresholds: HashMap<UserTier, f32>,
}

impl CostTracker {
    pub async fn track_request(
        &self,
        user_id: &str,
        model: &str,
        tokens_used: usize,
    ) -> Result<UsageStats> {
        let cost = self.calculate_cost(model, tokens_used);
        
        // Increment monthly usage
        let key = format!("usage:{}:{}", user_id, Utc::now().format("%Y%m"));
        self.redis.incrbyfloat(&key, cost).await?;
        
        // Check against threshold
        let total = self.redis.get::<f32>(&key).await?.unwrap_or(0.0);
        let threshold = self.alert_thresholds.get(&user_tier).unwrap_or(&10.0);
        
        if total > *threshold {
            self.send_cost_alert(user_id, total).await?;
        }
        
        Ok(UsageStats { total, cost })
    }
}
```

## Implementation Priority

### Week 1-2: Core RAG Improvements

1. Implement hybrid search with reranking ✓
2. Add context window management ✓
3. Enhance prompt templates with few-shot examples ✓

### Week 3-4: Performance Optimization

4. Multi-layer caching (Redis + in-memory) ✓
5. Request batching for embeddings ✓
6. Streaming token management ✓

### Week 5-6: Expanded Context

7. Market data integration (Finnhub free tier) ✓
8. User profile context ✓
9. Intelligent context filtering ✓

### Week 7-8: Model Strategy & Polish

10. Tiered model selection ✓
11. Cost tracking dashboard ✓
12. Frontend streaming enhancements ✓

## Success Metrics

- **Response Quality**: User satisfaction rating > 4.5/5
- **Accuracy**: Context relevance score > 85%
- **Performance**: P95 latency < 2s for streaming start
- **Cost**: Stay under $0.50/user/month for free tier
- **Cache Hit Rate**: > 40% for similar queries
- **Token Efficiency**: Reduce average tokens/response by 20%

## Files to Modify/Create

### Backend (Rust)

- `backend/src/service/reranker_service.rs` (new)
- `backend/src/service/context_manager.rs` (new)
- `backend/src/service/cache_service.rs` (new)
- `backend/src/service/streaming_manager.rs` (new)
- `backend/src/service/market_context_provider.rs` (new)
- `backend/src/service/model_router.rs` (new)
- `backend/src/service/vectorization_service.rs` (enhance)
- `backend/src/service/ai_chat_service.rs` (enhance)
- `backend/src/models/ai/chat_templates.rs` (enhance)
- `backend/Cargo.toml` (add dependencies)

### Frontend (Next.js/TypeScript)

- `hooks/use-ai-chat.ts` (enhance streaming)
- `app/app/chat/[chatId]/page.tsx` (add metadata display)
- `lib/services/ai-chat-service.ts` (handle enhanced responses)
- `lib/types/ai-chat.ts` (add new types)

### Configuration

- `backend/.env` (add Cohere, Finnhub API keys)
- `backend/src/turso/vector_config.rs` (model routing config)

### To-dos

- [ ] Implement hybrid search with Cohere reranking for better context retrieval
- [ ] Build intelligent context window manager with token counting and optimization
- [ ] Upgrade prompt templates with few-shot examples and chain-of-thought reasoning
- [ ] Implement L1 (memory) + L2 (Redis) caching strategy for responses and context
- [ ] Add request batching for embedding generation to reduce API calls
- [ ] Enhance streaming with real-time token counting, cost tracking, and metadata
- [ ] Integrate Finnhub API for real-time market data context enrichment
- [ ] Create user trading profile system and integrate into context retrieval
- [ ] Build context filter with MMR for diversity and smart ranking
- [ ] Implement tiered model selection strategy (free/standard/premium)
- [ ] Build cost tracking system with usage analytics and alerts
- [ ] Enhance chat UI with token display, cost info, and context source badges