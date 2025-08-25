# CLAUDE.md - Price Tracker Application

## Project Overview

A containerized price/version/number tracking application that monitors web pages for changes in specified values and sends notifications when thresholds are met. Features a plugin system for both tracking types and notification methods.

## Current Architecture

### Core Components ✅ Implemented
- **Web Scraping**: Playwright-based scraper with element finding (`src/core/scraper.ts`, `src/core/elementFinder.ts`)
- **Product Management**: Multi-URL product tracking (`src/core/productManager.ts`)
- **Tracking Engine**: Main orchestration logic (`src/core/tracker.ts`)
- **Database**: FileStore-based flat file storage (`src/core/database.ts`, `src/core/filestore.ts`)
- **Plugin System**: Extensible trackers and notifiers (`src/plugins/`)
- **Web Interface**: Express.js server with routes (`src/web/`)
- **Scheduler**: Cron-based job scheduling (`src/core/scheduler.ts`)

### Technology Stack
- **Language**: TypeScript
- **Runtime**: Node.js 22+
- **Web Scraping**: Playwright
- **Web Framework**: Express.js  
- **Database**: FileStore (JSON files) - *Replaced Prisma for simplicity*
- **Scheduler**: node-cron
- **Container**: Docker with Alpine Linux
- **Testing**: Vitest

## File Structure (Current Implementation)

```
price-tracker/
├── src/
│   ├── core/                   # Core business logic
│   │   ├── filestore.ts        # ✅ Flat file database implementation
│   │   ├── database.ts         # ✅ Database service layer
│   │   ├── scraper.ts          # ✅ Playwright web scraping
│   │   ├── elementFinder.ts    # ✅ DOM element selection
│   │   ├── tracker.ts          # ✅ Main tracking orchestration
│   │   ├── productManager.ts   # ✅ Product CRUD operations
│   │   └── scheduler.ts        # ✅ Cron job scheduling
│   ├── plugins/                # Plugin system
│   │   ├── base/              # ✅ Base interfaces
│   │   ├── trackers/          # ✅ Price, Version, Number trackers
│   │   ├── notifiers/         # ✅ Email, Discord notifiers  
│   │   └── PluginManager.ts   # ✅ Plugin loading system
│   ├── web/                   # Web interface
│   │   ├── server.ts          # ✅ Express server
│   │   └── routes/            # ✅ API endpoints
│   ├── types/
│   │   └── models.ts          # ✅ TypeScript type definitions
│   └── utils/                 # ✅ Logger, config utilities
├── tests/                     # ✅ Test suite (updated for FileStore)
├── scripts/                   # Migration scripts
│   └── migrate-from-prisma.ts # ✅ Prisma to FileStore migration
├── data/                      # JSON data files (created at runtime)
├── Dockerfile                 # ✅ Updated (Prisma removed)
├── package.json              # ✅ Updated (Prisma removed)
└── docker-compose.yml
```

## Data Models

**FileStore Implementation** (see `src/types/models.ts`)

Core entities stored as JSON files in `./data/`:
- **Product** - Trackable items with multiple sources
- **Source** - URLs to track for each product  
- **PriceHistory** - Historical value changes
- **NotificationConfig** - Per-product notification settings
- **PriceComparison** - Cross-source price comparisons
- **NotificationLog** - Notification delivery tracking
- **SystemSettings** - Application configuration

## Plugin System

**Extensible Architecture** (see `src/plugins/base/`)

- **TrackerPlugin** - Base class for tracking different value types (price, version, number)
- **NotifierPlugin** - Base class for notification delivery methods (email, Discord)
- **PluginManager** - Handles plugin loading and discovery

**Implemented Plugins:**
- **Trackers**: PriceTracker, VersionTracker, NumberTracker
- **Notifiers**: EmailNotifier, DiscordNotifier

## Development & Deployment

### Quick Start

```bash
# Install dependencies
npm install

# Run development server  
npm run dev

# Build for production
npm run build

# Run tests
npm test

# Migrate existing Prisma data (if needed)
npm run migrate
```

### Docker Deployment

```bash
# Build and run with Docker
docker build -t price-tracker .
docker run -p 3000:3000 -v ./data:/data price-tracker

# Or use Docker Compose
docker-compose up -d
```

### Environment Configuration

Key environment variables (see `.env.example`):

```bash
# Database & Server
DATABASE_URL=file:/data/tracker.db
PORT=3000
BASE_URL=http://localhost:3000

# Notifications  
SMTP_HOST=smtp.gmail.com
SMTP_USER=your-email@gmail.com
SMTP_PASS=your-app-password
DISCORD_WEBHOOK=https://discord.com/api/webhooks/...

# Scraping
MAX_CONCURRENT_CHECKS=5
RETRY_ATTEMPTS=3
```

## Key Architecture Decisions

### ✅ FileStore vs Prisma
- **Replaced** complex Prisma ORM with simple flat file JSON storage
- **Benefits**: No migrations, simpler deployment, better portability
- **Implementation**: In-memory caching with async file persistence

### ✅ Plugin System  
- **Extensible** tracker types (price, version, number) and notification methods
- **Easy to extend** with new plugins following base interfaces

### ✅ Multi-Source Tracking
- **Track products** across multiple URLs/stores
- **Compare prices** and identify best deals automatically
- **Consolidated notifications** with cross-source comparisons

---

*Database migration from Prisma available via `npm run migrate`*