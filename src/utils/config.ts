import { config as dotenvConfig } from 'dotenv';
import { z } from 'zod';

dotenvConfig();

const configSchema = z.object({
  // Server
  port: z.coerce.number().default(3000),
  baseUrl: z.string().default('http://localhost:3000'),
  secretKey: z.string().min(32, 'Secret key must be at least 32 characters'),
  nodeEnv: z.enum(['development', 'production', 'test']).default('development'),
  
  // Database
  databaseUrl: z.string().default('file:./tracker.db'),
  
  // Scraping
  maxConcurrentChecks: z.coerce.number().default(5),
  retryAttempts: z.coerce.number().default(3),
  retryDelay: z.coerce.number().default(5000),
  screenshotQuality: z.coerce.number().min(10).max(100).default(80),
  
  // Session
  sessionSecret: z.string().optional(),
  sessionMaxAge: z.coerce.number().default(86400000), // 24 hours
  
  // Email
  smtpHost: z.string().optional(),
  smtpPort: z.coerce.number().optional(),
  smtpSecure: z.coerce.boolean().optional(),
  smtpUser: z.string().optional(),
  smtpPass: z.string().optional(),
  smtpFrom: z.string().optional(),
  
  // Discord
  discordWebhook: z.string().url().optional(),
  
  // Logging
  logLevel: z.enum(['error', 'warn', 'info', 'debug']).default('info'),
  logFile: z.string().optional(),
});

function getConfig() {
  const rawConfig = {
    port: process.env.PORT,
    baseUrl: process.env.BASE_URL,
    secretKey: process.env.SECRET_KEY || 'your-secret-key-here-change-in-production',
    nodeEnv: process.env.NODE_ENV,
    databaseUrl: process.env.DATABASE_URL,
    maxConcurrentChecks: process.env.MAX_CONCURRENT_CHECKS,
    retryAttempts: process.env.RETRY_ATTEMPTS,
    retryDelay: process.env.RETRY_DELAY,
    screenshotQuality: process.env.SCREENSHOT_QUALITY,
    sessionSecret: process.env.SESSION_SECRET,
    sessionMaxAge: process.env.SESSION_MAX_AGE,
    smtpHost: process.env.SMTP_HOST,
    smtpPort: process.env.SMTP_PORT,
    smtpSecure: process.env.SMTP_SECURE,
    smtpUser: process.env.SMTP_USER,
    smtpPass: process.env.SMTP_PASS,
    smtpFrom: process.env.SMTP_FROM,
    discordWebhook: process.env.DISCORD_WEBHOOK,
    logLevel: process.env.LOG_LEVEL,
    logFile: process.env.LOG_FILE,
  };

  return configSchema.parse(rawConfig);
}

export const config = getConfig();
export type Config = z.infer<typeof configSchema>;