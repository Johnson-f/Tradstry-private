<!-- 7f3e0095-e043-467c-93cd-17a66a817ca8 a88e5259-befe-40b3-a13a-79e81729219f -->
# Tool Engine System Implementation

## Overview

Create a hybrid tool selection and execution system that enhances the AI chat service with dynamic tool calling. The system uses rule-based pattern matching for simple queries and AI-driven function calling for complex cases.

## Architecture

### 1. Tool Engine Module Structure

Create `backend/src/service/ai_service/tool_engine/` with:

- `mod.rs` - Module exports and tool registry
- `tool.rs` - Base tool trait and tool result types
- `tool_selector.rs` - Hybrid selection logic (rule-based + AI-driven)
- `market_tool.rs` - Market data fetching tool
- `trade_tool.rs` - User trade querying tool
- `vector_tool.rs` - Wrapper for existing HybridSearchService
- `executor.rs` - Tool execution orchestrator

### 2. Tool Definitions

#### MarketTool

- **Purpose**: Fetch real-time stock data
- **Input**: Stock symbols (extracted from query)
- **Output**: Quote data (price, change, volume, etc.)
- **Source**: Uses existing `MarketClient` and `quotes::get_quotes()`

#### TradeTool  

- **Purpose**: Query user's trading history
- **Input**: Filters (time_range, status, symbol, etc.)
- **Output**: Stock trades and option trades
- **Source**: Uses existing `Stock::find_all()` and `OptionTrade::find_all()` models

#### VectorTool

- **Purpose**: Retrieve relevant context from vector database
- **Input**: Query text, max_results
- **Output**: Context sources (already implemented in HybridSearchService)
- **Source**: Wraps existing `HybridSearchService`

### 3. Tool Selection Logic

#### Rule-Based Detection (`tool_selector.rs`)

Pattern matching for:

- **Stock symbols**: Regex `\b[A-Z]{1,5}\b` (uppercase 1-5 letter tickers)
- **Trade keywords**: "my trades", "winning", "losing", "portfolio", "positions"
- **Time references**: "this week", "last month", "today", "yesterday"
- **Market keywords**: "price", "quote", "stock", "market data"

#### AI-Driven Selection

- Use OpenRouter function calling API
- Define tool schemas in OpenRouter format
- LLM analyzes query and returns tool calls with parameters
- Fallback to rule-based if AI selection fails

### 4. Integration Points

#### Pre-Fetch Phase (in `AIChatService.generate_response()`)

```rust
// Before context retrieval
let pre_fetched_tools = tool_engine.select_and_execute_prefetch(user_id, &query).await?;
// Include pre-fetched results in initial context
```

#### During Generation (OpenRouter Function Calling)

- Extend `OpenRouterClient` to support function calling
- Add `tools` parameter to `ChatRequest`
- Handle `tool_calls` in response
- Execute tools and send results back to LLM

#### Post-Execution

- Format tool results as context
- Merge with existing context sources
- Include in system prompt

### 5. File Changes

#### New Files

- `backend/src/service/ai_service/tool_engine/mod.rs`
- `backend/src/service/ai_service/tool_engine/tool.rs`
- `backend/src/service/ai_service/tool_engine/tool_selector.rs`
- `backend/src/service/ai_service/tool_engine/market_tool.rs`
- `backend/src/service/ai_service/tool_engine/trade_tool.rs`
- `backend/src/service/ai_service/tool_engine/vector_tool.rs`
- `backend/src/service/ai_service/tool_engine/executor.rs`

#### Modified Files

- `backend/src/service/ai_service/mod.rs` - Export tool_engine module
- `backend/src/service/ai_service/chat_service.rs` - Integrate tool_engine
- `backend/src/service/ai_service/openrouter_client.rs` - Add function calling support

### 6. Implementation Details

#### Tool Trait

```rust
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> serde_json::Value;
    async fn execute(&self, params: serde_json::Value, context: ToolContext) -> Result<ToolResult>;
}
```

#### ToolContext

Contains user_id, database connection, and service clients needed for tool execution.

#### ToolResult

Standardized format for tool outputs that can be formatted for LLM context.

### 7. Example Flow

**Query**: "What's the price of TSLA and show me my winning trades this week"

1. **Pre-fetch phase**:

   - Rule-based detects: "TSLA" (symbol) + "winning trades" + "this week"
   - Executes: MarketTool("TSLA") + TradeTool(time_range: this_week, filter: winning)
   - Results formatted and added to context

2. **AI generation**:

   - Enhanced prompt includes pre-fetched data
   - LLM can request additional tools via function calling if needed
   - Final response incorporates all tool results

### 8. Configuration

Add tool engine config to `AIConfig`:

- Enable/disable pre-fetch
- Enable/disable AI-driven selection
- Tool execution timeout
- Max tools per query

### To-dos

- [ ] Create tool_engine module structure with mod.rs, tool.rs, and base types (Tool trait, ToolResult, ToolContext)
- [ ] Implement MarketTool that uses MarketClient to fetch stock quotes based on symbols extracted from query
- [ ] Implement TradeTool that queries user trades from database using Stock and OptionTrade models with filters (time_range, status, symbol)
- [ ] Implement VectorTool wrapper around existing HybridSearchService for context retrieval
- [ ] Implement hybrid tool selector with rule-based pattern matching (stock symbols, keywords, time references) and AI-driven selection using OpenRouter function calling
- [ ] Implement ToolExecutor that orchestrates tool selection, execution, and result formatting
- [ ] Extend OpenRouterClient to support function calling API (add tools parameter, handle tool_calls in response)
- [ ] Integrate tool_engine into AIChatService: add pre-fetch phase before context retrieval, support function calling during generation, format tool results in context
- [ ] Update ai_service/mod.rs to export tool_engine module and add ToolEngine to AppState dependencies