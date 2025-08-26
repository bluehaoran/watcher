# Price Tracker

A containerized price/version/number tracking application that monitors web pages for changes and sends notifications. Built with Node.js 22, TypeScript, Playwright, and a lightweight FileStore database.

## Features

- **Multi-Source Tracking**: Track products across multiple websites
- **Plugin Architecture**: Price, version, and number tracking plugins
- **Smart Element Detection**: Visual selector with confidence ranking
- **Notifications**: Email (SMTP) and Discord webhooks
- **Web Interface**: Dashboard, setup wizard, element scanner
- **Docker Ready**: Easy deployment with Docker Compose

## Quick Start

### Docker (Recommended)
```bash
git clone <repository-url>
cd price-tracker
cp .env.example .env
# Edit .env with your settings
docker-compose up -d
```

### Local Development
```bash
git clone <repository-url>
cd price-tracker
./test-setup.sh  # Automated setup script
npm run dev
```

Access at http://localhost:3000

## Usage

1. **Add Product**: Visit dashboard → "Add Product"
2. **Configure Sources**: Add URLs to track
3. **Select Elements**: Use visual picker at `/scanner`
4. **Set Notifications**: Configure email/Discord alerts
5. **Schedule**: Set check intervals

## Development

```bash
npm run dev        # Start with hot reload
npm run build      # Build for production
npm start          # Start production server
npm test           # Run tests
```

## Configuration

Key environment variables:
- `SECRET_KEY`: Application security key (required)
- `DATA_DIR`: FileStore data directory (default: `./data`)
- `SMTP_*`: Email notification settings
- `DISCORD_WEBHOOK`: Discord notification URL

See `.env.example` for complete options.

## API Testing

```bash
# Health check
curl http://localhost:3000/health

# Create product
curl -X POST http://localhost:3000/products \
  -H "Content-Type: application/json" \
  -d @test-product.json

# Test element scanner
curl -X POST http://localhost:3000/scanner/scan \
  -H "Content-Type: application/json" \
  -d '{"url":"https://example.com","searchText":"test"}'
```

## Troubleshooting

**Common Issues:**
- Port in use: Change `PORT` in `.env`
- Data issues: `rm -rf data/*.json` to reset FileStore
- Playwright issues: `npx playwright install chromium`
- Memory issues: `export NODE_OPTIONS="--max-old-space-size=1024"`

**Debug Mode:**
```bash
LOG_LEVEL=debug npm run dev
docker-compose logs -f price-tracker  # Docker logs
```

## CLI Commands

```bash
node dist/index.js status  # Show status
node dist/index.js check   # Manual tracking
node dist/index.js help    # Show help
```

## Contributing

1. Fork repository
2. Create feature branch
3. Ensure GPL v3 compliance
4. Submit pull request

## License

GNU General Public License v3.0 - see [LICENSE](LICENSE).

**Key implications:**
- ✅ Free to use, modify, distribute
- ✅ Commercial use allowed with compliance
- ✅ Source code must remain available
- ❌ Cannot incorporate into proprietary software

---

**Built with Node.js 22, TypeScript, Playwright, FileStore flat file database, and modern web technologies.**