1. Chat Flow (AIChatService)
User Message
    │
    ├─► Get/Create Session (Turso DB)
    │
    ├─► Retrieve Context (HybridSearchService)
    │   ├─► Generate query embedding (VoyagerClient)
    │   ├─► Vector similarity search (UpstashVectorClient)
    │   ├─► Keyword search (QdrantDocumentClient)
    │   └─► Merge & rerank results
    │
    ├─► Build Enhanced Messages
    │   ├─► System prompt with context
    │   ├─► Conversation history
    │   └─► Current user message
    │
    ├─► Generate AI Response (OpenRouterClient)
    │
    ├─► Store Messages (Turso DB)
    │
    └─► Vectorize Messages (VectorizationService)
        ├─► Generate embeddings (VoyagerClient)
        └─► Store in Upstash Vector DB

2. Insight Flow (AIInsightsService)
Insight Request
    │
    ├─► Check for existing insight (Turso DB)
    │
    ├─► Retrieve Trading Data
    │   ├─► Query vectors by insight type
    │   └─► Calculate data quality score
    │
    ├─► Generate Insight Content
    │   ├─► Build prompt with trading data
    │   ├─► Call OpenRouterClient
    │   └─► Parse structured response
    │
    ├─► Store Insight (Turso DB)
    │
    └─► Return Insight with metadata

3. Vectorization Flow (VectorizationService)
Content to Vectorize
    │
    ├─► Validate & Hash Content
    │
    ├─► Generate Embedding (VoyagerClient)
    │   └─► Convert text → vector (1536 dimensions)
    │
    ├─► Create Metadata
    │   ├─► User ID, Data Type, Entity ID
    │   ├─► Tags, Timestamp
    │   └─► Content hash
    │
    ├─► Store Vector (UpstashVectorClient)
    │   └─► User-specific namespace
    │
    └─► Store Search Document (QdrantDocumentClient)
        └─► For keyword search capability



Flow 
1. User Query → AIChatService.generate_response()
2. Query Embedding → VoyagerClient.embed_text("What were my best trades?")
3. Vector Search → UpstashVectorClient.query_similarity() 
   → Finds relevant trade notes, stocks, options
4. Keyword Search → QdrantDocumentClient.search_by_keyword()
   → Finds exact matches for "best trades"
5. Merge Results → HybridSearchService merges and reranks
6. Build Prompt → Adds context: "Based on your trade history: [retrieved trades]"
7. AI Generation → OpenRouterClient generates response
8. Store & Vectorize → Saves response and indexes it for future queries