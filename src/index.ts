import { config } from './utils/config';
import { logger } from './utils/logger';
import { connectDatabase } from './core/database';
import { PluginManager } from './plugins/PluginManager';
import { ProductManager } from './core/productManager';
import { Tracker } from './core/tracker';
import { Scheduler } from './core/scheduler';
import { startServer } from './web/server';
import { 
  initializeProductRoutes 
} from './web/routes/products';
import { 
  initializeSetupRoutes 
} from './web/routes/setup';
import {
  initializeSourceRoutes
} from './web/routes/sources';
import {
  initializeScannerRoutes
} from './web/routes/scanner';

class PriceTrackerApp {
  private pluginManager: PluginManager;
  private productManager: ProductManager;
  private tracker: Tracker;
  private scheduler: Scheduler;

  constructor() {
    this.pluginManager = new PluginManager();
    this.productManager = new ProductManager(this.pluginManager);
    this.tracker = new Tracker(this.pluginManager, this.productManager);
    this.scheduler = new Scheduler(this.tracker);
  }

  async initialize(): Promise<void> {
    try {
      logger.info('Starting Price Tracker application...');
      logger.info(`Node.js version: ${process.version}`);
      logger.info(`Environment: ${config.nodeEnv}`);

      // Connect to database
      await connectDatabase();

      // Load plugins
      await this.pluginManager.loadDefaultPlugins();
      logger.info(`Loaded ${this.pluginManager.getAvailableTrackers().length} trackers and ${this.pluginManager.getAvailableNotifiers().length} notifiers`);

      // Initialize tracker
      await this.tracker.initialize();

      // Initialize route handlers with dependencies
      initializeProductRoutes(this.productManager, this.pluginManager);
      initializeSetupRoutes(this.pluginManager);
      initializeSourceRoutes(this.productManager);
      initializeScannerRoutes(this.pluginManager);

      // Start web server
      const server = await startServer();

      // Start scheduler
      this.scheduler.start();

      logger.info('Price Tracker application started successfully');
      logger.info(`Web interface available at: ${config.baseUrl}`);

      // Graceful shutdown handling
      const shutdown = async (signal: string) => {
        logger.info(`${signal} received, shutting down gracefully...`);
        
        this.scheduler.stop();
        await this.tracker.close();
        
        server.close(() => {
          logger.info('Application shutdown complete');
          process.exit(0);
        });
      };

      process.on('SIGTERM', () => shutdown('SIGTERM'));
      process.on('SIGINT', () => shutdown('SIGINT'));

    } catch (error) {
      logger.error('Failed to initialize application:', error);
      process.exit(1);
    }
  }

  async runManualCheck(): Promise<void> {
    try {
      logger.info('Running manual tracking check...');
      const results = await this.scheduler.triggerManualTracking();
      logger.info('Manual check completed:', results);
    } catch (error) {
      logger.error('Manual check failed:', error);
    }
  }

  getStatus(): any {
    return {
      application: 'Price Tracker',
      version: '1.0.0',
      nodeVersion: process.version,
      uptime: process.uptime(),
      environment: config.nodeEnv,
      plugins: {
        trackers: this.pluginManager.getAvailableTrackers().length,
        notifiers: this.pluginManager.getAvailableNotifiers().length,
      },
      scheduler: this.scheduler.getTaskStatus(),
      memory: process.memoryUsage(),
    };
  }
}

// CLI handling
async function main() {
  const app = new PriceTrackerApp();

  const command = process.argv[2];
  
  switch (command) {
    case 'start':
    case undefined:
      await app.initialize();
      break;
      
    case 'check':
      await app.initialize();
      await app.runManualCheck();
      process.exit(0);
      break;
      
    case 'status':
      await app.initialize();
      console.log(JSON.stringify(app.getStatus(), null, 2));
      process.exit(0);
      break;
      
    case 'help':
      console.log(`
Price Tracker - Multi-source price and version tracking application

Usage:
  node dist/index.js [command]

Commands:
  start     Start the application (default)
  check     Run a manual tracking check
  status    Show application status
  help      Show this help message

Environment Variables:
  PORT                  Web server port (default: 3000)
  DATABASE_URL          SQLite database path (default: file:./tracker.db)
  BASE_URL              Base URL for the application (default: http://localhost:3000)
  SECRET_KEY            Secret key for sessions and security
  NODE_ENV              Environment (development/production)
  
  SMTP_HOST             SMTP server host for email notifications
  SMTP_PORT             SMTP server port (default: 587)
  SMTP_USER             SMTP username
  SMTP_PASS             SMTP password
  SMTP_FROM             From email address
  
  DISCORD_WEBHOOK       Discord webhook URL for notifications
  
  MAX_CONCURRENT_CHECKS Maximum concurrent scraping operations (default: 5)
  RETRY_ATTEMPTS        Number of retry attempts for failed scrapes (default: 3)
  LOG_LEVEL            Logging level (error/warn/info/debug, default: info)

Examples:
  npm start                    # Start the application
  npm run dev                  # Start in development mode with auto-reload
  node dist/index.js check     # Run manual tracking check
  node dist/index.js status    # Show status

Web Interface:
  http://localhost:3000        # Dashboard
  http://localhost:3000/setup  # Setup wizard
  http://localhost:3000/scanner # Element scanner tool

For more information, visit: https://github.com/your-repo/price-tracker
      `);
      break;
      
    default:
      console.error(`Unknown command: ${command}`);
      console.error('Use "help" to see available commands');
      process.exit(1);
  }
}

// Handle uncaught errors
process.on('uncaughtException', (error) => {
  logger.error('Uncaught exception:', error);
  process.exit(1);
});

process.on('unhandledRejection', (reason, promise) => {
  logger.error('Unhandled rejection at:', promise, 'reason:', reason);
  process.exit(1);
});

// Start the application
main().catch((error) => {
  console.error('Failed to start application:', error);
  process.exit(1);
});