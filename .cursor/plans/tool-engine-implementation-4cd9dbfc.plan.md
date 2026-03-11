<!-- 4cd9dbfc-971c-4449-8b21-f4e79847e911 6c44fd2a-acc7-4731-accf-2127cfbb230f -->
# Tool Engine Implementation Plan

## Architecture Overview

The tool engine will be a new service module that:

1. Defines tools that wrap existing `market_engine` functions
2. Adds function calling support to the OpenRouter client
3. Implements an agentic loop in `AIChatService` that executes tools and returns final responses

## Phase 1: Add Tool Calling to OpenRouter Client

Modify [backend/src/service/ai_service/model_connection/openrouter.rs](backend/src/service/ai_service/model_connection/openrouter.rs) to support function calling:

- Add `tools` field to `ChatRequest` struct (OpenRouter uses OpenAI-compatible format)
- Add `tool_calls` field to response parsing (`Choice`, `StreamChoice`)
- Add `ToolCall`, `ToolFunction`, `ToolDefinition` structs
- Create new method `generate_chat_with_tools()` that accepts tool definitions

Key structures to add:

```rust
pub struct ToolDefinition {
    pub r#type: String,  // "function"
    pub function: FunctionDefinition,
}

pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

pub struct ToolCall {
    pub id: String,
    pub r#type: String,
    pub function: FunctionCall,
}
```

## Phase 2: Create Tool Engine Service

Create new module at `backend/src/service/tool_engine/`:

**Files to create:**

- `mod.rs` - Module exports
- `definitions.rs` - Tool definitions for market_engine functions
- `executor.rs` - Tool dispatcher that routes calls to market_engine

**Tool definitions to implement (wrapping existing market_engine):**

| Tool Name | Market Engine Function | Description |

|-----------|----------------------|-------------|

| `get_stock_quote` | `quotes::get_quotes` | Get real-time stock price and details |

| `get_financials` | `financials::get_financials` | Get income statement, balance sheet, cash flow |

| `get_market_news` | `news::get_news` | Get market news by symbol |

| `get_earnings_transcript` | `earnings_transcripts::get_earnings_transcript` | Get earnings call transcripts |

| `search_symbol` | `search::search_symbols` | Search for stock symbols |

| `get_historical_data` | `historical::get_historical` | Get historical price data |

**Executor pattern:**

```rust
pub struct ToolEngine {
    market_client: MarketClient,
}

impl ToolEngine {
    pub fn get_tool_definitions() -> Vec<ToolDefinition>;
    pub async fn execute(&self, tool_call: ToolCall) -> Result<serde_json::Value>;
}
```

## Phase 3: Implement Agentic Loop in Chat Service

Modify [backend/src/service/ai_service/interface/chat_service.rs](backend/src/service/ai_service/interface/chat_service.rs):

- Add `ToolEngine` to `AIChatService` struct
- Create new method `generate_response_with_tools()` that:

  1. Sends user message to LLM with tool definitions
  2. If LLM returns `tool_calls`, execute each via `ToolEngine`
  3. Append tool results to conversation
  4. Call LLM again for final response
  5. Repeat until LLM returns content without tool calls (max 5 iterations)

**Flow:**

```
User Message -> LLM (with tools) 
  -> tool_calls? -> Execute via ToolEngine -> Tool Results -> LLM
  -> tool_calls? -> Execute again -> LLM  
  -> Final text response -> User
```

## Phase 4: Wire Up and Expose

- Add `ToolEngine` initialization to `AppState` in [backend/src/turso/mod.rs](backend/src/turso/mod.rs)
- Update chat routes to use tool-enabled generation when appropriate
- Consider adding a `use_tools: bool` flag to `ChatRequest` to toggle tool use

## Files to Modify

| File | Changes |

|------|---------|

| `backend/src/service/ai_service/model_connection/openrouter.rs` | Add tool calling structs and methods |

| `backend/src/service/ai_service/interface/chat_service.rs` | Add agentic loop logic |

| `backend/src/turso/mod.rs` | Initialize ToolEngine in AppState |

| `backend/src/service/mod.rs` | Export new tool_engine module |

## Files to Create

| File | Purpose |

|------|---------|

| `backend/src/service/tool_engine/mod.rs` | Module definition |

| `backend/src/service/tool_engine/definitions.rs` | Tool schemas for LLM |

| `backend/src/service/tool_engine/executor.rs` | Tool execution logic |

### To-dos

- [ ] Add tool/function calling support to OpenRouter client
- [ ] Create tool definitions wrapping market_engine functions
- [ ] Build tool executor that dispatches calls to market_engine
- [ ] Implement agentic loop in AIChatService for tool execution
- [ ] Wire ToolEngine to AppState and update chat routes