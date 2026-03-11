#!/bin/bash

set -e

# Replace with your VPS IP
VPS_IP="95.216.219.137"
VPS_USER="root"
SSH_KEY="$HOME/.ssh/id_ed25519_vps"
COMPOSE_DIR="/opt/qdrant"
SERVICE_NAME="qdrant"

echo "🚀 Deploying Qdrant to VPS..."
echo "Target: $VPS_USER@$VPS_IP"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m'

# Check SSH key
if [ ! -f "$SSH_KEY" ]; then
    echo -e "${RED}❌ SSH key not found at $SSH_KEY${NC}"
    exit 1
fi

# Generate or use existing API key
if [ -z "$QDRANT_API_KEY" ]; then
    echo -e "${BLUE}🔑 Generating Qdrant API key...${NC}"
    QDRANT_API_KEY=$(openssl rand -hex 32)
    echo -e "${GREEN}✅ Generated API key: $QDRANT_API_KEY${NC}"
    echo -e "${BLUE}💡 Save this key! Set it in your .env file as QDRANT_API_KEY${NC}"
else
    echo -e "${BLUE}🔑 Using provided QDRANT_API_KEY${NC}"
fi

# Create docker-compose.yml locally with API key
cat > docker-compose.qdrant.yml << EOF
services:
  qdrant:
    image: qdrant/qdrant:latest
    restart: always
    ports:
      - "6333:6333"
      - "6334:6334"
    volumes:
      - ./qdrant_storage:/qdrant/storage:z
    environment:
      - QDRANT__SERVICE__API_KEY=$QDRANT_API_KEY
      - QDRANT__SERVICE__GRPC_PORT=6334
EOF

echo -e "${BLUE}📋 Copying docker-compose.yml to VPS...${NC}"
ssh -i "$SSH_KEY" -o StrictHostKeyChecking=no "$VPS_USER@$VPS_IP" "mkdir -p $COMPOSE_DIR"
scp -i "$SSH_KEY" -o StrictHostKeyChecking=no docker-compose.qdrant.yml "$VPS_USER@$VPS_IP:$COMPOSE_DIR/docker-compose.yml"

echo -e "${GREEN}🚀 Starting Qdrant on VPS...${NC}"
ssh -i "$SSH_KEY" -o StrictHostKeyChecking=no "$VPS_USER@$VPS_IP" << 'ENDSSH'
set -e

cd /opt/qdrant

# Pull latest Qdrant image
echo "📥 Pulling Qdrant image..."
docker compose pull qdrant

# Stop existing container
echo "🛑 Stopping existing container..."
docker compose stop qdrant 2>/dev/null || true
docker compose rm -f qdrant 2>/dev/null || true

# Start Qdrant
echo "🔄 Starting Qdrant..."
docker compose up -d qdrant

# Wait for startup
sleep 3

# Check status
echo ""
echo "📊 Service status:"
docker compose ps qdrant

echo ""
echo "📋 Recent logs:"
docker compose logs --tail 20 qdrant

ENDSSH

if [ $? -eq 0 ]; then
    echo ""
    echo -e "${GREEN}✅ Qdrant deployed successfully!${NC}"
    echo ""
    echo "🔗 Access Qdrant REST API at: http://$VPS_IP:6333"
    echo "🔗 Access Qdrant gRPC at: http://$VPS_IP:6334"
    echo ""
    echo -e "${GREEN}🔑 API Key:${NC} $QDRANT_API_KEY"
    echo -e "${BLUE}💡 Add this to your .env file:${NC}"
    echo "   QDRANT_URL=http://$VPS_IP:6334"
    echo "   QDRANT_API_KEY=$QDRANT_API_KEY"
    echo ""
    echo "📋 Useful commands:"
    echo "  View logs: ssh -i $SSH_KEY $VPS_USER@$VPS_IP 'cd $COMPOSE_DIR && docker compose logs -f qdrant'"
    echo "  Check status: ssh -i $SSH_KEY $VPS_USER@$VPS_IP 'cd $COMPOSE_DIR && docker compose ps'"
else
    echo -e "${RED}❌ Deployment failed!${NC}"
    exit 1
fi

# Cleanup local temp file
rm -f docker-compose.qdrant.yml