# Backend Development Guide

This guide provides a detailed overview of the backend project structure and instructions for setting up the development environment for the Tradstry platform.

## Project Structure

The backend is primarily built with **Rust** using the Actix Web framework, with a supporting **Go** microservice for brokerage integrations.

### Root Directory
- **`Cargo.toml` / `Cargo.lock`**: Rust project manifest and lock file.
- **`src/`**: Main Rust source code.
- **`snaptrade-service/`**: Source code for the Go microservice handling SnapTrade API interactions.
- **`database/`**: SQL scripts for database initialization, schema definitions, and migrations.
- **`docs/`**: Documentation folder.
- **`build-docs/`**: Build and deployment documentation (legacy/reference).
- **`.env.example`**: Template for environment variables.
- **`docker-compose.yml`**: Docker composition for running the backend and dependent services.

### Source Code (`src/`)
- **`main.rs`**: Application entry point. Sets up the HTTP server, middleware, routes, and dependency injection.
- **`models/`**: Data structures and database interaction logic. Organized by domain:
  - `stock/`: Stock trade models.
  - `options/`: Option trade models.
  - `notebook/`: Journaling and notes models.
  - `analytics/`: Structures for trade analysis.
  - `alerts/`: Alert rule definitions.
- **`routes/`**: HTTP API endpoint handlers. Maps URLs to service functions.
- **`service/`**: Core business logic layer.
    - `ai_service/`: Integration with LLMs (OpenRouter, Gemini) and Vector DBs (Qdrant, Upstash).
    - `analytics_engine/`: Logic for calculating trading metrics (PnL, win rate, Sharpe ratio, etc.).
    - `brokerage/`: Logic for brokerage connections and synchronization.
    - `market_engine/`: Fetching and processing market data (Yahoo Finance, etc.).
    - `notebook_engine/`: Logic for the trading journal, calendar, and collaboration features.
    - `notifications/`: Push notifications and alert dispatching logic.
    - `tool_engine/`: AI tool definitions and execution logic.
- **`turso/`**: Infrastructure layer for Database (Turso/libSQL), Redis, and Authentication.
- **`websocket/`**: WebSocket handling for real-time updates and collaboration.
- **`middleware/`**: Actix Web middleware (e.g., Rate Limiting).

## Prerequisites

- **Rust**: Latest stable version (install via [rustup](https://rustup.rs/)).
- **Go**: Version 1.24+ (required only if running `snaptrade-service` locally from source).
- **Docker**: Recommended for running dependencies locally.
- **Turso Account**: For the database.
- **Supabase Account**: For authentication.

## Local Development Setup

### 1. Clone and Install Dependencies

```bash
# Clone the repository
git clone <repo-url>
cd Tradstry/backend

# Install Rust dependencies
cargo build
```

### 2. Environment Configuration

Create a `.env` file in the `backend/` root directory. Copy the example as a starting point:

```bash
cp .env.example .env
```

**Critical Variables to Configure:**
- **Database**: `REGISTRY_DB_URL` and `REGISTRY_DB_TOKEN` (Connection to the main Turso registry DB).
- **Auth**: `SUPABASE_URL` and `SUPABASE_ANON_KEY` (For validating JWTs).
- **AI**: `OPENROUTER_API_KEY` or `GEMINI_API_KEY` (For AI analysis features).
- **Cache**: `UPSTASH_REDIS_REST_URL` and `UPSTASH_REDIS_REST_TOKEN` (For caching and rate limiting).
- **Vector DB**: `QDRANT_URL` and `QDRANT_API_KEY` (For semantic search).

### 3. Database Setup

The application uses a **multi-tenant architecture**:
1. **Registry DB**: A central database stores the mapping of Users to their specific Database URLs.
2. **User DBs**: Each user gets their own isolated Turso database.

When running locally, ensure you have a Registry DB set up. The schemas for user databases are located in `database/02_users_schema/`. The backend handles creating user databases automatically upon initialization.

### 4. Running the Backend

**Standard Rust Run:**
```bash
# Set environment to development
export RUST_ENV=development

# Run the server
cargo run
```
The server will start on port `9000` (or as defined in `PORT`).

**With Hot-Reloading (Recommended):**
Install `cargo-watch`:
```bash
cargo install cargo-watch
cargo watch -x run
```

### 5. Working with the SnapTrade Service (Go)

The Rust backend communicates with the `snaptrade-service` for brokerage integrations.

**Option A: Docker (Recommended)**
Use docker-compose to run the microservice alongside the backend (or just the service).
```bash
docker-compose up -d snaptrade-service
```

**Option B: Run Locally**
```bash
cd snaptrade-service
cp .env.example .env # Configure SnapTrade credentials
go run main.go
```
Ensure `SNAPTRADE_SERVICE_URL` in the backend `.env` points to your local Go service (e.g., `http://localhost:8080`).

## Architecture Highlights

- **Authentication**: Authentication is handled by Supabase. The frontend sends a JWT in the `Authorization` header. The backend `turso/auth.rs` module validates this token against Supabase.
- **Real-time Updates**: The `websocket/` module manages connections. Clients subscribe to specific data streams (quotes, trade updates).
- **AI Integration**: The `ai_service` uses an "Agentic" approach where the LLM can call defined tools (in `tool_engine/`) to fetch market data before answering user queries.

## Testing

Run unit and integration tests:

```bash
cargo test
```
