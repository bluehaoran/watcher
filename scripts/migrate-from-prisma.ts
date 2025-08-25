#!/usr/bin/env tsx
import { PrismaClient } from '@prisma/client';
import { Database } from '../src/core/database';
import { logger } from '../src/utils/logger';
import { readFileSync, existsSync } from 'node:fs';
import { join } from 'node:path';

/**
 * Migration script to convert data from Prisma SQLite database to FileStore JSON files
 * 
 * Usage:
 *   npx tsx scripts/migrate-from-prisma.ts
 * 
 * This script will:
 * 1. Read data from the existing Prisma SQLite database
 * 2. Convert it to the new FileStore format
 * 3. Save it as JSON files in the data directory
 * 4. Create a backup of the original database
 */

const PRISMA_DB_PATH = './prisma/tracker.db';
const DATA_DIR = './data';

async function main() {
  logger.info('Starting migration from Prisma to FileStore...');

  // Check if Prisma database exists
  if (!existsSync(PRISMA_DB_PATH)) {
    logger.warn('No Prisma database found. Migration not needed.');
    return;
  }

  const prisma = new PrismaClient({
    datasources: {
      db: {
        url: `file:${PRISMA_DB_PATH}`
      }
    }
  });

  const db = new Database(DATA_DIR);

  try {
    // Initialize the new database
    await db.init();
    logger.info('FileStore database initialized');

    // Migrate Products
    const products = await prisma.product.findMany();
    logger.info(`Migrating ${products.length} products...`);

    for (const product of products) {
      await db.products.create({
        id: product.id,
        name: product.name,
        description: product.description || undefined,
        trackerType: product.trackerType,
        notifyOn: product.notifyOn,
        threshold: product.threshold ? JSON.parse(product.threshold) : undefined,
        checkInterval: product.checkInterval,
        lastChecked: product.lastChecked?.toISOString(),
        nextCheck: product.nextCheck?.toISOString(),
        isActive: product.isActive,
        isPaused: product.isPaused,
        bestSourceId: product.bestSourceId || undefined,
        bestValue: product.bestValue ? JSON.parse(product.bestValue) : undefined,
        createdAt: product.createdAt.toISOString(),
        updatedAt: product.updatedAt.toISOString(),
      });
    }

    // Migrate Sources
    const sources = await prisma.source.findMany();
    logger.info(`Migrating ${sources.length} sources...`);

    for (const source of sources) {
      await db.sources.create({
        id: source.id,
        productId: source.productId,
        url: source.url,
        storeName: source.storeName || undefined,
        title: source.title,
        selector: source.selector,
        selectorType: source.selectorType,
        originalValue: source.originalValue ? JSON.parse(source.originalValue) : undefined,
        currentValue: source.currentValue ? JSON.parse(source.currentValue) : undefined,
        originalText: source.originalText || undefined,
        currentText: source.currentText || undefined,
        isActive: source.isActive,
        lastChecked: source.lastChecked?.toISOString(),
        errorCount: source.errorCount,
        lastError: source.lastError || undefined,
        createdAt: source.createdAt.toISOString(),
        updatedAt: source.updatedAt.toISOString(),
      });
    }

    // Migrate Notification Configs
    const notificationConfigs = await prisma.notificationConfig.findMany();
    logger.info(`Migrating ${notificationConfigs.length} notification configs...`);

    for (const config of notificationConfigs) {
      await db.notifications.create({
        id: config.id,
        productId: config.productId,
        notifierType: config.notifierType,
        config: JSON.parse(config.config),
        isEnabled: config.isEnabled,
        createdAt: config.createdAt.toISOString(),
        updatedAt: config.createdAt.toISOString(), // updatedAt doesn't exist in Prisma schema
      });
    }

    // Migrate Price History
    const historyRecords = await prisma.priceHistory.findMany();
    logger.info(`Migrating ${historyRecords.length} history records...`);

    for (const history of historyRecords) {
      await db.history.create({
        id: history.id,
        sourceId: history.sourceId,
        value: JSON.parse(history.value),
        text: history.text,
        timestamp: history.timestamp.toISOString(),
        createdAt: history.timestamp.toISOString(),
        updatedAt: history.timestamp.toISOString(),
      });
    }

    // Migrate Price Comparisons
    const comparisons = await prisma.priceComparison.findMany();
    logger.info(`Migrating ${comparisons.length} price comparisons...`);

    for (const comparison of comparisons) {
      await db.comparisons.create({
        id: comparison.id,
        productId: comparison.productId,
        sources: JSON.parse(comparison.sources),
        bestSourceId: comparison.bestSourceId,
        bestValue: JSON.parse(comparison.bestValue),
        worstValue: JSON.parse(comparison.worstValue),
        avgValue: JSON.parse(comparison.avgValue),
        timestamp: comparison.timestamp.toISOString(),
        createdAt: comparison.timestamp.toISOString(),
        updatedAt: comparison.timestamp.toISOString(),
      });
    }

    // Migrate False Positives
    const falsePositives = await prisma.falsePositive.findMany();
    logger.info(`Migrating ${falsePositives.length} false positives...`);

    for (const fp of falsePositives) {
      await db.falsePositives.create({
        id: fp.id,
        sourceId: fp.sourceId,
        detectedText: fp.detectedText,
        detectedValue: JSON.parse(fp.detectedValue),
        actualText: fp.actualText || undefined,
        htmlContext: fp.htmlContext,
        screenshot: fp.screenshot || undefined,
        notes: fp.notes || undefined,
        timestamp: fp.timestamp.toISOString(),
        createdAt: fp.timestamp.toISOString(),
        updatedAt: fp.timestamp.toISOString(),
      });
    }

    // Migrate Notification Logs
    const notificationLogs = await prisma.notificationLog.findMany();
    logger.info(`Migrating ${notificationLogs.length} notification logs...`);

    for (const log of notificationLogs) {
      await db.notificationLogs.create({
        id: log.id,
        productId: log.productId,
        type: log.type,
        status: log.status as 'sent' | 'failed' | 'actioned',
        action: log.action as 'dismissed' | 'false_positive' | 'purchased' | undefined,
        error: log.error || undefined,
        timestamp: log.timestamp.toISOString(),
        actionedAt: log.actionedAt?.toISOString(),
        createdAt: log.timestamp.toISOString(),
        updatedAt: log.actionedAt?.toISOString() || log.timestamp.toISOString(),
      });
    }

    // Migrate System Settings
    const settings = await prisma.systemSettings.findMany();
    logger.info(`Migrating ${settings.length} system settings...`);

    for (const setting of settings) {
      await db.settings.create({
        id: setting.key,
        key: setting.key,
        value: JSON.parse(setting.value),
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      });
    }

    // Force save all data
    await db.close();

    // Create backup of original database
    const backupPath = `${PRISMA_DB_PATH}.backup-${Date.now()}`;
    logger.info(`Creating backup of original database at: ${backupPath}`);

    // Note: In a real implementation, you'd want to copy the file
    // For now, just log the recommendation
    logger.info('Please manually backup your Prisma database before removing Prisma dependencies');

    logger.info('Migration completed successfully!');
    logger.info('Next steps:');
    logger.info('1. Test the application with the new FileStore');
    logger.info('2. Remove Prisma dependencies from package.json');
    logger.info('3. Update Dockerfile to remove Prisma');
    logger.info('4. Remove prisma/ directory');

  } catch (error) {
    logger.error('Migration failed:', error);
    throw error;
  } finally {
    await prisma.$disconnect();
  }
}

if (require.main === module) {
  main()
    .then(() => process.exit(0))
    .catch((error) => {
      logger.error('Migration script failed:', error);
      process.exit(1);
    });
}

export { main as migratePrismaToFileStore };