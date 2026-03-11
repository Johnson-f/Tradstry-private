<!-- d742f309-78cb-4b2b-a535-24324b353628 9a53d5b2-805c-4ae0-b218-01d99b936afa -->
# Vector Database AI System Implementation Plan

## Overview

Implement a three-service AI system (AI Chat, AI Insights, AI Reports) with Upstash Vector database, Voyager finance embeddings, and Google Gemini LLM integration on Rust backend.

## Architecture Components

### 1. Vector Database Layer (Upstash Vector)

- Use Upstash Vector REST API (no native Rust SDK available)
- Store vector embeddings of all user trading data
- Support similarity search for context retrieval

### 2. Embedding Layer (Voyager voyage-finance-2)

- Finance-specific embedding model with 32K context length
- Optimized for financial trading data retrieval
- HTTP API integration via reqwest

### 3. LLM Layer (Google Gemini)

- Primary LLM for chat, insights, and report generation
- Context-aware responses using vector similarity search
- HTTP API integration via reqwest

### 4. Storage Layer

- **Vector Storage**: Upstash Vector (serverless, distributed)
- **Insights Storage**: Turso user databases (persistent, queryable)
- **Cache**: Existing Upstash Redis (metadata, quick access)

## Implementation Steps

### Phase 1: Infrastructure Setup

#### 1.1 Add Dependencies to Cargo.toml

```toml
# HTTP client for API calls (Voyager, Upstash Vector)
reqwest = { version = "0.12", features = ["json", "rustls-tls-webpki-roots"] }

# AI/LLM Integration
genai = "0.4.0" # Unified interface for Gemini and other LLMs

# Additional serialization
bincode = "1.3" # For efficient binary vector storage
serde_json = "1.0" # JSON handling for API requests/responses

# Async runtime (already included, but ensure full features)
tokio = { version = "1.0", features = ["full"] }
```

**Integration Method Decision:**

1. **Voyager (voyage-finance-2)**: Use `reqwest` for direct HTTP API calls

   - **Reason**: No official Rust SDK available
   - **Pros**: Direct control, flexible, no external dependencies
   - **Cons**: Manual error handling, need to implement retry logic

2. **Google Gemini**: Use `genai` crate (version 0.4.0)

   - **Reason**: Official Rust client with unified interface
   - **Pros**: Well-maintained, abstracts API complexity, handles auth/retries
   - **Cons**: Additional dependency (but worth it for reliability)

3. **Upstash Vector**: Use `reqwest` for REST API calls

   - **Reason**: No official Rust SDK, REST API is well-documented
   - **Pros**: Simple HTTP interface, serverless-friendly
   - **Cons**: Manual implementation required

#### 1.2 Create Upstash Vector Configuration

**File**: `backend/src/turso/vector_config.rs`

- Store Upstash Vector URL and token
- Connection configuration
- Vector dimensions (voyage-finance-2 uses 1024 dimensions)

#### 1.3 Create Voyager Client

**File**: `backend/src/service/voyager_client.rs`

- HTTP client for Voyager API
- Generate embeddings for text data
- Handle API errors and retries
- Batch embedding generation support

#### 1.4 Create Upstash Vector Client

**File**: `backend/src/service/upstash_vector_client.rs`

- REST API client for Upstash Vector
- Upsert vectors with metadata
- Query vectors by similarity
- Delete vectors
- Namespace management per user

#### 1.5 Create Gemini Client

**File**: `backend/src/service/gemini_client.rs`

- HTTP client for Google Gemini API
- Chat completion with context
- **Streaming support (required for real-time token-by-token responses)**
- Server-Sent Events (SSE) implementation
- Token management and error handling

### Phase 2: Data Vectorization System

#### 2.1 Create Vectorization Service

**File**: `backend/src/service/vectorization_service.rs`

- Main orchestrator for vectorization
- Format trading data for embedding
- Store vectors in Upstash with metadata
- Handle real-time and batch updates

#### 2.2 Define Vector Metadata Schema

```rust
struct VectorMetadata {
    user_id: String,
    data_type: DataType, // Stock, Option, TradeNote, Notebook, Playbook
    entity_id: String,
    timestamp: DateTime<Utc>,
    tags: Vec<String>,
}

enum DataType {
    Stock,
    Option,
    TradeNote,
    NotebookEntry,
    PlaybookStrategy,
}
```

#### 2.3 Implement Data Formatters

**File**: `backend/src/service/data_formatter.rs`

- `format_stock_for_embedding()`: Convert stock trade to text
- `format_option_for_embedding()`: Convert option trade to text
- `format_trade_note_for_embedding()`: Extract relevant note content
- `format_notebook_for_embedding()`: Format notebook entries
- `format_playbook_for_embedding()`: Format playbook strategies

Example format:

```
"Stock trade: BUY 100 shares of AAPL at $150.00 on 2024-01-15. 
Exit: SELL at $160.00 on 2024-01-20. 
P&L: $1000.00 (6.67% gain). 
Notes: Strong earnings beat, momentum trade."
```

#### 2.4 Implement Real-Time Vectorization Hooks

**Locations**:

- `backend/src/routes/stocks.rs` - After create/update/delete
- `backend/src/routes/options.rs` - After create/update/delete
- `backend/src/routes/trade_notes.rs` - After create/update/delete
- `backend/src/routes/notebook.rs` - After create/update/delete
- `backend/src/routes/playbook.rs` - After create/update/delete

Pattern:

```rust
// After successful trade creation
tokio::spawn(async move {
    vectorization_service.vectorize_trade(user_id, trade_id).await;
});
```

#### 2.5 Implement Batch Vectorization Cron Job

**File**: `backend/src/service/batch_vectorization.rs`

- Scheduled task runner using tokio::time::interval
- Process unvectorized data in batches
- Update existing vectors if data changed
- Run every hour for non-critical updates

### Phase 3: AI Chat Service

#### 3.1 Create AI Chat Models

**File**: `backend/src/models/ai/chat.rs`

```rust
struct ChatMessage {
    role: MessageRole, // User, Assistant, System
    content: String,
    timestamp: DateTime<Utc>,
}

struct ChatSession {
    id: String,
    user_id: String,
    messages: Vec<ChatMessage>,
    context_vectors: Vec<String>, // Vector IDs used for context
}

struct ChatRequest {
    user_id: String,
    message: String,
    session_id: Option<String>,
}

struct ChatResponse {
    message: String,
    session_id: String,
    sources: Vec<ContextSource>, // What data influenced the response
}
```

#### 3.2 Create AI Chat Service

**File**: `backend/src/service/ai_chat_service.rs`

- Retrieve relevant context via vector similarity search
- Build system prompt with user trading context
- Send to Gemini API with context
- Store conversation in Turso
- Track which vectors influenced response

Functions:

- `generate_response()`: Main chat handler
- `retrieve_context()`: Get relevant trading data via vector search
- `build_system_prompt()`: Create context-aware prompt
- `store_conversation()`: Save to database

#### 3.3 Create Chat Routes

**File**: `backend/src/routes/ai_chat.rs`

- `POST /api/ai/chat` - Send message, get response
- `GET /api/ai/chat/sessions` - List user's chat sessions
- `GET /api/ai/chat/sessions/{id}` - Get session history
- `DELETE /api/ai/chat/sessions/{id}` - Delete session

#### 3.4 Create Chat Database Schema

**File**: `backend/database/08_ai_chat/01_chat_tables.sql`

```sql
CREATE TABLE IF NOT EXISTS chat_sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    title TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS chat_messages (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    context_vectors TEXT, -- JSON array of vector IDs
    created_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES chat_sessions(id) ON DELETE CASCADE
);
```

### Phase 4: AI Insights Service

#### 4.1 Create Insights Models

**File**: `backend/src/models/ai/insights.rs`

```rust
struct InsightRequest {
    user_id: String,
    time_range: TimeRange,
    insight_type: InsightType,
}

enum InsightType {
    TradingPatterns,
    PerformanceAnalysis,
    RiskAssessment,
    BehavioralAnalysis,
}

struct Insight {
    id: String,
    user_id: String,
    time_range: TimeRange,
    insight_type: InsightType,
    title: String,
    content: String,
    key_findings: Vec<String>,
    recommendations: Vec<String>,
    data_sources: Vec<String>, // Which trades/data informed this
    generated_at: DateTime<Utc>,
}
```

#### 4.2 Create Insights Service

**File**: `backend/src/service/ai_insights_service.rs`

- Fetch analytics data for time period
- Retrieve relevant trading data via vectors
- Generate insights using Gemini
- Store insights in Turso
- Support on-demand and scheduled generation

Functions:

- `generate_insights()`: Main insight generator
- `analyze_trading_patterns()`: Pattern detection
- `analyze_performance()`: Performance metrics analysis
- `analyze_risk()`: Risk assessment
- `analyze_behavior()`: Behavioral patterns

#### 4.3 Create Insights Routes

**File**: `backend/src/routes/ai_insights.rs`

- `POST /api/ai/insights/generate` - Generate new insights
- `GET /api/ai/insights` - List user's insights
- `GET /api/ai/insights/{id}` - Get specific insight
- `DELETE /api/ai/insights/{id}` - Delete insight

Query parameters:

- `time_range`: 7d, 30d, 90d, ytd, 1y
- `insight_type`: patterns, performance, risk, behavior

#### 4.4 Create Insights Database Schema

**File**: `backend/database/08_ai_chat/02_insights_tables.sql`

```sql
CREATE TABLE IF NOT EXISTS ai_insights (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    time_range TEXT NOT NULL,
    insight_type TEXT NOT NULL,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    key_findings TEXT, -- JSON array
    recommendations TEXT, -- JSON array
    data_sources TEXT, -- JSON array
    generated_at TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX idx_insights_user_time ON ai_insights(user_id, time_range);
CREATE INDEX idx_insights_type ON ai_insights(insight_type);
```

#### 4.5 Implement Scheduled Insights Generation

**File**: `backend/src/service/scheduled_insights.rs`

- Background task using tokio
- Run daily at configurable time
- Generate insights for common periods (7d, 30d)
- Store results for quick access
- Notify users of new insights (optional)

### Phase 5: AI Reports Service

#### 5.1 Create Reports Models

**File**: `backend/src/models/ai/reports.rs`

```rust
struct ReportRequest {
    user_id: String,
    time_range: TimeRange,
    report_type: ReportType,
    include_sections: Vec<ReportSection>,
}

enum ReportType {
    Comprehensive,
    Performance,
    Risk,
    Trading,
}

enum ReportSection {
    Summary,
    Analytics,
    Insights,
    Trades,
    Patterns,
    Recommendations,
}

struct TradingReport {
    id: String,
    user_id: String,
    time_range: TimeRange,
    report_type: ReportType,
    summary: String,
    analytics: AnalyticsData,
    insights: Vec<Insight>,
    trades: Vec<TradeData>,
    recommendations: Vec<String>,
    generated_at: DateTime<Utc>,
}

struct AnalyticsData {
    total_pnl: String,
    win_rate: String,
    profit_factor: String,
    // ... include all analytics from existing endpoints
}
```

#### 5.2 Create Reports Service

**File**: `backend/src/service/ai_reports_service.rs`

- Aggregate analytics data
- Fetch relevant insights
- Retrieve trade data
- Generate comprehensive analysis with Gemini
- Format for frontend consumption

Functions:

- `generate_report()`: Main report generator
- `fetch_analytics()`: Get analytics data
- `fetch_insights()`: Get or generate insights
- `fetch_trades()`: Get trade data
- `generate_summary()`: AI-generated executive summary
- `generate_recommendations()`: AI-generated recommendations

#### 5.3 Create Reports Routes

**File**: `backend/src/routes/ai_reports.rs`

- `POST /api/ai/reports/generate` - Generate new report
- `GET /api/ai/reports` - List user's reports
- `GET /api/ai/reports/{id}` - Get specific report
- `DELETE /api/ai/reports/{id}` - Delete report
- `GET /api/ai/reports/{id}/pdf` - Export as PDF (optional)

#### 5.4 Create Reports Database Schema

**File**: `backend/database/08_ai_chat/03_reports_tables.sql`

```sql
CREATE TABLE IF NOT EXISTS ai_reports (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    time_range TEXT NOT NULL,
    report_type TEXT NOT NULL,
    summary TEXT NOT NULL,
    analytics TEXT NOT NULL, -- JSON
    insights TEXT NOT NULL, -- JSON array
    trades TEXT NOT NULL, -- JSON array
    recommendations TEXT NOT NULL, -- JSON array
    generated_at TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX idx_reports_user_time ON ai_reports(user_id, time_range);
CREATE INDEX idx_reports_type ON ai_reports(report_type);
```

### Phase 6: Integration & Configuration

#### 6.1 Update AppState

**File**: `backend/src/turso/mod.rs`

```rust
pub struct AppState {
    // ... existing fields
    pub vectorization_service: Arc<VectorizationService>,
    pub ai_chat_service: Arc<AIChatService>,
    pub ai_insights_service: Arc<AIInsightsService>,
    pub ai_reports_service: Arc<AIReportsService>,
}
```

#### 6.2 Update Environment Configuration

**File**: `backend/.env`

```bash
# Voyager API
VOYAGER_API_KEY=your_voyager_api_key
VOYAGER_API_URL=https://api.voyageai.com/v1

# Upstash Vector
UPSTASH_VECTOR_URL=your_upstash_vector_url
UPSTASH_VECTOR_TOKEN=your_upstash_vector_token

# Google Gemini
GEMINI_API_KEY=your_gemini_api_key
GEMINI_MODEL=gemini-2.5-flash # or gemini-1.5-flash for faster responses

# AI Configuration
AI_INSIGHTS_SCHEDULE_ENABLED=true
AI_INSIGHTS_SCHEDULE_HOUR=2 # 2 AM UTC
BATCH_VECTORIZATION_INTERVAL_MINUTES=60
MAX_CONTEXT_VECTORS=10 # How many vectors to retrieve for context
```

#### 6.3 Register Routes in main.rs

```rust
.configure(configure_ai_chat_routes)
.configure(configure_ai_insights_routes)
.configure(configure_ai_reports_routes)
```

#### 6.4 Initialize Services in main.rs

```rust
let voyager_client = Arc::new(VoyagerClient::new(...));
let upstash_vector = Arc::new(UpstashVectorClient::new(...));
let gemini_client = Arc::new(GeminiClient::new(...));
let vectorization_service = Arc::new(VectorizationService::new(...));
// ... initialize other services
```

### Phase 7: Advanced Features & Optimization

#### 7.1 Implement Smart Data Chunking

**File**: `backend/src/service/data_chunker.rs`

**Purpose**: Split large content (notebooks, playbooks) into optimal chunks for embeddings

```rust
pub struct DataChunker {
    max_chunk_size: usize,
    overlap_size: usize,
}

pub struct DataChunk {
    chunk_id: String,
    parent_id: String,
    chunk_index: u32,
    content: String,
    chunk_type: ChunkType,
    metadata: ChunkMetadata,
}

pub enum ChunkType {
    Summary,       // High-level overview
    Detail,        // Detailed content
    Code,          // Code snippets
    Strategy,      // Trading strategies
    RiskAnalysis,  // Risk-related content
}

pub struct ChunkMetadata {
    word_count: usize,
    token_estimate: usize,
    importance_score: f32,
}
```

**Implementation**:

- Chunk notebook entries > 1000 words
- Chunk playbook strategies > 500 words
- Smart chunking based on content structure (paragraphs, sections)
- Maintain context overlap between chunks
- Assign importance scores to prioritize chunks

**Integration**:

- Modify `data_formatter.rs` to use chunking for large content
- Store chunk relationships in vector metadata
- Retrieve parent document when chunk matches query

#### 7.2 Multi-Level Cache Architecture

**File**: `backend/src/service/ai_cache_service.rs`

**Purpose**: Implement tiered caching for optimal performance

```rust
pub struct AICacheService {
    l1_cache: Arc<Mutex<LruCache<String, CachedVector>>>, // In-memory (100MB)
    l2_cache: Arc<CacheService>, // Redis (existing)
    vector_db: Arc<UpstashVectorClient>, // Vector database
    metrics: CacheMetrics,
}

pub struct CachedVector {
    vector: Vec<f32>,
    metadata: VectorMetadata,
    last_accessed: DateTime<Utc>,
    access_count: u32,
    ttl: Duration,
}

pub struct CacheMetrics {
    l1_hits: AtomicU64,
    l1_misses: AtomicU64,
    l2_hits: AtomicU64,
    l2_misses: AtomicU64,
    evictions: AtomicU64,
}
```

**Cache Strategy**:

- **L1 (In-memory)**: Hot vectors, 5-minute TTL, LRU eviction
- **L2 (Redis)**: Warm vectors, 30-minute TTL
- **L3 (Vector DB)**: All vectors, persistent

**Cache Warming**:

```rust
async fn warm_cache_for_user(&self, user_id: &str) -> Result<()> {
    // Preload recent trades (last 30 days)
    // Preload frequently accessed patterns
    // Preload user's common queries
}
```

**Cache Invalidation**:

```rust
async fn invalidate_user_vectors(&self, user_id: &str, entity_ids: &[String]) -> Result<()> {
    // Clear from L1 and L2 when data changes
    // Update vector in L3 (Upstash Vector)
}
```

#### 7.3 Add AI Service Orchestrator

**File**: `backend/src/service/ai_orchestrator.rs`

**Purpose**: Centralized request routing and service coordination

```rust
pub struct AIOrchestrator {
    chat_service: Arc<AIChatService>,
    insights_service: Arc<AIInsightsService>,
    reports_service: Arc<AIReportsService>,
    analytics_pipeline: Arc<RealTimeAnalytics>,
    cache_service: Arc<AICacheService>,
    rate_limiter: Arc<RateLimiter>,
    circuit_breaker: Arc<CircuitBreaker>,
}

pub enum AIRequest {
    Chat { session_id: String, message: String },
    Insight { time_range: TimeRange, insight_type: InsightType },
    Report { time_range: TimeRange, report_type: ReportType },
    HybridAnalysis { query: String, include_predictions: bool },
}

pub enum AIResponse {
    Chat(ChatResponse),
    Insight(Insight),
    Report(TradingReport),
    HybridAnalysis(HybridAnalysisResponse),
}
```

**Features**:

- Unified request handling
- Automatic retry with backoff
- Circuit breaker pattern
- Rate limiting per user
- Request queuing for high load
- Service health monitoring

**Hybrid Analysis**:

```rust
async fn handle_hybrid_request(&self, request: HybridAnalysisRequest) -> Result<AIResponse> {
    // Combine chat, insights, and predictions
    let (chat, insights, predictions) = tokio::join!(
        self.chat_service.generate_response(&request.query),
        self.insights_service.generate_quick_insights(&request.user_id),
        self.analytics_pipeline.predict_next_period(&request.user_id)
    );
    
    Ok(AIResponse::HybridAnalysis(HybridAnalysisResponse {
        answer: chat?,
        insights: insights?,
        predictions: predictions?,
    }))
}
```

#### 7.4 Add Context-Aware Prompt Engineering

**File**: `backend/src/service/prompt_builder.rs`

**Purpose**: Dynamic, intelligent prompt construction

```rust
pub struct PromptBuilder {
    user_profile: UserProfile,
    trading_context: TradingContext,
    conversation_history: Vec<ChatMessage>,
}

pub struct UserProfile {
    user_id: String,
    trading_style: TradingStyle,
    experience_level: ExperienceLevel,
    risk_tolerance: RiskTolerance,
    preferred_assets: Vec<String>,
    timezone: String,
}

pub enum TradingStyle {
    DayTrader,
    SwingTrader,
    PositionTrader,
    Scalper,
    ValueInvestor,
}

pub struct TradingContext {
    recent_performance: PerformanceSnapshot,
    active_positions: Vec<Position>,
    recent_patterns: Vec<Pattern>,
    market_conditions: MarketConditions,
}
```

**Prompt Templates**:

```rust
impl PromptBuilder {
    fn build_chat_prompt(&self, user_message: &str, context_data: &[ContextData]) -> String {
        format!(
            "You are an expert trading AI assistant for {name}, a {style} trader with {experience} experience.

TRADER PROFILE:
- Trading Style: {style}
- Risk Tolerance: {risk}
- Win Rate (30d): {win_rate}%
- P&L (30d): ${pnl}

RELEVANT TRADING DATA:
{context_summary}

CURRENT MARKET CONDITIONS:
{market_conditions}

USER QUESTION: {message}

Provide specific, actionable advice based on their trading history and current market conditions. 
Reference specific trades when relevant. Be concise but thorough.",
            name = self.user_profile.name,
            style = self.user_profile.trading_style,
            experience = self.user_profile.experience_level,
            risk = self.user_profile.risk_tolerance,
            win_rate = self.trading_context.recent_performance.win_rate,
            pnl = self.trading_context.recent_performance.pnl,
            context_summary = self.format_context_data(context_data),
            market_conditions = self.format_market_conditions(),
            message = user_message
        )
    }
    
    fn build_insight_prompt(&self, analytics: &AnalyticsData) -> String {
        // Template for insights generation
    }
    
    fn build_report_prompt(&self, data: &ReportData) -> String {
        // Template for report generation
    }
}
```

#### 7.5 Add Real-Time Analytics Pipeline

**File**: `backend/src/service/realtime_analytics.rs`

**Purpose**: Live metrics calculation and monitoring

```rust
pub struct RealTimeAnalytics {
    metrics_calculator: Arc<MetricsCalculator>,
    pattern_detector: Arc<PatternDetector>,
    anomaly_detector: Arc<AnomalyDetector>,
    stream_processor: Arc<StreamProcessor>,
}

pub struct LiveMetrics {
    current_pnl: f64,
    win_rate_24h: f64,
    trade_frequency: f64,
    risk_score: f64,
    momentum_score: f64,
    alerts: Vec<Alert>,
    last_updated: DateTime<Utc>,
}

pub struct Alert {
    alert_type: AlertType,
    severity: Severity,
    message: String,
    triggered_at: DateTime<Utc>,
}

pub enum AlertType {
    LargeLoss,
    UnusualActivity,
    RiskThresholdExceeded,
    OpportunityDetected,
    PatternMatched,
}
```

**Implementation**:

```rust
impl RealTimeAnalytics {
    async fn process_trade_event(&self, trade: &Trade) -> Result<()> {
        // Update live metrics
        self.update_live_metrics(trade).await?;
        
        // Detect patterns
        if let Some(pattern) = self.pattern_detector.analyze(trade).await? {
            self.trigger_pattern_alert(pattern).await?;
        }
        
        // Check anomalies
        if let Some(anomaly) = self.anomaly_detector.check(trade).await? {
            self.trigger_anomaly_alert(anomaly).await?;
        }
        
        Ok(())
    }
    
    async fn calculate_risk_score(&self, user_id: &str) -> Result<f64> {
        let recent_trades = self.get_recent_trades(user_id, Duration::days(7)).await?;
        
        let factors = RiskFactors {
            position_concentration: self.calculate_concentration(&recent_trades),
            leverage_usage: self.calculate_leverage(&recent_trades),
            volatility_exposure: self.calculate_volatility(&recent_trades),
            drawdown_magnitude: self.calculate_drawdown(&recent_trades),
        };
        
        Ok(factors.compute_score())
    }
}
```

#### 7.6 Add Predictive Analytics

**File**: `backend/src/service/predictive_analytics.rs`

**Purpose**: Forecast trading performance and patterns

```rust
pub struct PredictiveAnalytics {
    trend_predictor: Arc<TrendPredictor>,
    performance_predictor: Arc<PerformancePredictor>,
    risk_predictor: Arc<RiskPredictor>,
    ml_model: Arc<MLModel>,
}

pub struct PerformancePrediction {
    expected_pnl: f64,
    expected_win_rate: f64,
    confidence_score: f64,
    prediction_range: (f64, f64), // Min, Max
    risk_factors: Vec<RiskFactor>,
    opportunities: Vec<Opportunity>,
    recommendations: Vec<String>,
}

pub struct TrendPrediction {
    trend_direction: TrendDirection,
    strength: f64,
    duration_estimate: Duration,
    key_indicators: Vec<Indicator>,
}
```

**Implementation**:

```rust
impl PredictiveAnalytics {
    async fn predict_next_week_performance(&self, user_id: &str) -> Result<PerformancePrediction> {
        // Get historical data (90 days)
        let historical = self.get_historical_data(user_id, Duration::days(90)).await?;
        
        // Extract patterns
        let patterns = self.extract_patterns(&historical).await?;
        
        // Calculate trend momentum
        let momentum = self.calculate_momentum(&historical);
        
        // Generate prediction
        let prediction = self.ml_model.predict(PredictionInput {
            historical_data: historical,
            patterns,
            momentum,
            market_conditions: self.get_market_conditions().await?,
        }).await?;
        
        Ok(PerformancePrediction {
            expected_pnl: prediction.pnl,
            expected_win_rate: prediction.win_rate,
            confidence_score: prediction.confidence,
            prediction_range: prediction.range,
            risk_factors: self.identify_risk_factors(&patterns),
            opportunities: self.identify_opportunities(&patterns),
            recommendations: self.generate_recommendations(&prediction),
        })
    }
    
    async fn predict_pattern_outcome(&self, pattern: &Pattern) -> Result<PatternOutcome> {
        // Predict success probability of detected patterns
        let historical_outcomes = self.get_pattern_history(pattern).await?;
        let success_rate = self.calculate_pattern_success_rate(&historical_outcomes);
        
        Ok(PatternOutcome {
            pattern: pattern.clone(),
            success_probability: success_rate,
            expected_return: self.calculate_expected_return(&historical_outcomes),
            risk_level: self.assess_pattern_risk(&historical_outcomes),
        })
    }
}
```

#### 7.7 Add Batch Processing Optimization

**File**: `backend/src/service/batch_optimizer.rs`

**Purpose**: Efficient batch operations for vectorization and AI requests

```rust
pub struct BatchProcessor {
    batch_size: usize,
    max_wait_time: Duration,
    processing_queue: Arc<Mutex<VecDeque<BatchTask>>>,
    worker_pool: Arc<ThreadPool>,
}

pub struct BatchTask {
    task_id: String,
    user_id: String,
    task_type: BatchTaskType,
    payload: TaskPayload,
    priority: Priority,
    created_at: DateTime<Utc>,
}

pub enum BatchTaskType {
    Vectorization,
    EmbeddingGeneration,
    InsightGeneration,
    AnalyticsCalculation,
}
```

**Optimization Strategies**:

```rust
impl BatchProcessor {
    async fn optimize_and_process(&self, tasks: Vec<BatchTask>) -> Result<Vec<TaskResult>> {
        // Group by user for cache efficiency
        let by_user = self.group_by_user(tasks);
        
        // Group by type for API batching
        let by_type = self.group_by_type(by_user);
        
        // Sort by priority
        let prioritized = self.prioritize_tasks(by_type);
        
        // Process in optimized batches
        let results = self.process_batches(prioritized).await?;
        
        Ok(results)
    }
    
    async fn batch_vectorize(&self, tasks: Vec<VectorizationTask>) -> Result<Vec<VectorResult>> {
        // Combine similar content for efficient API calls
        let grouped = self.group_similar_content(&tasks);
        
        // Generate embeddings in parallel batches
        let batch_size = 10; // Voyager API limit
        let batches: Vec<_> = grouped.chunks(batch_size).collect();
        
        let futures = batches.into_iter().map(|batch| {
            self.voyager_client.batch_embed(batch)
        });
        
        let results = futures::future::join_all(futures).await;
        
        // Store in vector database efficiently
        self.batch_upsert_vectors(results.into_iter().flatten().collect()).await?;
        
        Ok(results.into_iter().flatten().collect())
    }
}
```

#### 7.8 Add Comprehensive Metrics

**File**: `backend/src/service/ai_metrics.rs`

**Purpose**: Track and monitor all AI system metrics

```rust
pub struct AIMetricsCollector {
    vectorization_metrics: Arc<Mutex<VectorizationMetrics>>,
    ai_service_metrics: Arc<Mutex<AIServiceMetrics>>,
    performance_metrics: Arc<Mutex<PerformanceMetrics>>,
    cost_metrics: Arc<Mutex<CostMetrics>>,
}

#[derive(Serialize, Clone)]
pub struct VectorizationMetrics {
    total_vectors: u64,
    vectors_per_second: f64,
    avg_embedding_time: Duration,
    cache_hit_rate: f64,
    error_rate: f64,
    batch_efficiency: f64,
    storage_used_mb: f64,
}

#[derive(Serialize, Clone)]
pub struct AIServiceMetrics {
    chat_requests: u64,
    chat_avg_response_time: Duration,
    chat_error_rate: f64,
    
    insights_generated: u64,
    insights_avg_time: Duration,
    insights_cache_hit_rate: f64,
    
    reports_generated: u64,
    reports_avg_time: Duration,
    
    context_retrieval_time: Duration,
    gemini_api_calls: u64,
    gemini_tokens_used: u64,
    gemini_errors: u64,
}

#[derive(Serialize, Clone)]
pub struct PerformanceMetrics {
    p50_latency: Duration,
    p95_latency: Duration,
    p99_latency: Duration,
    throughput_rps: f64,
    concurrent_requests: u32,
    queue_depth: u32,
}

#[derive(Serialize, Clone)]
pub struct CostMetrics {
    voyager_api_cost: f64,
    gemini_api_cost: f64,
    upstash_vector_cost: f64,
    total_cost: f64,
    cost_per_user: f64,
    cost_per_request: f64,
}
```

**Metrics Collection**:

```rust
impl AIMetricsCollector {
    pub fn record_vectorization(&self, duration: Duration, success: bool) {
        let mut metrics = self.vectorization_metrics.lock().unwrap();
        metrics.total_vectors += if success { 1 } else { 0 };
        metrics.avg_embedding_time = self.calculate_moving_avg(
            metrics.avg_embedding_time,
            duration,
            metrics.total_vectors,
        );
        if !success {
            metrics.error_rate = self.calculate_error_rate(metrics.total_vectors);
        }
    }
    
    pub fn record_ai_request(&self, request_type: AIRequestType, duration: Duration, success: bool) {
        let mut metrics = self.ai_service_metrics.lock().unwrap();
        match request_type {
            AIRequestType::Chat => {
                metrics.chat_requests += 1;
                metrics.chat_avg_response_time = self.calculate_moving_avg(
                    metrics.chat_avg_response_time,
                    duration,
                    metrics.chat_requests,
                );
            }
            AIRequestType::Insight => {
                metrics.insights_generated += 1;
                metrics.insights_avg_time = self.calculate_moving_avg(
                    metrics.insights_avg_time,
                    duration,
                    metrics.insights_generated,
                );
            }
            AIRequestType::Report => {
                metrics.reports_generated += 1;
                metrics.reports_avg_time = self.calculate_moving_avg(
                    metrics.reports_avg_time,
                    duration,
                    metrics.reports_generated,
                );
            }
        }
    }
    
    pub async fn get_metrics_snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            vectorization: self.vectorization_metrics.lock().unwrap().clone(),
            ai_services: self.ai_service_metrics.lock().unwrap().clone(),
            performance: self.performance_metrics.lock().unwrap().clone(),
            cost: self.cost_metrics.lock().unwrap().clone(),
            timestamp: Utc::now(),
        }
    }
}
```

**Metrics Endpoint**:

```rust
// In routes
pub async fn get_ai_metrics(
    metrics_collector: web::Data<Arc<AIMetricsCollector>>,
) -> Result<HttpResponse> {
    let snapshot = metrics_collector.get_metrics_snapshot().await;
    Ok(HttpResponse::Ok().json(ApiResponse::success(snapshot)))
}
```

#### 7.9 Add Health Checks

**File**: `backend/src/service/ai_health_checker.rs`

**Purpose**: Monitor health of all AI services

```rust
pub struct AIHealthChecker {
    voyager_client: Arc<VoyagerClient>,
    upstash_vector: Arc<UpstashVectorClient>,
    gemini_client: Arc<GeminiClient>,
    cache_service: Arc<AICacheService>,
    last_check: Arc<Mutex<DateTime<Utc>>>,
    health_status: Arc<Mutex<HealthStatus>>,
}

#[derive(Serialize, Clone)]
pub struct HealthStatus {
    overall: ServiceHealth,
    voyager: ServiceHealth,
    upstash_vector: ServiceHealth,
    gemini: ServiceHealth,
    cache: ServiceHealth,
    database: ServiceHealth,
    last_checked: DateTime<Utc>,
    uptime: Duration,
}

#[derive(Serialize, Clone, PartialEq)]
pub enum ServiceHealth {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

pub struct HealthCheckResult {
    service_name: String,
    health: ServiceHealth,
    response_time: Duration,
    error_message: Option<String>,
    metadata: HashMap<String, String>,
}
```

**Health Check Implementation**:

```rust
impl AIHealthChecker {
    pub async fn check_all_services(&self) -> Result<HealthStatus> {
        let start = Instant::now();
        
        // Run health checks in parallel
        let (voyager, upstash, gemini, cache, database) = tokio::join!(
            self.check_voyager_health(),
            self.check_upstash_health(),
            self.check_gemini_health(),
            self.check_cache_health(),
            self.check_database_health()
        );
        
        let overall = self.calculate_overall_health(&[
            &voyager, &upstash, &gemini, &cache, &database
        ]);
        
        let status = HealthStatus {
            overall,
            voyager: voyager.health,
            upstash_vector: upstash.health,
            gemini: gemini.health,
            cache: cache.health,
            database: database.health,
            last_checked: Utc::now(),
            uptime: start.elapsed(),
        };
        
        // Cache the status
        *self.health_status.lock().unwrap() = status.clone();
        *self.last_check.lock().unwrap() = Utc::now();
        
        Ok(status)
    }
    
    async fn check_voyager_health(&self) -> HealthCheckResult {
        let start = Instant::now();
        
        match self.voyager_client.health_check().await {
            Ok(_) => HealthCheckResult {
                service_name: "Voyager API".to_string(),
                health: ServiceHealth::Healthy,
                response_time: start.elapsed(),
                error_message: None,
                metadata: HashMap::new(),
            },
            Err(e) => HealthCheckResult {
                service_name: "Voyager API".to_string(),
                health: ServiceHealth::Unhealthy,
                response_time: start.elapsed(),
                error_message: Some(e.to_string()),
                metadata: HashMap::new(),
            }
        }
    }
    
    async fn check_gemini_health(&self) -> HealthCheckResult {
        let start = Instant::now();
        
        // Test with minimal request
        match self.gemini_client.test_connection().await {
            Ok(_) => HealthCheckResult {
                service_name: "Gemini API".to_string(),
                health: ServiceHealth::Healthy,
                response_time: start.elapsed(),
                error_message: None,
                metadata: HashMap::new(),
            },
            Err(e) => {
                // Check if degraded (high latency) vs unhealthy (error)
                let health = if start.elapsed() > Duration::from_secs(5) {
                    ServiceHealth::Degraded
                } else {
                    ServiceHealth::Unhealthy
                };
                
                HealthCheckResult {
                    service_name: "Gemini API".to_string(),
                    health,
                    response_time: start.elapsed(),
                    error_message: Some(e.to_string()),
                    metadata: HashMap::new(),
                }
            }
        }
    }
    
    async fn check_upstash_health(&self) -> HealthCheckResult {
        let start = Instant::now();
        
        match self.upstash_vector.ping().await {
            Ok(latency) => {
                let health = if latency > Duration::from_millis(200) {
                    ServiceHealth::Degraded
                } else {
                    ServiceHealth::Healthy
                };
                
                HealthCheckResult {
                    service_name: "Upstash Vector".to_string(),
                    health,
                    response_time: start.elapsed(),
                    error_message: None,
                    metadata: [("latency_ms".to_string(), latency.as_millis().to_string())]
                        .into_iter().collect(),
                }
            },
            Err(e) => HealthCheckResult {
                service_name: "Upstash Vector".to_string(),
                health: ServiceHealth::Unhealthy,
                response_time: start.elapsed(),
                error_message: Some(e.to_string()),
                metadata: HashMap::new(),
            }
        }
    }
    
    fn calculate_overall_health(&self, results: &[&HealthCheckResult]) -> ServiceHealth {
        if results.iter().all(|r| r.health == ServiceHealth::Healthy) {
            ServiceHealth::Healthy
        } else if results.iter().any(|r| r.health == ServiceHealth::Unhealthy) {
            ServiceHealth::Unhealthy
        } else {
            ServiceHealth::Degraded
        }
    }
    
    pub async fn get_cached_status(&self) -> HealthStatus {
        self.health_status.lock().unwrap().clone()
    }
}
```

**Health Check Routes**:

```rust
// In routes
pub async fn health_check(
    health_checker: web::Data<Arc<AIHealthChecker>>,
) -> Result<HttpResponse> {
    let status = health_checker.check_all_services().await?;
    
    let http_status = match status.overall {
        ServiceHealth::Healthy => StatusCode::OK,
        ServiceHealth::Degraded => StatusCode::OK,
        ServiceHealth::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
        ServiceHealth::Unknown => StatusCode::SERVICE_UNAVAILABLE,
    };
    
    Ok(HttpResponse::build(http_status).json(ApiResponse::success(status)))
}

pub async fn health_check_cached(
    health_checker: web::Data<Arc<AIHealthChecker>>,
) -> Result<HttpResponse> {
    let status = health_checker.get_cached_status().await;
    Ok(HttpResponse::Ok().json(ApiResponse::success(status)))
}
```

**Periodic Health Checks**:

```rust
// In main.rs initialization
async fn run_periodic_health_checks(health_checker: Arc<AIHealthChecker>) {
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    
    loop {
        interval.tick().await;
        
        match health_checker.check_all_services().await {
            Ok(status) => {
                log::info!("AI Services Health: {:?}", status.overall);
                
                if status.overall == ServiceHealth::Unhealthy {
                    log::error!("AI Services unhealthy: {:?}", status);
                    // Trigger alerts, notifications, etc.
                }
            }
            Err(e) => {
                log::error!("Health check failed: {}", e);
            }
        }
    }
}
```

### Phase 8: Testing & Error Handling

#### 8.1 Unit Tests

- Test vector generation for each data type
- Test similarity search accuracy
- Test AI response generation
- Test report compilation
- Test smart chunking algorithm
- Test cache eviction policies
- Test predictive model accuracy

#### 8.2 Integration Tests

- End-to-end chat flow
- Insights generation pipeline
- Report generation pipeline
- Vector update triggers
- Multi-level cache behavior
- Batch processing efficiency
- Health check responses

#### 8.3 Performance Testing

- Load testing (1000+ concurrent users)
- Vector search latency under load
- Cache hit rate analysis
- API rate limit handling
- Memory usage optimization
- Database query optimization

#### 8.4 Error Handling & Resilience

- API failure fallbacks
- Retry logic with exponential backoff
- Circuit breaker activation/recovery
- Graceful degradation scenarios
- Comprehensive error logging
- Alert triggering for critical failures

## Key Implementation Notes

### Vector Namespace Strategy

Each user gets their own namespace in Upstash Vector:

- Format: `user_{user_id}`
- Isolates user data
- Enables efficient queries per user

### Context Retrieval Strategy

For AI Chat:

1. Convert user message to embedding
2. Query Upstash Vector for similar content (top 5-10)
3. Retrieve full data for matched vectors from Turso
4. Build context string from retrieved data
5. Send context + message to Gemini

### Data Update Flow

**Real-time** (Critical):

- Trade creation/update
- Trade notes
- Major playbook changes

**Batch** (Non-critical):

- Notebook entries
- Bulk imports
- Historical data updates

### Cost Optimization

- Cache embeddings for unchanged data
- Use Gemini Flash for quick insights
- Use Gemini Pro for comprehensive reports
- Batch embedding generation when possible
- Monitor Upstash Vector storage usage

## Files to Create/Modify

### New Files (20+)

1. `backend/src/turso/vector_config.rs`
2. `backend/src/service/voyager_client.rs`
3. `backend/src/service/upstash_vector_client.rs`
4. `backend/src/service/gemini_client.rs`
5. `backend/src/service/vectorization_service.rs`
6. `backend/src/service/data_formatter.rs`
7. `backend/src/service/batch_vectorization.rs`
8. `backend/src/service/ai_chat_service.rs`
9. `backend/src/service/ai_insights_service.rs`
10. `backend/src/service/ai_reports_service.rs`
11. `backend/src/service/scheduled_insights.rs`
12. `backend/src/models/ai/mod.rs`
13. `backend/src/models/ai/chat.rs`
14. `backend/src/models/ai/insights.rs`
15. `backend/src/models/ai/reports.rs`
16. `backend/src/routes/ai_chat.rs`
17. `backend/src/routes/ai_insights.rs`
18. `backend/src/routes/ai_reports.rs`
19. `backend/database/08_ai_chat/01_chat_tables.sql`
20. `backend/database/08_ai_chat/02_insights_tables.sql`
21. `backend/database/08_ai_chat/03_reports_tables.sql`

### Modified Files (7+)

1. `backend/Cargo.toml` - Add dependencies
2. `backend/src/main.rs` - Initialize services, register routes
3. `backend/src/turso/mod.rs` - Update AppState
4. `backend/src/routes/stocks.rs` - Add vectorization hooks
5. `backend/src/routes/options.rs` - Add vectorization hooks
6. `backend/src/routes/trade_notes.rs` - Add vectorization hooks
7. `backend/src/routes/notebook.rs` - Add vectorization hooks
8. `backend/src/routes/playbook.rs` - Add vectorization hooks
9. `backend/src/service/mod.rs` - Export new services
10. `backend/src/models/mod.rs` - Export AI models

## Success Metrics

- ✓ All trading data vectorized within 5 minutes
- ✓ Chat response time < 3 seconds
- ✓ Insights generation < 30 seconds
- ✓ Report generation < 60 seconds
- ✓ Vector similarity search < 100ms
- ✓ 99.9% uptime for AI services

### To-dos

- [ ] Set up infrastructure: Add dependencies, create configs, and initialize clients for Voyager, Upstash Vector, and Gemini
- [ ] Build vectorization system: Data formatters, vectorization service, and metadata schema
- [ ] Implement real-time vectorization hooks in all CRUD routes (stocks, options, notes, notebook, playbook)
- [ ] Implement batch vectorization cron job for non-critical data updates
- [ ] Create AI chat models, database schema, and table migrations
- [ ] Build AI chat service with context retrieval, Gemini integration, and conversation storage
- [ ] Create chat API routes and integrate with main application
- [ ] Create AI insights models, database schema, and table migrations
- [ ] Build AI insights service with analytics integration and pattern analysis
- [ ] Create insights API routes and integrate with main application
- [ ] Implement scheduled insights generation background job
- [ ] Create AI reports models, database schema, and table migrations
- [ ] Build AI reports service with data aggregation and comprehensive analysis
- [ ] Create reports API routes and integrate with main application
- [ ] Write integration tests for all AI services and vectorization flows
- [ ] Implement caching, optimize performance, add monitoring and error handling