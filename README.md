# Price Tracker

A containerized price/version/number tracking application that monitors web pages for changes in specified values and sends notifications when thresholds are met. Built with Node.js 22, TypeScript, and Playwright.

## ( Features

- **Multi-Source Tracking**: Track the same product across multiple websites
- **Plugin-Based Architecture**: Extensible tracker and notifier plugins
- **Smart Element Detection**: Visual element picker with confidence ranking
- **Multiple Tracker Types**:
  - =° Price tracking with multi-currency support
  - = Semantic version tracking (1.20.1, 2.0.0-beta)
  - =Ê Generic number tracking (stock, scores, ratings)
- **Notification Systems**:
  - =ç Email notifications via SMTP
  - =¬ Discord notifications via webhooks
- **Cross-Source Comparison**: Find the best deals automatically
- **Flexible Scheduling**: Custom cron expressions for check intervals
- **Web Interface**: Dashboard, setup wizard, and element scanner
- **Docker Ready**: Easy deployment with Docker Compose

## =€ Quick Start

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

## =' Development

### Setup Development Environment

```bash
# Install dependencies
npm install

# Generate Prisma client
npm run db:generate

# Start development server with auto-reload
npm run dev

# Build for production
npm run build

# Start production server
npm start
```

## =Ê Architecture

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

## =Ö Usage

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

## =à Configuration

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
4. Submit a pull request

## =Ä License

MIT License - see LICENSE file for details.

---

**Built with Node.js 22, TypeScript, Playwright, and modern web technologies.**