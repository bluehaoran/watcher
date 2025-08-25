import { FileStore } from './filestore';
import { logger } from '../utils/logger';
import {
  Product,
  Source,
  PriceHistory,
  NotificationConfig,
  PriceComparison,
  FalsePositive,
  NotificationLog,
  SystemSettings,
  ProductWithSources,
  ProductSummary
} from '../types/models';

export class Database {
  public products: FileStore<Product>;
  public sources: FileStore<Source>;
  public history: FileStore<PriceHistory>;
  public notifications: FileStore<NotificationConfig>;
  public comparisons: FileStore<PriceComparison>;
  public falsePositives: FileStore<FalsePositive>;
  public notificationLogs: FileStore<NotificationLog>;
  public settings: FileStore<SystemSettings>;

  constructor(dataDir = './data') {
    this.products = new FileStore<Product>('products', dataDir);
    this.sources = new FileStore<Source>('sources', dataDir);
    this.history = new FileStore<PriceHistory>('history', dataDir);
    this.notifications = new FileStore<NotificationConfig>('notifications', dataDir);
    this.comparisons = new FileStore<PriceComparison>('comparisons', dataDir);
    this.falsePositives = new FileStore<FalsePositive>('false-positives', dataDir);
    this.notificationLogs = new FileStore<NotificationLog>('notification-logs', dataDir);
    this.settings = new FileStore<SystemSettings>('settings', dataDir);
  }

  async init(): Promise<void> {
    try {
      await Promise.all([
        this.products.init(),
        this.sources.init(),
        this.history.init(),
        this.notifications.init(),
        this.comparisons.init(),
        this.falsePositives.init(),
        this.notificationLogs.init(),
        this.settings.init(),
      ]);
      logger.info('Database initialized successfully');
    } catch (error) {
      logger.error('Failed to initialize database:', error);
      throw error;
    }
  }

  async close(): Promise<void> {
    try {
      await Promise.all([
        this.products.flush(),
        this.sources.flush(),
        this.history.flush(),
        this.notifications.flush(),
        this.comparisons.flush(),
        this.falsePositives.flush(),
        this.notificationLogs.flush(),
        this.settings.flush(),
      ]);
      logger.info('Database closed successfully');
    } catch (error) {
      logger.error('Failed to close database:', error);
      throw error;
    }
  }

  // Helper methods for common queries
  async getProductWithSources(productId: string): Promise<ProductWithSources | null> {
    const product = await this.products.findById(productId);
    if (!product) return null;

    const [sources, notifications] = await Promise.all([
      this.sources.findMany(s => s.productId === productId),
      this.notifications.findMany(n => n.productId === productId)
    ]);

    return { ...product, sources, notifications };
  }

  async getProductsWithSources(): Promise<ProductWithSources[]> {
    const products = await this.products.findMany();
    const result: ProductWithSources[] = [];

    for (const product of products) {
      const [sources, notifications] = await Promise.all([
        this.sources.findMany(s => s.productId === product.id),
        this.notifications.findMany(n => n.productId === product.id)
      ]);
      result.push({ ...product, sources, notifications });
    }

    return result;
  }

  async getProductSummaries(): Promise<ProductSummary[]> {
    const products = await this.products.findMany();
    const summaries: ProductSummary[] = [];

    for (const product of products) {
      const sources = await this.sources.findMany(s => s.productId === product.id);
      const bestSource = product.bestSourceId 
        ? sources.find(s => s.id === product.bestSourceId)
        : null;

      summaries.push({
        id: product.id,
        name: product.name,
        trackerType: product.trackerType,
        isActive: product.isActive,
        isPaused: product.isPaused,
        lastChecked: product.lastChecked,
        sourceCount: sources.length,
        bestValue: product.bestValue,
        bestStoreName: bestSource?.storeName
      });
    }

    return summaries;
  }

  async getSourceHistory(sourceId: string, limit = 50): Promise<PriceHistory[]> {
    const history = await this.history.findMany(
      h => h.sourceId === sourceId,
      { sortBy: 'timestamp', sortOrder: 'desc', limit }
    );
    return history;
  }

  async getActiveProducts(): Promise<Product[]> {
    return this.products.findMany(p => p.isActive && !p.isPaused);
  }

  async getProductsByType(trackerType: string): Promise<Product[]> {
    return this.products.findMany(p => p.trackerType === trackerType);
  }

  async getSourcesForProduct(productId: string): Promise<Source[]> {
    return this.sources.findMany(s => s.productId === productId);
  }

  async getActiveSourcesForProduct(productId: string): Promise<Source[]> {
    return this.sources.findMany(s => s.productId === productId && s.isActive);
  }

  async getNotificationConfigsForProduct(productId: string): Promise<NotificationConfig[]> {
    return this.notifications.findMany(n => n.productId === productId && n.isEnabled);
  }

  async deleteProduct(productId: string): Promise<void> {
    // Delete in correct order to handle foreign key relationships
    await Promise.all([
      this.history.deleteMany(h => {
        // Find sources for this product first
        return this.sources.all().some(s => s.productId === productId && s.id === h.sourceId);
      }),
      this.falsePositives.deleteMany(fp => {
        // Find sources for this product first
        return this.sources.all().some(s => s.productId === productId && s.id === fp.sourceId);
      }),
      this.sources.deleteMany(s => s.productId === productId),
      this.notifications.deleteMany(n => n.productId === productId),
      this.comparisons.deleteMany(c => c.productId === productId),
      this.notificationLogs.deleteMany(nl => nl.productId === productId)
    ]);

    await this.products.delete(productId);
  }

  async deleteSource(sourceId: string): Promise<void> {
    await Promise.all([
      this.history.deleteMany(h => h.sourceId === sourceId),
      this.falsePositives.deleteMany(fp => fp.sourceId === sourceId)
    ]);

    await this.sources.delete(sourceId);
  }

  // Settings helpers
  async getSetting<T>(key: string, defaultValue?: T): Promise<T | undefined> {
    const setting = await this.settings.findFirst(s => s.key === key);
    return setting ? setting.value : defaultValue;
  }

  async setSetting<T>(key: string, value: T): Promise<void> {
    const existing = await this.settings.findFirst(s => s.key === key);
    if (existing) {
      await this.settings.update(existing.id, { value });
    } else {
      await this.settings.create({ key, value });
    }
  }

  // Transaction wrapper
  async transaction<T>(fn: (db: Database) => Promise<T>): Promise<T> {
    // Simple transaction implementation - could be enhanced with rollback support
    return await fn(this);
  }
}

// Global database instance
export const db = new Database(process.env.DATA_DIR || './data');

// Legacy exports for compatibility during migration
export const prisma = {
  $connect: () => db.init(),
  $disconnect: () => db.close(),
};

export async function connectDatabase() {
  try {
    await db.init();
    logger.info('Database connected successfully');
  } catch (error) {
    logger.error('Failed to connect to database:', error);
    throw error;
  }
}

export async function disconnectDatabase() {
  try {
    await db.close();
    logger.info('Database disconnected');
  } catch (error) {
    logger.error('Failed to disconnect from database:', error);
    throw error;
  }
}