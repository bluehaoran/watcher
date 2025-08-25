import { prisma } from './database';
import { logger } from '../utils/logger';
import { PluginManager } from '../plugins/PluginManager';

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
      const product = await prisma.$transaction(async (tx) => {
        // Create the product
        const newProduct = await tx.product.create({
          data: {
            ...productData,
            threshold: data.threshold ? JSON.stringify(data.threshold) : undefined,
          }
        });

        // Create sources
        for (const sourceData of sources) {
          const { storeName, ...rest } = sourceData;
          await tx.source.create({
            data: {
              ...rest,
              productId: newProduct.id,
              storeName: storeName || this.extractDomainName(sourceData.url),
              title: '', // Will be populated on first scrape
            }
          });
        }

        // Create notification configs
        for (const notificationData of notifications) {
          await tx.notificationConfig.create({
            data: {
              productId: newProduct.id,
              notifierType: notificationData.notifierType,
              config: JSON.stringify(notificationData.config),
            }
          });
        }

        return newProduct;
      });

      logger.info(`Created product: ${product.name} (${product.id})`);
      return product;

    } catch (error) {
      logger.error('Failed to create product:', error);
      throw error;
    }
  }

  async updateProduct(productId: string, data: UpdateProductData) {
    try {
      const product = await prisma.product.update({
        where: { id: productId },
        data: {
          ...data,
          threshold: data.threshold ? JSON.stringify(data.threshold) : undefined,
          updatedAt: new Date(),
        }
      });

      logger.info(`Updated product: ${product.name} (${productId})`);
      return product;

    } catch (error) {
      logger.error(`Failed to update product ${productId}:`, error);
      throw error;
    }
  }

  async deleteProduct(productId: string) {
    try {
      await prisma.product.delete({
        where: { id: productId }
      });

      logger.info(`Deleted product: ${productId}`);

    } catch (error) {
      logger.error(`Failed to delete product ${productId}:`, error);
      throw error;
    }
  }

  async getProduct(productId: string) {
    try {
      return await prisma.product.findUnique({
        where: { id: productId },
        include: {
          sources: true,
          notifications: true,
          comparisons: {
            orderBy: { timestamp: 'desc' },
            take: 1
          }
        }
      });
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
  } = {}) {
    try {
      const where: any = {};
      
      if (options.isActive !== undefined) {
        where.isActive = options.isActive;
      }
      
      if (options.trackerType) {
        where.trackerType = options.trackerType;
      }

      return await prisma.product.findMany({
        where,
        include: {
          sources: true,
          notifications: true,
          comparisons: {
            orderBy: { timestamp: 'desc' },
            take: 1
          }
        },
        take: options.limit,
        skip: options.offset,
        orderBy: { createdAt: 'desc' }
      });

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
      const source = await prisma.source.create({
        data: {
          ...sourceData,
          productId,
          storeName: sourceData.storeName || this.extractDomainName(sourceData.url),
          title: '', // Will be populated on first scrape
        }
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
      const source = await prisma.source.update({
        where: { id: sourceId },
        data: {
          ...data,
          updatedAt: new Date(),
        }
      });

      logger.info(`Updated source: ${sourceId}`);
      return source;

    } catch (error) {
      logger.error(`Failed to update source ${sourceId}:`, error);
      throw error;
    }
  }

  async removeSourceFromProduct(sourceId: string) {
    try {
      await prisma.source.delete({
        where: { id: sourceId }
      });

      logger.info(`Removed source: ${sourceId}`);

    } catch (error) {
      logger.error(`Failed to remove source ${sourceId}:`, error);
      throw error;
    }
  }

  async updateBestDeal(productId: string) {
    try {
      const product = await prisma.product.findUnique({
        where: { id: productId },
        include: { sources: true }
      });

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

        const currentValue = JSON.parse(source.currentValue as string);
        
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
        await prisma.product.update({
          where: { id: productId },
          data: {
            bestSourceId: bestSource.id,
            bestValue: JSON.stringify(bestValue),
          }
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
        value: JSON.parse(s.currentValue),
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

    await prisma.priceComparison.create({
      data: {
        productId,
        sources: JSON.stringify(sourcesData),
        bestSourceId: bestSourceData?.sourceId || '',
        bestValue: JSON.stringify({ amount: bestValue }),
        worstValue: JSON.stringify({ amount: worstValue }),
        avgValue: JSON.stringify({ amount: avgValue }),
      }
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