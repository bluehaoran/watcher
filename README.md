# Price Tracker

A containerized price/version/number tracking application that monitors web pages for changes in specified values and sends notifications when thresholds are met. Built with Node.js 22, TypeScript, and Playwright.

## ( Features

- **Multi-Source Tracking**: Track the same product across multiple websites
- **Plugin-Based Architecture**: Extensible tracker and notifier plugins
- **Smart Element Detection**: Visual element picker with confidence ranking
- **Multiple Tracker Types**:
  - =� Price tracking with multi-currency support
  - = Semantic version tracking (1.20.1, 2.0.0-beta)
  - =� Generic number tracking (stock, scores, ratings)
- **Notification Systems**:
  - =� Email notifications via SMTP
  - =� Discord notifications via webhooks
- **Cross-Source Comparison**: Find the best deals automatically
- **Flexible Scheduling**: Custom cron expressions for check intervals
- **Web Interface**: Dashboard, setup wizard, and element scanner
- **Docker Ready**: Easy deployment with Docker Compose

## =� Quick Start

### Using Docker (Recommended)

1. **Clone and Setup**
   ```bash
   git clone <repository-url>
   cd price-tracker
   cp .env.example .env
   ```

2. **Configure Environment**
   Edit `.env` with your settings:
   ```bash
   # Basic settings
   SECRET_KEY=your-secret-key-change-this-in-production
   BASE_URL=http://localhost:3000
   
   # Email notifications (optional)
   SMTP_HOST=smtp.gmail.com
   SMTP_USER=your-email@gmail.com
   SMTP_PASS=your-app-password
   SMTP_FROM=Price Tracker <your-email@gmail.com>
   
   # Discord notifications (optional)
   DISCORD_WEBHOOK=https://discord.com/api/webhooks/...
   ```

3. **Start the Application**
   ```bash
   docker-compose up -d
   ```

4. **Access the Web Interface**
   - Open http://localhost:3000
   - Follow the setup wizard to add your first product

## 🚀 Installation & Setup

### Prerequisites

- **Node.js 22+** (check with `node --version`)
- **npm 10+** (check with `npm --version`)
- **Git** for version control
- **Docker & Docker Compose** (optional, for containerized deployment)

### Method 1: Docker Setup (Recommended)

1. **Clone the Repository**
   ```bash
   git clone <repository-url>
   cd price-tracker
   ```

2. **Configure Environment**
   ```bash
   cp .env.example .env
   # Edit .env with your preferred text editor
   nano .env
   ```

3. **Start with Docker Compose**
   ```bash
   docker-compose up -d
   ```

4. **Access the Application**
   - Web Interface: http://localhost:3000
   - Health Check: http://localhost:3000/health

### Method 2: Local Development Setup

1. **Clone and Install Dependencies**
   ```bash
   git clone <repository-url>
   cd price-tracker
   npm install
   ```

2. **Configure Environment**
   ```bash
   cp .env.example .env
   # Edit .env and set required values:
   # - SECRET_KEY (generate a secure random string)
   # - DATABASE_URL=file:./data/tracker.db
   # - NODE_ENV=development
   ```

3. **Setup Database**
   ```bash
   # Create data directory
   mkdir -p data logs
   
   # Generate Prisma client
   npm run db:generate
   
   # Run database migrations (if any)
   npm run db:migrate
   ```

4. **Install Playwright Browsers**
   ```bash
   npx playwright install chromium
   ```

5. **Start Development Server**
   ```bash
   npm run dev
   ```

## 🧪 Testing the Application

### Basic Functionality Test

1. **Health Check**
   ```bash
   curl http://localhost:3000/health
   # Should return: {"status":"healthy","timestamp":"..."}
   ```

2. **Access Web Interface**
   - Open http://localhost:3000 in your browser
   - You should see the Price Tracker dashboard

### Test with a Real Product

1. **Navigate to Setup Wizard**
   - Go to http://localhost:3000/setup
   - Click "Add Product"

2. **Test Element Scanner**
   - Visit http://localhost:3000/scanner
   - Enter a test URL: `https://example.com`
   - Search for text: "Example"
   - Verify elements are found and ranked

3. **Create a Test Product**
   ```bash
   # Use the web interface or API:
   curl -X POST http://localhost:3000/products \
     -H "Content-Type: application/json" \
     -d '{
       "name": "Test Product",
       "trackerType": "price",
       "sources": [{
         "url": "https://example.com",
         "selector": ".price",
         "selectorType": "css"
       }],
       "notifications": []
     }'
   ```

### Manual Tracking Test

```bash
# Trigger a manual tracking cycle
npm run start -- check
# or via API:
curl -X POST http://localhost:3000/api/track/manual
```

## 🔧 Development & Testing

### Development Commands

```bash
# Start development server with hot reload
npm run dev

# Build TypeScript to JavaScript
npm run build

# Start production server
npm start

# Format code
npm run format

# Lint code
npm run lint

# Check types
npx tsc --noEmit
```

### Database Management

```bash
# Generate Prisma client after schema changes
npm run db:generate

# Create and run migrations
npm run db:migrate

# Open database browser
npm run db:studio

# Reset database (development only)
rm -rf data/tracker.db*
npm run db:migrate
```

### Running Tests

```bash
# Run unit tests
npm test

# Run tests with UI
npm run test:ui

# Run tests in watch mode
npm run test -- --watch

# Run specific test file
npm test -- src/plugins/trackers/PriceTracker.test.ts
```

### CLI Commands

```bash
# Show application status
node dist/index.js status

# Run manual tracking check
node dist/index.js check

# Show help
node dist/index.js help
```

## 🧩 Component Testing

### Test Individual Plugins

1. **Price Tracker Plugin**
   ```bash
   # Test price parsing
   curl -X POST http://localhost:3000/api/test/tracker \
     -H "Content-Type: application/json" \
     -d '{"type":"price","text":"$99.99"}'
   ```

2. **Element Scanner**
   ```bash
   # Test element detection
   curl -X POST http://localhost:3000/scanner/scan \
     -H "Content-Type: application/json" \
     -d '{"url":"https://example.com","searchText":"test"}'
   ```

3. **Notification System**
   ```bash
   # Test email notifications (requires SMTP config)
   curl -X POST http://localhost:3000/setup/test-notifier \
     -H "Content-Type: application/json" \
     -d '{
       "type":"email",
       "config":{
         "host":"smtp.gmail.com",
         "port":587,
         "user":"your-email@gmail.com",
         "pass":"your-app-password",
         "to":"test@example.com"
       }
     }'
   ```

## 🐛 Troubleshooting

### Common Issues

1. **Port Already in Use**
   ```bash
   # Change port in .env file or:
   PORT=3001 npm run dev
   ```

2. **Database Connection Issues**
   ```bash
   # Ensure data directory exists
   mkdir -p data
   # Regenerate Prisma client
   npm run db:generate
   ```

3. **Playwright Browser Issues**
   ```bash
   # Install browsers
   npx playwright install chromium
   # Or use system browser
   export PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH=/usr/bin/chromium-browser
   ```

4. **Memory Issues**
   ```bash
   # Increase memory limit
   export NODE_OPTIONS="--max-old-space-size=1024"
   ```

### Debug Mode

```bash
# Enable debug logging
LOG_LEVEL=debug npm run dev

# Enable Playwright debug
DEBUG=pw:* npm run dev

# Check application logs
tail -f logs/combined.log
# or for Docker:
docker-compose logs -f price-tracker
```

### Performance Testing

```bash
# Monitor memory usage
npm run dev &
watch -n 1 'ps aux | grep node'

# Test concurrent requests
curl -X GET http://localhost:3000/products &
curl -X GET http://localhost:3000/products &
curl -X GET http://localhost:3000/products &
```

## 🔍 API Testing

### Using curl

```bash
# Get all products
curl http://localhost:3000/products

# Get specific product
curl http://localhost:3000/products/PRODUCT_ID

# Create product
curl -X POST http://localhost:3000/products \
  -H "Content-Type: application/json" \
  -d @test-product.json

# Health check
curl http://localhost:3000/health
```

### Using Postman/Insomnia

Import these endpoints:
- `GET /health` - Health check
- `GET /products` - List products
- `POST /products` - Create product
- `GET /products/{id}` - Get product details
- `PUT /products/{id}` - Update product
- `DELETE /products/{id}` - Delete product
- `POST /scanner/scan` - Test element detection

## 📊 Monitoring

### Application Metrics

```bash
# Check application status
curl http://localhost:3000/api/health | jq

# Monitor logs
tail -f data/logs/combined.log

# Database size
ls -lah data/tracker.db*
```

### Docker Monitoring

```bash
# Container stats
docker stats price-tracker

# Container logs
docker logs price-tracker -f

# Resource usage
docker-compose top
```

## =� Architecture

The application follows a modular plugin-based architecture:

- **Core Engine**: Handles scraping, tracking, and scheduling
- **Plugin System**: Extensible trackers (Price, Version, Number) and notifiers (Email, Discord)
- **Web Interface**: Dashboard, element scanner, and management UI
- **Database**: SQLite with Prisma ORM for data persistence

## =3 Docker Deployment

```bash
# Development
docker-compose up

# Production
NODE_ENV=production docker-compose up -d

# View logs
docker-compose logs -f price-tracker
```

## =� Usage

1. **Add a Product**: Visit the dashboard and click "Add Product"
2. **Configure Sources**: Add URLs where the product can be found
3. **Select Elements**: Use the visual element picker to choose tracking targets
4. **Set Notifications**: Configure email or Discord alerts
5. **Schedule Checks**: Set how often to monitor for changes

The application will automatically:
- Scrape configured URLs on schedule
- Parse and compare values
- Find the best deals across sources
- Send notifications when thresholds are met

## =� Configuration

Key environment variables:

- `SECRET_KEY`: Application security key (required)
- `DATABASE_URL`: SQLite database path
- `SMTP_*`: Email notification settings
- `DISCORD_WEBHOOK`: Discord notification URL
- `MAX_CONCURRENT_CHECKS`: Scraping concurrency limit

See `.env.example` for complete configuration options.

## > Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Ensure your changes are compatible with GPL v3 requirements
5. Submit a pull request

## =� License

This project is licensed under the GNU General Public License v3.0 - see the [LICENSE](LICENSE) file for details.

### GPL v3 Requirements

This software is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.

**Key implications:**
- ✅ Free to use, modify, and distribute
- ✅ Source code must remain available
- ✅ Derivative works must also be GPL v3
- ✅ Commercial use allowed with GPL v3 compliance
- ❌ Cannot be incorporated into proprietary software

For commercial licensing options or questions about GPL v3 compliance, please open an issue.

---

**Built with Node.js 22, TypeScript, Playwright, and modern web technologies.**