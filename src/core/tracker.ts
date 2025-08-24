import { prisma } from './database.js';
import { logger } from '../utils/logger.js';
import { WebScraper } from './scraper.js';
import { PluginManager } from '../plugins/PluginManager.js';
import { ProductManager } from './productManager.js';
import { NotificationEvent } from '../plugins/base/NotifierPlugin.js';
import { config } from '../utils/config.js';

export interface TrackingResult {
  success: boolean;
  sourceId: string;
  oldValue?: any;
  newValue?: any;
  changed: boolean;
  error?: string;
  notifications?: string[];
}

export class Tracker {
  private scraper: WebScraper;
  
  constructor(
    private pluginManager: PluginManager,
    private productManager: ProductManager
  ) {
    this.scraper = new WebScraper();
  }

  async initialize() {
    await this.scraper.initialize();
    logger.info('Tracker initialized');
  }

  async trackProduct(productId: string): Promise<TrackingResult[]> {
    const product = await this.productManager.getProduct(productId);
    if (!product) {
      throw new Error(`Product not found: ${productId}`);
    }

    if (!product.isActive || product.isPaused) {
      logger.info(`Skipping inactive/paused product: ${product.name}`);
      return [];
    }

    const tracker = this.pluginManager.getTracker(product.trackerType);
    if (!tracker) {
      throw new Error(`Tracker not found for type: ${product.trackerType}`);
    }

    const results: TrackingResult[] = [];

    // Track each source
    for (const source of product.sources) {
      if (!source.isActive) {
        logger.info(`Skipping inactive source: ${source.url}`);
        continue;
      }

      try {
        const result = await this.trackSource(source, tracker, product);
        results.push(result);

        if (result.success) {
          // Update last checked time
          await prisma.source.update({
            where: { id: source.id },
            data: {
              lastChecked: new Date(),
              errorCount: 0,
              lastError: null,
            }
          });
        }

      } catch (error) {
        logger.error(`Failed to track source ${source.url}:`, error);
        
        // Update error count
        const errorCount = source.errorCount + 1;
        await prisma.source.update({
          where: { id: source.id },
          data: {
            errorCount,
            lastError: error instanceof Error ? error.message : String(error),
            lastChecked: new Date(),
          }
        });

        // Disable source if too many errors
        if (errorCount >= config.retryAttempts) {
          await prisma.source.update({
            where: { id: source.id },
            data: { isActive: false }
          });
          
          logger.warn(`Disabled source due to repeated errors: ${source.url}`);
        }

        results.push({
          success: false,
          sourceId: source.id,
          changed: false,
          error: error instanceof Error ? error.message : String(error)
        });
      }
    }

    // Update product's last checked time and next check time
    await prisma.product.update({
      where: { id: productId },
      data: {
        lastChecked: new Date(),
        nextCheck: this.calculateNextCheck(product.checkInterval),
      }
    });

    // Update best deal across sources
    await this.productManager.updateBestDeal(productId);

    // Send notifications if there were changes
    await this.processNotifications(product, results);

    return results;
  }

  async trackAllProducts(): Promise<{ [productId: string]: TrackingResult[] }> {
    const products = await this.productManager.getProducts({ isActive: true });
    const results: { [productId: string]: TrackingResult[] } = {};

    logger.info(`Starting tracking cycle for ${products.length} products`);

    for (const product of products) {
      // Check if it's time to check this product
      if (product.nextCheck && product.nextCheck > new Date()) {
        logger.debug(`Skipping product ${product.name} - not due for check yet`);
        continue;
      }

      try {
        results[product.id] = await this.trackProduct(product.id);
      } catch (error) {
        logger.error(`Failed to track product ${product.id}:`, error);
        results[product.id] = [{
          success: false,
          sourceId: '',
          changed: false,
          error: error instanceof Error ? error.message : String(error)
        }];
      }

      // Small delay between products to avoid overwhelming sites
      await this.delay(1000);
    }

    logger.info(`Tracking cycle completed. Processed ${Object.keys(results).length} products`);
    return results;
  }

  private async trackSource(source: any, tracker: any, product: any): Promise<TrackingResult> {
    const scrapeResult = await this.scraper.scrape(source.url, source.selector);
    
    if (!scrapeResult.success) {
      throw new Error(`Failed to scrape ${source.url}: ${scrapeResult.error}`);
    }

    // Update page title if empty
    if (!source.title && scrapeResult.title) {
      await prisma.source.update({
        where: { id: source.id },
        data: { title: scrapeResult.title }
      });
    }

    const parseResult = tracker.parse(scrapeResult.content || '');
    
    if (!parseResult.success) {
      throw new Error(`Failed to parse content from ${source.url}: low confidence or invalid format`);
    }

    const oldValue = source.currentValue ? JSON.parse(source.currentValue) : null;
    const newValue = parseResult.value;

    // Store the new value and text
    await prisma.source.update({
      where: { id: source.id },
      data: {
        currentValue: JSON.stringify(newValue),
        currentText: scrapeResult.content,
        originalValue: oldValue ? undefined : JSON.stringify(newValue),
        originalText: source.originalText || scrapeResult.content,
      }
    });

    // Record in history
    await prisma.priceHistory.create({
      data: {
        sourceId: source.id,
        value: JSON.stringify(newValue),
        text: scrapeResult.content || '',
      }
    });

    // Check if value changed
    let changed = false;
    if (oldValue) {
      const comparison = tracker.compare(oldValue, newValue);
      changed = comparison.changed;
    }

    return {
      success: true,
      sourceId: source.id,
      oldValue,
      newValue,
      changed,
      notifications: changed ? ['change_detected'] : []
    };
  }

  private async processNotifications(product: any, results: TrackingResult[]) {
    const changedSources = results.filter(r => r.success && r.changed);
    
    if (changedSources.length === 0) return;

    const tracker = this.pluginManager.getTracker(product.trackerType);
    if (!tracker) return;

    // Check if we should notify based on the product's notification rules
    const shouldNotify = this.shouldNotify(product, changedSources, tracker);
    if (!shouldNotify) return;

    // Build notification event
    const event = await this.buildNotificationEvent(product, changedSources, tracker);
    
    // Send notifications
    for (const notificationConfig of product.notifications) {
      if (!notificationConfig.isEnabled) continue;

      const notifier = this.pluginManager.getNotifier(notificationConfig.notifierType);
      if (!notifier) continue;

      try {
        const notifierConfig = JSON.parse(notificationConfig.config);
        await notifier.initialize(notifierConfig);
        
        const result = await notifier.notify(event);
        
        // Log the notification
        await prisma.notificationLog.create({
          data: {
            productId: product.id,
            type: notificationConfig.notifierType,
            status: result.success ? 'sent' : 'failed',
            error: result.error || null,
          }
        });

        if (result.success) {
          logger.info(`Sent ${notificationConfig.notifierType} notification for product ${product.name}`);
        } else {
          logger.error(`Failed to send ${notificationConfig.notifierType} notification:`, result.error);
        }

      } catch (error) {
        logger.error(`Failed to send notification for product ${product.id}:`, error);
      }
    }
  }

  private shouldNotify(product: any, changedSources: TrackingResult[], tracker: any): boolean {
    const notifyOn = product.notifyOn || 'any_change';
    
    if (notifyOn === 'any_change') {
      return true;
    }

    // Check threshold-based notifications
    for (const sourceResult of changedSources) {
      if (!sourceResult.oldValue || !sourceResult.newValue) continue;

      const comparison = tracker.compare(sourceResult.oldValue, sourceResult.newValue);
      
      if (notifyOn === 'decrease' && comparison.changeType === 'decreased') {
        if (this.meetsThreshold(product.threshold, comparison)) {
          return true;
        }
      }
      
      if (notifyOn === 'increase' && comparison.changeType === 'increased') {
        if (this.meetsThreshold(product.threshold, comparison)) {
          return true;
        }
      }
    }

    return false;
  }

  private meetsThreshold(threshold: any, comparison: any): boolean {
    if (!threshold) return true;

    const thresholdConfig = typeof threshold === 'string' ? JSON.parse(threshold) : threshold;
    if (!thresholdConfig || !thresholdConfig.value) return true;

    if (thresholdConfig.type === 'relative') {
      return comparison.percentChange >= thresholdConfig.value;
    } else {
      return comparison.difference >= thresholdConfig.value;
    }
  }

  private async buildNotificationEvent(product: any, changedSources: TrackingResult[], tracker: any): Promise<NotificationEvent> {
    const sources = await prisma.source.findMany({
      where: { productId: product.id, isActive: true }
    });

    const baseUrl = config.baseUrl;
    
    const actionUrls = {
      dismiss: `${baseUrl}/actions/dismiss/${product.id}`,
      falsePositive: `${baseUrl}/actions/false-positive/${product.id}`,
      purchased: `${baseUrl}/actions/purchased/${product.id}`,
      viewProduct: `${baseUrl}/products/${product.id}`
    };

    // If multiple sources, build comparison data
    if (sources.length > 1) {
      const allSources = sources
        .filter(s => s.currentValue)
        .map(s => {
          const value = JSON.parse(s.currentValue);
          const changed = changedSources.some(cs => cs.sourceId === s.id);
          
          return {
            sourceId: s.id,
            storeName: s.storeName,
            value,
            formattedValue: tracker.format(value),
            url: s.url,
            changed
          };
        });

      // Find best deal
      const bestSource = allSources.reduce((best, current) => {
        if (!best) return current;
        
        const comparison = tracker.compare(best.value, current.value);
        
        // For prices, lower is better
        if (product.trackerType === 'price') {
          return comparison.changeType === 'increased' ? current : best;
        } else {
          return comparison.changeType === 'increased' ? best : current;
        }
      });

      // Calculate savings for price tracking
      let savings;
      if (product.trackerType === 'price' && bestSource) {
        const worstPrice = Math.max(...allSources.map(s => s.value.amount || 0));
        const bestPrice = bestSource.value.amount || 0;
        
        if (worstPrice > bestPrice) {
          savings = {
            amount: worstPrice - bestPrice,
            percentage: ((worstPrice - bestPrice) / worstPrice) * 100
          };
        }
      }

      // Use the first changed source for the main change data
      const primaryChange = changedSources[0];
      const comparison = tracker.compare(primaryChange.oldValue, primaryChange.newValue);

      return {
        product: {
          id: product.id,
          name: product.name
        },
        comparison: {
          best: bestSource,
          allSources,
          savings
        },
        changeType: comparison.changeType,
        oldValue: primaryChange.oldValue,
        newValue: primaryChange.newValue,
        formattedOld: tracker.format(primaryChange.oldValue),
        formattedNew: tracker.format(primaryChange.newValue),
        difference: tracker.format({ amount: comparison.difference }),
        threshold: product.threshold ? JSON.parse(product.threshold) : undefined,
        actionUrls
      };

    } else {
      // Single source notification
      const source = sources[0];
      const sourceResult = changedSources.find(cs => cs.sourceId === source.id);
      
      if (!sourceResult) {
        throw new Error('No changed source found for single source product');
      }

      const comparison = tracker.compare(sourceResult.oldValue, sourceResult.newValue);

      return {
        product: {
          id: product.id,
          name: product.name
        },
        source: {
          id: source.id,
          url: source.url,
          storeName: source.storeName
        },
        changeType: comparison.changeType,
        oldValue: sourceResult.oldValue,
        newValue: sourceResult.newValue,
        formattedOld: tracker.format(sourceResult.oldValue),
        formattedNew: tracker.format(sourceResult.newValue),
        difference: tracker.format({ amount: comparison.difference }),
        threshold: product.threshold ? JSON.parse(product.threshold) : undefined,
        actionUrls
      };
    }
  }

  private calculateNextCheck(cronExpression: string): Date {
    // Simple implementation - in a real app you'd use a proper cron parser
    const now = new Date();
    
    // Default to 24 hours from now
    const nextCheck = new Date(now.getTime() + 24 * 60 * 60 * 1000);
    
    return nextCheck;
  }

  private delay(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms));
  }

  async close() {
    await this.scraper.close();
    logger.info('Tracker closed');
  }
}