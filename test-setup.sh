#!/bin/bash

# Price Tracker - Quick Test Setup Script
# This script helps verify the application is working correctly

set -e

echo "🧪 Price Tracker Test Setup"
echo "=========================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print status
print_status() {
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓${NC} $1"
    else
        echo -e "${RED}✗${NC} $1"
    fi
}

# Check prerequisites
echo -e "\n${YELLOW}Checking Prerequisites...${NC}"

# Check Node.js version
node --version > /dev/null 2>&1
print_status "Node.js is installed"

# Check npm version
npm --version > /dev/null 2>&1
print_status "npm is installed"

# Check if we're in the right directory
if [ ! -f "package.json" ]; then
    echo -e "${RED}Error: Please run this script from the price-tracker directory${NC}"
    exit 1
fi

# Install dependencies if needed
if [ ! -d "node_modules" ]; then
    echo -e "\n${YELLOW}Installing dependencies...${NC}"
    npm install
    print_status "Dependencies installed"
fi

# Setup environment
if [ ! -f ".env" ]; then
    echo -e "\n${YELLOW}Setting up environment...${NC}"
    cp .env.example .env
    # Generate a random secret key
    SECRET_KEY=$(openssl rand -hex 32 2>/dev/null || head -c 32 /dev/urandom | xxd -p -c 32)
    sed -i.bak "s/your-secret-key-here-change-in-production/$SECRET_KEY/g" .env
    sed -i.bak "s/NODE_ENV=production/NODE_ENV=development/g" .env
    sed -i.bak "s|DATABASE_URL=file:/data/tracker.db|DATABASE_URL=file:./data/tracker.db|g" .env
    rm .env.bak 2>/dev/null || true
    print_status "Environment configured"
fi

# Create directories
mkdir -p data logs
print_status "Directories created"

# Generate Prisma client
echo -e "\n${YELLOW}Setting up database...${NC}"
npm run db:generate > /dev/null 2>&1
print_status "Prisma client generated"

# Install Playwright browsers
if ! command -v playwright &> /dev/null; then
    echo -e "\n${YELLOW}Installing Playwright browsers...${NC}"
    npx playwright install chromium > /dev/null 2>&1
    print_status "Playwright browsers installed"
fi

# Build the application
echo -e "\n${YELLOW}Building application...${NC}"
npm run build > /dev/null 2>&1
print_status "Application built"

echo -e "\n${GREEN}✅ Setup complete!${NC}"
echo -e "\nTo start the application:"
echo -e "  ${YELLOW}npm run dev${NC}     # Development mode with hot reload"
echo -e "  ${YELLOW}npm start${NC}       # Production mode"
echo -e "  ${YELLOW}docker-compose up${NC} # Docker mode"

echo -e "\nTo test the application:"
echo -e "  ${YELLOW}curl http://localhost:3000/health${NC}"
echo -e "  ${YELLOW}open http://localhost:3000${NC}"

echo -e "\nUseful commands:"
echo -e "  ${YELLOW}node dist/index.js help${NC}     # Show CLI help"
echo -e "  ${YELLOW}node dist/index.js status${NC}   # Show status"
echo -e "  ${YELLOW}node dist/index.js check${NC}    # Manual tracking check"

echo -e "\n${GREEN}Happy tracking! 🔍${NC}"