#!/bin/bash

# Combined deployment script for both backend and snaptrade-service
# This script deploys both services to the VPS
# Usage: ./deploy-all.sh [production|staging]

set -e

ENV=${1:-production}
# Replace with your VPS IP 
VPS_IP="95.216.219.137"
VPS_USER="root"
SSH_KEY="$HOME/.ssh/id_ed25519_vps"
COMPOSE_DIR="/opt/tradstry"

echo "🚀 Starting combined deployment to VPS..."
echo "Environment: $ENV"
echo "Target: $VPS_USER@$VPS_IP"
echo "Services: backend, snaptrade-service"
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Check if SSH key exists
if [ ! -f "$SSH_KEY" ]; then
    echo -e "${RED}❌ SSH key not found at $SSH_KEY${NC}"
    exit 1
fi

# Deploy both services
echo -e "${GREEN}🚀 Deploying backend service...${NC}"
./deploy.sh "$ENV"

echo -e "${GREEN}🚀 Deploying snaptrade-service...${NC}"
./snaptrade-service/deploy.sh "$ENV"

echo ""
echo -e "${GREEN}✅ Combined deployment complete!${NC}"
echo ""
echo -e "${YELLOW}📋 Useful commands:${NC}"
echo "  Backend logs: ssh -i $SSH_KEY $VPS_USER@$VPS_IP 'cd $COMPOSE_DIR && docker compose logs -f backend'"
echo "  Snaptrade logs: ssh -i $SSH_KEY $VPS_USER@$VPS_IP 'cd $COMPOSE_DIR && docker compose logs -f snaptrade-service'"
echo "  Status: ssh -i $SSH_KEY $VPS_USER@$VPS_IP 'cd $COMPOSE_DIR && docker compose ps'"

