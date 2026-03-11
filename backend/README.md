# Tradistry Backend - Setup Guide

## Overview

The Tradistry backend is a high-performance Rust-based API built with ActixWeb 4.4, serving a multi-tenant architecture where each user gets their own isolated Turso (SQLite) database.

## Prerequisites

### Required Software
- **Rust**: 1.70+ (install from [rustup.rs](https://rustup.rs/))
- **Cargo**: Comes with Rust
- **Docker**: For local development and database services
- **Git**: For version control

### System Requirements
- **RAM**: 4GB minimum
- **Disk Space**: 2GB for dependencies and databases
- **OS**: Linux, macOS, or Windows (with WSL2)

## Development Setup

### 1. Clone Repository
```bash
git clone <repository-url>
cd tradstry
```

### 2. Install Dependencies
```bash
cargo build
```

### 3. Environment Configuration

Create the environment file:

```bash
cp .env.example .env.production
```

**Required Environment Variables:**

```bash
# Production Environment
RUST_ENV=production
RUST_LOG=info

# Database Configuration
REGISTRY_DB_URL=libsql://your-registry-db.turso.io
REGISTRY_DB_TOKEN=your-turso-registry-token
TURSO_API_TOKEN=your-turso-api-token
TURSO_ORG=your-turso-organization

# Supabase Authentication
SUPABASE_URL=https://your-project.supabase.co
SUPABASE_ANON_KEY=your-supabase-anon-key
SUPABASE_SERVICE_ROLE_KEY=your-supabase-service-role-key

# Uploadcare (File Storage)
UPLOADCARE_PUBLIC_KEY=your-uploadcare-public-key
UPLOADCARE_SECRET_KEY=your-uploadcare-secret-key

# AI Services
OPENROUTER_API_KEY=sk-or-v1-...
OPENROUTER_SITE_URL=https://tradstry.com
OPENROUTER_SITE_NAME=Tradstry

# Vector Search
UPSTASH_VECTOR_REST_URL=https://your-vector-db.upstash.io
UPSTASH_VECTOR_REST_TOKEN=your-vector-token
QDRANT_URL=https://your-qdrant-instance.qdrant.io
QDRANT_API_KEY=your-qdrant-api-key

# Redis (Caching)
UPSTASH_REDIS_REST_URL=https://your-redis.upstash.io
UPSTASH_REDIS_REST_TOKEN=your-redis-token

# Search
UPSTASH_SEARCH_REST_URL=https://your-search.upstash.io
UPSTASH_SEARCH_REST_TOKEN=your-search-token

# Web Push Notifications
VAPID_PUBLIC_KEY=your-vapid-public-key
VAPID_PRIVATE_KEY=your-vapid-private-key
WEB_PUSH_SUBJECT=mailto:support@tradstry.com

# Server Configuration
PORT=8080
HOST=0.0.0.0

# CORS (Production Origins)
ALLOWED_ORIGINS=https://tradstry.com,https://app.tradstry.com
```

### 4. Run Development Server

```bash
# Using startup script
../start.sh

# Or manually
export PORT=9000
export RUST_BACKTRACE=1
cargo run
```

## Production Deployment

### Prerequisites

- VPS with Ubuntu/Debian
- Docker and Docker Compose installed
- Domain name configured
- SSL certificates ready
- SSH access configured

### 1. Server Preparation

```bash
# Update system
sudo apt update && sudo apt upgrade -y

# Install Docker
curl -fsSL https://get.docker.com -o get-docker.sh
sudo sh get-docker.sh
sudo usermod -aG docker $USER

# Install Docker Compose
sudo curl -L "https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
sudo chmod +x /usr/local/bin/docker-compose
```

### 2. Deploy to Production

#### Option A: Using Docker Compose (Recommended)

```bash
# Create deployment directory
sudo mkdir -p /opt/tradstry
sudo chown $USER:$USER /opt/tradstry
cd /opt/tradstry

# Copy project files (from your local machine)
rsync -avz --delete \
  --exclude 'node_modules' \
  --exclude '.git' \
  --exclude 'target' \
  ./ root@your-vps-ip:/opt/tradstry/
```

Create production environment file:

```bash
nano backend/.env.production
# Add all required environment variables (see above)
```

Create Docker Compose file for production:

```yaml
# docker-compose.production.yaml
version: '3.8'

services:
  backend:
    build:
      context: .
      dockerfile: backend/dockerfile
    container_name: tradstry-backend
    restart: unless-stopped
    environment:
      - RUST_ENV=production
    env_file:
      - backend/.env.production
    ports:
      - "8080:8080"
    volumes:
      - ./backend/logs:/app/logs
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
    networks:
      - tradstry-network

  nginx:
    image: nginx:alpine
    container_name: tradstry-nginx
    restart: unless-stopped
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx/nginx.conf:/etc/nginx/nginx.conf:ro
      - ./nginx/tradstry.conf:/etc/nginx/conf.d/tradstry.conf:ro
      - ./nginx/ssl:/etc/nginx/ssl:ro
      - ./nginx/logs:/var/log/nginx
    depends_on:
      - backend
    healthcheck:
      test: ["CMD", "nginx", "-t"]
      interval: 30s
      timeout: 10s
      retries: 3
    networks:
      - tradstry-network

networks:
  tradstry-network:
    driver: bridge
```

Start the services:

```bash
docker-compose -f docker-compose.production.yaml up -d --build
```

#### Option B: Manual Deployment

```bash
# Build the backend
cd backend
cargo build --release

# Install as system service
sudo cp target/release/tradstry-backend /usr/local/bin/
sudo useradd -r -s /bin/false tradstry
sudo chown tradstry:tradstry /usr/local/bin/tradstry-backend

# Create systemd service
sudo nano /etc/systemd/system/tradstry-backend.service
```

Service file content:

```ini
[Unit]
Description=Tradstry Backend API
After=network.target

[Service]
Type=simple
User=tradstry
EnvironmentFile=/opt/tradstry/backend/.env.production
ExecStart=/usr/local/bin/tradstry-backend
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal
SyslogIdentifier=tradstry-backend

[Install]
WantedBy=multi-user.target
```

Enable and start the service:

```bash
sudo systemctl daemon-reload
sudo systemctl enable tradstry-backend
sudo systemctl start tradstry-backend
sudo systemctl status tradstry-backend
```

### 3. Nginx Configuration

Install and configure Nginx as reverse proxy:

```bash
# Install Nginx
sudo apt install nginx -y

# Create site configuration
sudo nano /etc/nginx/sites-available/tradstry
```

Nginx configuration:

```nginx
# /etc/nginx/sites-available/tradstry
upstream backend {
    server localhost:8080;
}

server {
    listen 80;
    server_name tradstry.com www.tradstry.com;

    # Redirect to HTTPS
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name tradstry.com www.tradstry.com;

    # SSL Configuration
    ssl_certificate /etc/ssl/certs/tradstry.crt;
    ssl_certificate_key /etc/ssl/private/tradstry.key;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers ECDHE-RSA-AES128-GCM-SHA256:ECDHE-RSA-AES256-GCM-SHA384;
    ssl_prefer_server_ciphers off;

    # Security headers
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-XSS-Protection "1; mode=block" always;
    add_header Referrer-Policy "strict-origin-when-cross-origin" always;

    # Rate limiting
    limit_req_zone $binary_remote_addr zone=api:10m rate=10r/s;
    limit_req zone=api burst=20 nodelay;

    # CORS
    add_header 'Access-Control-Allow-Origin' 'https://app.tradstry.com' always;
    add_header 'Access-Control-Allow-Methods' 'GET, POST, PUT, DELETE, OPTIONS' always;
    add_header 'Access-Control-Allow-Headers' 'Authorization, Content-Type, X-Requested-With' always;
    add_header 'Access-Control-Allow-Credentials' 'true' always;

    # Handle preflight requests
    if ($request_method = 'OPTIONS') {
        return 204;
    }

    # API routes
    location /api/ {
        proxy_pass http://backend;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # Rate limiting for API
        limit_req zone=api burst=20 nodelay;

        # Timeout settings
        proxy_connect_timeout 30s;
        proxy_send_timeout 30s;
        proxy_read_timeout 30s;
    }

    # Health check (no rate limiting)
    location /health {
        proxy_pass http://backend;
        proxy_set_header Host $host;
        access_log off;
    }
}
```

Enable the site:

```bash
sudo ln -s /etc/nginx/sites-available/tradstry /etc/nginx/sites-enabled/
sudo nginx -t
sudo systemctl reload nginx
```

### 4. SSL Certificate Setup

Using Let's Encrypt:

```bash
# Install certbot
sudo apt install certbot python3-certbot-nginx -y

# Get certificate
sudo certbot --nginx -d tradstry.com -d www.tradstry.com

# Set up auto-renewal
sudo crontab -e
# Add: 0 12 * * * /usr/bin/certbot renew --quiet
```

### 5. Firewall Configuration

```bash
# Configure UFW
sudo ufw allow OpenSSH
sudo ufw allow 'Nginx Full'
sudo ufw --force enable

# Check status
sudo ufw status
```

## Deployment Scripts

### Automated Deployment Script

Create `scripts/deploy-backend.sh`:

```bash
#!/bin/bash

# Tradstry Backend Production Deployment Script
set -e

# Configuration
VPS_HOST="${VPS_HOST:-your-vps-ip}"
VPS_USER="${VPS_USER:-root}"
SSH_KEY="${SSH_KEY:-~/.ssh/id_rsa}"
DEPLOY_DIR="${DEPLOY_DIR:-/opt/tradstry}"

echo "🚀 Deploying Tradstry Backend to production..."

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m'

# Check prerequisites
check_prerequisites() {
    if ! command -v rsync &> /dev/null; then
        echo -e "${RED}rsync not found. Install with: apt install rsync${NC}"
        exit 1
    fi
}

# Backup current deployment
backup_current() {
    echo -e "${BLUE}📦 Creating backup...${NC}"
    ssh -i "$SSH_KEY" "$VPS_USER@$VPS_HOST" "
        mkdir -p /opt/tradstry-backups
        TIMESTAMP=\$(date +%Y%m%d_%H%M%S)
        cp -r $DEPLOY_DIR /opt/tradstry-backups/backup_\$TIMESTAMP
        echo \"Backup created: backup_\$TIMESTAMP\"
    "
}

# Deploy code
deploy_code() {
    echo -e "${BLUE}📤 Deploying code...${NC}"
    rsync -avz --delete \
        --exclude 'node_modules' \
        --exclude '.git' \
        --exclude 'target' \
        --exclude '.next' \
        --exclude '*.log' \
        -e "ssh -i $SSH_KEY" \
        ./ "$VPS_USER@$VPS_HOST:$DEPLOY_DIR/"
}

# Update services
update_services() {
    echo -e "${BLUE}🔄 Updating services...${NC}"
    ssh -i "$SSH_KEY" "$VPS_USER@$VPS_HOST" "
        cd $DEPLOY_DIR

        # Stop services
        docker-compose -f docker-compose.production.yaml down || true

        # Rebuild and start
        docker-compose -f docker-compose.production.yaml up -d --build

        # Wait for health check
        echo 'Waiting for services to be healthy...'
        for i in {1..30}; do
            if curl -f http://localhost:8080/health > /dev/null 2>&1; then
                echo '✅ Backend is healthy'
                break
            fi
            sleep 2
        done
    "
}

# Verify deployment
verify_deployment() {
    echo -e "${BLUE}🔍 Verifying deployment...${NC}"

    # Test backend health
    if ssh -i "$SSH_KEY" "$VPS_USER@$VPS_HOST" "curl -f http://localhost:8080/health"; then
        echo -e "${GREEN}✅ Backend health check passed${NC}"
    else
        echo -e "${RED}❌ Backend health check failed${NC}"
        exit 1
    fi

    # Test through Nginx
    if curl -f https://tradstry.com/health; then
        echo -e "${GREEN}✅ Full stack health check passed${NC}"
    else
        echo -e "${RED}❌ Full stack health check failed${NC}"
        exit 1
    fi
}

# Main deployment
main() {
    check_prerequisites
    backup_current
    deploy_code
    update_services
    verify_deployment

    echo -e "${GREEN}🎉 Deployment completed successfully!${NC}"
    echo ""
    echo -e "${BLUE}📊 Deployment Summary:${NC}"
    echo "  Host: $VPS_HOST"
    echo "  Directory: $DEPLOY_DIR"
    echo "  Time: $(date)"
    echo ""
    echo -e "${BLUE}🔗 Useful commands:${NC}"
    echo "  Check logs: ssh $VPS_USER@$VPS_HOST 'cd $DEPLOY_DIR && docker-compose -f docker-compose.production.yaml logs -f'"
    echo "  Restart: ssh $VPS_USER@$VPS_HOST 'cd $DEPLOY_DIR && docker-compose -f docker-compose.production.yaml restart'"
    echo "  Health check: curl https://tradstry.com/health"
}

# Rollback function
rollback() {
    echo -e "${BLUE}🔄 Rolling back to previous deployment...${NC}"
    ssh -i "$SSH_KEY" "$VPS_USER@$VPS_HOST" "
        cd /opt/tradstry-backups
        LATEST_BACKUP=\$(ls -td backup_* | head -1)
        if [ -n \"\$LATEST_BACKUP\" ]; then
            echo \"Restoring \$LATEST_BACKUP\"
            cd $DEPLOY_DIR
            docker-compose -f docker-compose.production.yaml down
            cp -r /opt/tradstry-backups/\$LATEST_BACKUP/* ./
            docker-compose -f docker-compose.production.yaml up -d --build
            echo \"Rollback completed\"
        else
            echo \"No backup found for rollback\"
            exit 1
        fi
    "
}

# Handle command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--host) VPS_HOST="$2"; shift 2 ;;
        -u|--user) VPS_USER="$2"; shift 2 ;;
        -k|--key) SSH_KEY="$2"; shift 2 ;;
        -d|--dir) DEPLOY_DIR="$2"; shift 2 ;;
        --rollback) ROLLBACK=true; shift ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

if [ "$ROLLBACK" = true ]; then
    rollback
else
    main "$@"
fi
```

Make executable and run:

```bash
chmod +x scripts/deploy-backend.sh

# Deploy
./scripts/deploy-backend.sh -h your-vps-ip -u root -k ~/.ssh/your-key

# Or set environment variables
export VPS_HOST=your-vps-ip
export VPS_USER=root
export SSH_KEY=~/.ssh/your-key
./scripts/deploy-backend.sh
```

## Monitoring & Maintenance

### Health Checks

```bash
# Backend health
curl http://localhost:8080/health

# Through Nginx
curl https://tradstry.com/health

# Full API test
curl https://tradstry.com/api/options/test
```

### Log Management

```bash
# View logs
docker-compose -f docker-compose.production.yaml logs -f backend

# Nginx logs
docker logs -f tradstry-nginx

# System logs
sudo journalctl -u tradstry-backend -f
```

### Performance Monitoring

```bash
# Container stats
docker stats tradstry-backend tradstry-nginx

# System resources
htop
df -h
free -h

# Network connections
netstat -tulpn | grep :8080
```

### Backup Strategy

```bash
# Database backup (Turso handles replication automatically)

# Environment backup
cp backend/.env.production backend/.env.production.backup

# SSL certificates
sudo cp /etc/ssl/certs/tradstry.crt /etc/ssl/certs/tradstry.crt.backup
```

### Updates & Rollbacks

```bash
# Update deployment
git pull origin main
./scripts/deploy-backend.sh

# Rollback if needed
./scripts/deploy-backend.sh --rollback
```

## Troubleshooting

### Common Issues

#### Backend Not Starting

**Symptoms:**
- Container exits immediately
- Health check fails
- Port 8080 not listening

**Solutions:**
```bash
# Check logs
docker logs tradstry-backend

# Validate environment
docker exec tradstry-backend env | grep RUST

# Test database connection
docker exec tradstry-backend curl -f http://localhost:8080/health

# Rebuild
docker-compose -f docker-compose.production.yaml build --no-cache backend
```

#### Database Connection Issues

**Symptoms:**
- "Failed to connect to database" errors
- Health check shows unhealthy database

**Solutions:**
```bash
# Verify Turso credentials
grep TURSO backend/.env.production

# Test database connectivity
curl -H "Authorization: Bearer YOUR_TOKEN" https://your-db.turso.io/health

# Check network connectivity
docker exec tradstry-backend ping 8.8.8.8
```

#### CORS Errors

**Symptoms:**
- Frontend can't make API calls
- Browser console shows CORS errors

**Solutions:**
```bash
# Check Nginx CORS headers
curl -I https://tradstry.com/api/health

# Verify frontend domain in ALLOWED_ORIGINS
grep ALLOWED_ORIGINS backend/.env.production

# Restart services
docker-compose -f docker-compose.production.yaml restart
```

#### SSL Certificate Issues

**Symptoms:**
- "Connection not secure" warnings
- HTTPS redirects failing

**Solutions:**
```bash
# Check certificate validity
openssl s_client -connect tradstry.com:443 -servername tradstry.com

# Renew with Let's Encrypt
sudo certbot renew --dry-run
sudo certbot renew

# Reload Nginx
sudo nginx -t && sudo nginx -s reload
```

#### High Memory Usage

**Symptoms:**
- Container restarts due to OOM
- Slow response times

**Solutions:**
```bash
# Check memory usage
docker stats tradstry-backend

# Add memory limits to docker-compose
services:
  backend:
    deploy:
      resources:
        limits:
          memory: 1G
        reservations:
          memory: 512M

# Profile application
docker exec tradstry-backend curl http://localhost:8080/debug/pprof/heap
```

### Performance Optimization

```bash
# Database query optimization
EXPLAIN QUERY PLAN SELECT * FROM trades WHERE user_id = ?;

# Add database indexes
CREATE INDEX idx_trades_user_date ON trades(user_id, created_at);

# Connection pooling (handled by libsql client)

# Optimize Nginx
worker_processes auto;
worker_connections 1024;
```

### Security Hardening

```bash
# Regular updates
sudo apt update && sudo apt upgrade

# Fail2ban for SSH protection
sudo apt install fail2ban

# Configure firewall
sudo ufw status

# SSL security
ssl_protocols TLSv1.2 TLSv1.3;
ssl_ciphers ECDHE-RSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA384;
```

## API Documentation

### Core Endpoints

#### Health Check
```http
GET /health
```

Response:
```json
{
  "success": true,
  "data": {
    "status": "healthy",
    "database": "connected",
    "timestamp": "2024-01-01T12:00:00Z"
  }
}
```

#### User Authentication
```http
GET /me
Authorization: Bearer <jwt-token>
```

#### User Data
```http
GET /my-data
Authorization: Bearer <jwt-token>
```

### Trading Endpoints

#### Analytics
```http
GET /api/analytics/core?time_range=30d
GET /api/analytics/risk?time_range=30d
GET /api/analytics/performance?time_range=30d
GET /api/analytics/time-series?time_range=30d
GET /api/analytics/grouped?time_range=30d
```

#### Trade Management
```http
GET /api/trade-notes
POST /api/trade-notes
PUT /api/trade-notes/{id}
DELETE /api/trade-notes/{id}
```

#### Market Data
```http
GET /api/stocks/quote/{symbol}
GET /api/options/chains/{symbol}
GET /api/markets/movers
```

### AI Endpoints

#### Chat
```http
POST /api/ai/chat
Content-Type: application/json

{
  "message": "What are some good trading strategies?",
  "context": "stock_trading"
}
```

#### Insights
```http
POST /api/ai/insights/trade-analysis
Content-Type: application/json

{
  "trade_id": 123,
  "analysis_type": "performance"
}
```

### Notebook Endpoints

#### Documents
```http
GET /api/notebook/documents
POST /api/notebook/documents
PUT /api/notebook/documents/{id}
DELETE /api/notebook/documents/{id}
```

#### Images
```http
POST /api/notebook/images/upload
GET /api/notebook/images/{id}
DELETE /api/notebook/images/{id}
```

### WebSocket Connections

Real-time updates for market data:

```javascript
const ws = new WebSocket('wss://tradstry.com/api/ws');

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  // Handle real-time market updates
};
```

## Support & Contributing

For issues or contributions:

1. Check existing issues on GitHub
2. Create a new issue with detailed information
3. Include logs and error messages
4. Specify your environment and setup

### Development Commands

```bash
# Build
cargo build

# Run tests
cargo test

# Format code
cargo fmt

# Lint code
cargo clippy

# Generate docs
cargo doc --open
```

