import { db } from './database';
import { logger } from '../utils/logger';
import { PluginManager } from '../plugins/PluginManager';
import { 
  CreateProduct, 
  CreateSource, 
  CreateNotificationConfig, 
  UpdateProduct,
  ProductWithSources
} from '../types/models';

export interface CreateProductData {
  name: string;
  description?: string;
  trackerType: string;
  notifyOn: string;
  threshold?: any;
  checkInterval: string;
  sources: Array<{
    url: string;
    storeName?: string;
    title: string;
    selector: string;
    selectorType: string;
  }>;
  notifications: Array<{
    notifierType: string;
    config: any;
  }>;
}

export interface UpdateProductData {
  name?: string;
  description?: string;
  notifyOn?: string;
  threshold?: any;
  checkInterval?: string;
  isActive?: boolean;
  isPaused?: boolean;
}

export class ProductManager {
  constructor(private pluginManager: PluginManager) {}

  async createProduct(data: CreateProductData) {
    const { sources, notifications, ...productData } = data;

    try {
      return await db.transaction(async () => {
        // Create the product
        const product = await db.products.create({
          ...productData,
          isActive: true,
          isPaused: false,
        });

        // Create sources
        for (const sourceData of sources) {
          const { storeName, ...rest } = sourceData;
          await db.sources.create({
            ...rest,
            productId: product.id,
            storeName: storeName || this.extractDomainName(sourceData.url),
            errorCount: 0,
            isActive: true,
          });
        }

        // Create notification configs
        for (const notificationData of notifications) {
          await db.notifications.create({
            productId: product.id,
            notifierType: notificationData.notifierType,
            config: notificationData.config,
            isEnabled: true,
          });
        }

        logger.info(`Created product: ${product.name} (${product.id})`);
        return product;
      });

    } catch (error) {
      logger.error('Failed to create product:', error);
      throw error;
    }
  }

  async updateProduct(productId: string, data: UpdateProductData) {
    try {
      const product = await db.products.update(productId, data);
      
      if (!product) {
        throw new Error(`Product not found: ${productId}`);
      }

      logger.info(`Updated product: ${product.name} (${productId})`);
      return product;

    } catch (error) {
      logger.error(`Failed to update product ${productId}:`, error);
      throw error;
    }
  }

  async deleteProduct(productId: string) {
    try {
      await db.deleteProduct(productId);
      logger.info(`Deleted product: ${productId}`);
    } catch (error) {
      logger.error(`Failed to delete product ${productId}:`, error);
      throw error;
    }
  }

  async getProduct(productId: string): Promise<ProductWithSources | null> {
    try {
      return await db.getProductWithSources(productId);
    } catch (error) {
      logger.error(`Failed to get product ${productId}:`, error);
      throw error;
    }
  }

  async getProducts(options: {
    isActive?: boolean;
    trackerType?: string;
    limit?: number;
    offset?: number;
  } = {}): Promise<ProductWithSources[]> {
    try {
      const filter = (product: any) => {
        if (options.isActive !== undefined && product.isActive !== options.isActive) {
          return false;
        }
        if (options.trackerType && product.trackerType !== options.trackerType) {
          return false;
        }
        return true;
      };

      const products = await db.products.findMany(filter, {
        limit: options.limit,
        offset: options.offset,
        sortBy: 'createdAt',
        sortOrder: 'desc'
      });

      // Get full products with sources and notifications
      const result: ProductWithSources[] = [];
      for (const product of products) {
        const productWithSources = await db.getProductWithSources(product.id);
        if (productWithSources) {
          result.push(productWithSources);
        }
      }

      return result;
    } catch (error) {
      logger.error('Failed to get products:', error);
      throw error;
    }
  }

  async addSourceToProduct(productId: string, sourceData: {
    url: string;
    storeName?: string;
    selector: string;
    selectorType: string;
  }) {
    try {
      const source = await db.sources.create({
        ...sourceData,
        productId,
        storeName: sourceData.storeName || this.extractDomainName(sourceData.url),
        title: '', // Will be populated on first scrape
        errorCount: 0,
        isActive: true,
      });

      logger.info(`Added source ${source.url} to product ${productId}`);
      return source;

    } catch (error) {
      logger.error(`Failed to add source to product ${productId}:`, error);
      throw error;
    }
  }

  async updateSource(sourceId: string, data: {
    url?: string;
    storeName?: string;
    selector?: string;
    selectorType?: string;
    isActive?: boolean;
  }) {
    try {
      const source = await db.sources.update(sourceId, data);
      
      if (!source) {
        throw new Error(`Source not found: ${sourceId}`);
      }

      logger.info(`Updated source: ${sourceId}`);
      return source;

    } catch (error) {
      logger.error(`Failed to update source ${sourceId}:`, error);
      throw error;
    }
  }

  async removeSourceFromProduct(sourceId: string) {
    try {
      await db.deleteSource(sourceId);
      logger.info(`Removed source: ${sourceId}`);
    } catch (error) {
      logger.error(`Failed to remove source ${sourceId}:`, error);
      throw error;
    }
  }

  async updateBestDeal(productId: string) {
    try {
      const product = await db.getProductWithSources(productId);

      if (!product) {
        throw new Error(`Product not found: ${productId}`);
      }

      const tracker = this.pluginManager.getTracker(product.trackerType);
      if (!tracker) {
        throw new Error(`Tracker not found: ${product.trackerType}`);
      }

      // Find the best value among all sources
      let bestSource: any = null;
      let bestValue: any = null;

      for (const source of product.sources) {
        if (!source.currentValue || !source.isActive) continue;

        const currentValue = source.currentValue;
        
        if (!bestValue) {
          bestValue = currentValue;
          bestSource = source;
          continue;
        }

        const comparison = tracker.compare(bestValue, currentValue);
        
        // For prices, lower is better; for versions, higher is better
        if (product.trackerType === 'price') {
          if (comparison.changeType === 'increased') {
            // Current value is higher, keep best
          } else if (comparison.changeType === 'decreased') {
            // Current value is lower, update best
            bestValue = currentValue;
            bestSource = source;
          }
        } else {
          if (comparison.changeType === 'increased') {
            // Current value is higher, update best
            bestValue = currentValue;
            bestSource = source;
          }
        }
      }

      if (bestSource && bestValue) {
        await db.products.update(productId, {
          bestSourceId: bestSource.id,
          bestValue: bestValue,
        });

        // Create price comparison record
        await this.createPriceComparison(productId, product.sources);
      }

    } catch (error) {
      logger.error(`Failed to update best deal for product ${productId}:`, error);
      throw error;
    }
  }

  private async createPriceComparison(productId: string, sources: any[]) {
    const tracker = this.pluginManager.getTracker('price');
    if (!tracker) return;

    const sourcesData = sources
      .filter(s => s.currentValue && s.isActive)
      .map(s => ({
        sourceId: s.id,
        value: s.currentValue,
        storeName: s.storeName
      }));

    if (sourcesData.length < 2) return;

    // Find best, worst, and calculate average for price tracking
    const values = sourcesData.map(s => s.value.amount || 0).filter(v => v > 0);
    if (values.length === 0) return;

    const bestValue = Math.min(...values);
    const worstValue = Math.max(...values);
    const avgValue = values.reduce((sum, val) => sum + val, 0) / values.length;

    const bestSourceData = sourcesData.find(s => s.value.amount === bestValue);

    await db.comparisons.create({
      productId,
      sources: sourcesData,
      bestSourceId: bestSourceData?.sourceId || '',
      bestValue: { amount: bestValue },
      worstValue: { amount: worstValue },
      avgValue: { amount: avgValue },
      timestamp: new Date().toISOString(),
    });
  }

  private extractDomainName(url: string): string {
    try {
      const domain = new URL(url).hostname;
      return domain.replace('www.', '');
    } catch (error) {
      return 'Unknown Store';
    }
  }
}