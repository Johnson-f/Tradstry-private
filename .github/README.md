# Tradstry - AI-Powered Trading Journal & Analytics Platform

Tradstry is a comprehensive trading journal and analytics platform that helps traders track, analyze, and improve their trading performance using AI-powered insights and real-time analytics.

## Overview

Tradstry combines advanced journaling capabilities with sophisticated analytics to transform how traders make decisions. The platform integrates with brokerage accounts, provides real-time market data, and uses AI to generate personalized trading insights and reports.

### Key Features

- **Real-time Analytics**: Comprehensive performance tracking with risk metrics, P&L analysis, and market correlation insights
- **AI-Powered Insights**: Automated behavioral analysis, pattern recognition, and personalized recommendations
- **Advanced Journaling**: Rich text notes, trade tagging, playbook creation, and multimedia support
- **Brokerage Integration**: Direct connection to trading accounts for automatic trade importing
- **Market Data**: Live quotes, historical data, technical indicators, and news aggregation
- **Calendar Integration**: Sync with Google Calendar for trading events and reminders
- **AI Chat**: Interactive AI assistant for trading analysis and strategy discussions
- **Responsive Design**: Full-featured web application with mobile support

## Tech Stack

### Frontend
- **Framework**: Next.js 16 with React 19
- **Language**: TypeScript
- **Styling**: Tailwind CSS with custom components
- **UI Library**: Radix UI primitives with custom design system
- **State Management**: Zustand
- **Data Fetching**: TanStack Query (React Query)
- **Real-time**: WebSocket connections
- **Forms**: React Hook Form with Zod validation
- **Package Manager**: Bun

### Backend
- **Language**: Rust with Axum web framework
- **Database**: Turso (SQLite-compatible) with Drizzle ORM
- **Cache**: Redis (Upstash)
- **Vector Search**: Qdrant for AI embeddings
- **Search**: Upstash for hybrid search
- **Authentication**: Supabase Auth with Google OAuth
- **Storage**: Supabase Storage for files and images

### AI & ML Services
- **LLM**: OpenRouter API with multiple model support
- **Embeddings**: Voyage AI for semantic search
- **Reranking**: Custom AI reranking for search results
- **Brokerage**: Snaptrade API for account integration

### Infrastructure
- **Deployment**: Docker with multi-stage builds
- **Reverse Proxy**: Nginx with security headers
- **Monitoring**: Health check endpoints
- **CDN**: Vercel for frontend hosting

## Architecture

The application follows a microservices architecture with clear separation of concerns:

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Next.js App   │────│   Rust Backend  │────│     Database     │
│                 │    │                 │    │   (Turso/SQL)    │
│ • Landing Pages │    │ • API Routes    │    │                 │
│ • Dashboard     │    │ • Auth Handling │    │ • User Data      │
│ • Analytics UI  │    │ • Business Logic│    │ • Trade Records  │
│ • Journaling    │    │ • External APIs │    └─────────────────┘
└─────────────────┘    └─────────────────┘            │
        │                      │                      │
        └──────────────────────┼──────────────────────┘
                               │
                    ┌─────────────────┐
                    │   AI Services   │
                    │                 │
                    │ • OpenRouter    │
                    │ • Voyage AI     │
                    │ • Qdrant        │
                    └─────────────────┘
```

### Key Components

#### Frontend (`/app`)
- **Landing Pages**: Marketing and onboarding (`/`)
- **Dashboard**: Main trading interface (`/dashboard`)
- **Analytics**: Performance metrics and charts (`/analytics`)
- **Journaling**: Trade notes and playbooks (`/journaling`)
- **Education**: Learning resources (`/education`)
- **API Routes**: Next.js API handlers (`/api`)

#### Backend (`/backend`)
- **REST API**: CRUD operations for all entities
- **External Integrations**: Brokerage, market data, AI services
- **Background Jobs**: Data synchronization and AI processing
- **Authentication**: JWT token management

#### Services (`/lib/services`)
- **API Client**: Centralized HTTP requests
- **Analytics Service**: Performance calculations
- **Brokerage Service**: Account synchronization
- **AI Services**: Chat, insights, and reports
- **Market Data**: Quotes and historical data
- **User Service**: Profile and initialization

## Local Development Setup

### Prerequisites

- **Node.js**: 18+ (with Bun package manager)
- **Rust**: 1.70+ with Cargo
- **Docker**: For backend and database services
- **Git**: For version control

### 1. Clone and Install

```bash
# Clone the repository
git clone <repository-url>
cd tradstry

# Install frontend dependencies
bun install

# Install backend dependencies
cd backend
cargo build
cd ..
```

### 2. Environment Configuration

#### Frontend Environment Variables

Create `.env.local` in the root directory:

```bash
# Copy from template
cp env-templates/frontend.env.production .env.local

# Required variables (get from your accounts):
NEXT_PUBLIC_SUPABASE_URL=your_supabase_url
NEXT_PUBLIC_SUPABASE_PUBLISHABLE_OR_ANON_KEY=your_supabase_anon_key
NEXT_PUBLIC_API_URL=http://localhost:8080
GOOGLE_CLIENT_ID=your_google_client_id
GOOGLE_CLIENT_SECRET=your_google_client_secret
```

#### Backend Environment Variables

Create `backend/.env`:

```bash
# Copy from template
cp env-templates/backend.env.production backend/.env

# Required variables (get from your accounts):
SUPABASE_URL=your_supabase_url
SUPABASE_ANON_KEY=your_supabase_anon_key
SUPABASE_SERVICE_ROLE_KEY=your_supabase_service_role_key
TURSO_DB_URL=your_turso_database_url
TURSO_API_TOKEN=your_turso_token
UPSTASH_REDIS_REST_URL=your_redis_url
UPSTASH_REDIS_REST_TOKEN=your_redis_token
OPENROUTER_API_KEY=your_openrouter_key
VOYAGER_API_KEY=your_voyage_key
UPSTASH_VECTOR_REST_URL=your_vector_url
UPSTASH_VECTOR_REST_TOKEN=your_vector_token
UPSTASH_SEARCH_REST_URL=your_search_url
UPSTASH_SEARCH_REST_TOKEN=your_search_token
QDRANT_URL=your_qdrant_url
QDRANT_API_KEY=your_qdrant_key
UPLOADCARE_PUBLIC_KEY=your_uploadcare_public_key
UPLOADCARE_SECRET_KEY=your_uploadcare_secret_key
```

### 3. Database Setup

```bash
# Start database services (if using Docker)
docker-compose -f backend/docker-compose.yml up -d

# Or use local Turso database
# Follow Turso documentation for local setup
```

### 4. Run Development Servers

#### Frontend (Terminal 1)
```bash
# Start Next.js development server
bun run dev

# Server will be available at http://localhost:3000
```

#### Backend (Terminal 2)
```bash
# Start Rust backend server
cd backend
cargo run

# API will be available at http://localhost:8080
```

#### Alternative: Docker Development
```bash
# Build and run with Docker Compose
docker-compose up --build

# Frontend: http://localhost:3000
# Backend: http://localhost:8080
```

### 5. Database Migration (if needed)

```bash
# Run database migrations
cd backend
cargo run --bin migrate
```

## Project Structure

```
tradstry/
├── app/                          # Next.js App Router
│   ├── (auth)/                   # Authentication pages
│   ├── (dashboard)/              # Protected dashboard routes
│   ├── (landing)/                # Public landing pages
│   ├── api/                      # API routes
│   ├── globals.css               # Global styles
│   └── layout.tsx                # Root layout
├── backend/                      # Rust backend
│   ├── src/
│   │   ├── main.rs              # Application entry point
│   │   ├── routes/              # API route handlers
│   │   ├── models/              # Database models
│   │   └── services/            # Business logic
│   ├── Cargo.toml               # Rust dependencies
│   └── docker-compose.yml       # Backend services
├── components/                   # React components
│   ├── ui/                      # Reusable UI components
│   ├── analytics/               # Analytics-specific components
│   ├── journaling/              # Journaling components
│   ├── brokerage/               # Brokerage integration
│   └── landing/                 # Landing page components
├── lib/                         # Shared utilities and services
│   ├── services/                # API service clients
│   ├── types/                   # TypeScript type definitions
│   ├── hooks/                   # Custom React hooks
│   ├── utils/                   # Utility functions
│   ├── supabase/                # Supabase configuration
│   └── websocket/               # WebSocket client
├── docs/                        # Documentation
├── env-templates/               # Environment variable templates
├── nginx/                       # Production proxy configuration
├── public/                      # Static assets
└── scripts/                     # Deployment and utility scripts
```

## Core Features

### Analytics Dashboard
- **Performance Metrics**: Win rate, profit factor, Sharpe ratio
- **Risk Analysis**: Maximum drawdown, volatility, correlation
- **Time Series**: Daily/weekly/monthly P&L charts
- **Trade Analysis**: Individual trade performance, tagging system

### AI-Powered Features
- **AI Chat**: Natural language trading assistant
- **Insights**: Automated pattern recognition and recommendations
- **Reports**: Comprehensive trading performance reports
- **Insights**: Behavioral analysis and market intelligence

### Advanced Journaling
- **Trade Notes**: Rich text editor with images and attachments
- **Playbooks**: Strategy templates and trade setups
- **Tags**: Customizable trade categorization
- **Search**: Full-text search with AI-powered relevance

### Brokerage Integration
- **Account Sync**: Automatic trade importing
- **Position Tracking**: Real-time portfolio monitoring
- **Transaction History**: Complete trading history import
- **Multi-Account**: Support for multiple brokerage accounts

### Market Data
- **Real-time Quotes**: Live price updates
- **Technical Indicators**: Moving averages, RSI, MACD
- **Historical Data**: Multi-timeframe data analysis
- **News Aggregation**: Market news and earnings reports

## Development Commands

```bash
# Frontend
bun run dev              # Start development server
bun run build            # Build for production
bun run start            # Start production server
bun run lint             # Run ESLint
bun run type-check       # Run TypeScript type checking
bun run format           # Format code with Prettier

# Backend
cd backend
cargo build              # Build Rust application
cargo run                # Run development server
cargo test               # Run tests
cargo clippy             # Run linter

# Docker
docker-compose up        # Start all services
docker-compose down      # Stop all services
docker-compose logs      # View logs
```

## Deployment

Tradstry supports multiple deployment strategies:

### Production Deployment
- **Frontend**: Vercel with Next.js standalone build
- **Backend**: Docker container on VPS
- **Database**: Turso cloud database
- **CDN**: Vercel edge network

### Development Deployment
- **Local**: Docker Compose for full stack
- **Staging**: Vercel preview deployments
- **CI/CD**: GitHub Actions with automated testing

For detailed deployment instructions, see:
- [`docs/DEPLOYMENT.md`](docs/DEPLOYMENT.md) - Complete deployment guide
- [`docs/DEPLOYMENT_SUMMARY.md`](docs/DEPLOYMENT_SUMMARY.md) - Implementation summary

## Security Features

- **Authentication**: Supabase Auth with Google OAuth
- **Authorization**: Row-level security (RLS) policies
- **API Security**: CORS, rate limiting, input validation
- **Data Encryption**: Encrypted storage and transmission
- **Audit Logging**: Comprehensive activity tracking

## Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/your-feature`
3. Make your changes and run tests
4. Format code: `bun run format`
5. Commit changes: `git commit -am 'Add your feature'`
6. Push to branch: `git push origin feature/your-feature`
7. Submit a pull request

### Code Standards
- **TypeScript**: Strict type checking enabled
- **ESLint**: Airbnb config with React rules
- **Prettier**: Consistent code formatting
- **Testing**: Unit tests for critical functions
- **Documentation**: JSDoc for public APIs

## Documentation

- [**Deployment Guide**](docs/DEPLOYMENT.md) - Production deployment instructions
- [**API Documentation**](docs/API.md) - Backend API reference
- [**Component Library**](docs/COMPONENTS.md) - UI component documentation
- [**Architecture Decisions**](docs/ARCHITECTURE.md) - Technical design decisions

## License

This project is proprietary software. All rights reserved.

## Support

For support and questions:
- **Issues**: GitHub Issues for bug reports and feature requests
- **Discussions**: GitHub Discussions for questions and community support
- **Documentation**: Comprehensive docs in the `/docs` directory

---

**Tradstry** - Transform your trading with data-driven insights and AI-powered analysis.