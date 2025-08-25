import express from 'express';
import session from 'express-session';
import helmet from 'helmet';
import rateLimit from 'express-rate-limit';
import path from 'path';
import { logger } from '../utils/logger';
import { config } from '../utils/config';
import { setupRoutes } from './routes/setup';
import { productRoutes } from './routes/products';
import { sourceRoutes } from './routes/sources';
import { scannerRoutes } from './routes/scanner';
import { actionRoutes } from './routes/actions';

export function createServer() {
  const app = express();

  // Security middleware
  app.use(helmet({
    contentSecurityPolicy: {
      directives: {
        defaultSrc: ["'self'"],
        styleSrc: ["'self'", "'unsafe-inline'"],
        scriptSrc: ["'self'", "'unsafe-inline'"],
        imgSrc: ["'self'", "data:", "blob:"],
      },
    },
  }));

  // Rate limiting
  const limiter = rateLimit({
    windowMs: 15 * 60 * 1000, // 15 minutes
    max: 100, // Limit each IP to 100 requests per windowMs
    standardHeaders: true,
    legacyHeaders: false,
  });
  app.use(limiter);

  // Body parsing middleware
  app.use(express.json({ limit: '10mb' }));
  app.use(express.urlencoded({ extended: true, limit: '10mb' }));

  // Session middleware
  app.use(session({
    secret: config.sessionSecret || config.secretKey,
    resave: false,
    saveUninitialized: false,
    cookie: {
      secure: config.nodeEnv === 'production',
      maxAge: config.sessionMaxAge,
    },
  }));

  // Static files
  const publicPath = path.join(__dirname, '../../public');
  app.use('/static', express.static(publicPath));

  // View engine setup (basic HTML templates)
  app.set('view engine', 'html');
  app.set('views', path.join(__dirname, 'views'));
  app.engine('html', (filePath, options, callback) => {
    const fs = require('fs');
    fs.readFile(filePath, 'utf-8', callback);
  });

  // Health check endpoint
  app.get('/health', (req, res) => {
    res.json({ 
      status: 'healthy', 
      timestamp: new Date().toISOString(),
      uptime: process.uptime(),
      memory: process.memoryUsage(),
    });
  });

  // Routes
  app.use('/setup', setupRoutes);
  app.use('/products', productRoutes);
  app.use('/sources', sourceRoutes);
  app.use('/scanner', scannerRoutes);
  app.use('/actions', actionRoutes);

  // Root route
  app.get('/', (req, res) => {
    res.redirect('/products');
  });

  // API routes
  app.get('/api/health', (req, res) => {
    res.json({ status: 'ok', timestamp: new Date().toISOString() });
  });

  // Error handling middleware
  app.use((err: Error, req: express.Request, res: express.Response, next: express.NextFunction) => {
    logger.error('Express error:', err);
    
    if (res.headersSent) {
      return next(err);
    }

    const isDevelopment = config.nodeEnv === 'development';
    
    res.status(500).json({
      error: 'Internal Server Error',
      message: isDevelopment ? err.message : 'Something went wrong',
      stack: isDevelopment ? err.stack : undefined,
    });
  });

  // 404 handler
  app.use((req, res) => {
    res.status(404).json({
      error: 'Not Found',
      message: `Route ${req.method} ${req.path} not found`,
    });
  });

  return app;
}

export async function startServer() {
  const app = createServer();
  
  const server = app.listen(config.port, () => {
    logger.info(`Server running on port ${config.port}`);
    logger.info(`Environment: ${config.nodeEnv}`);
    logger.info(`Base URL: ${config.baseUrl}`);
  });

  // Graceful shutdown
  process.on('SIGTERM', () => {
    logger.info('SIGTERM received, shutting down gracefully');
    server.close(() => {
      logger.info('Process terminated');
      process.exit(0);
    });
  });

  process.on('SIGINT', () => {
    logger.info('SIGINT received, shutting down gracefully');
    server.close(() => {
      logger.info('Process terminated');
      process.exit(0);
    });
  });

  return server;
}