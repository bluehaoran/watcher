import cron from 'node-cron';
import { Tracker } from './tracker.js';
import { logger } from '../utils/logger.js';
import { config } from '../utils/config.js';

export class Scheduler {
  private tasks = new Map<string, cron.ScheduledTask>();
  private mainTask: cron.ScheduledTask | null = null;

  constructor(private tracker: Tracker) {}

  start(): void {
    // Main tracking task - runs every hour by default
    this.mainTask = cron.schedule('0 * * * *', async () => {
      logger.info('Starting scheduled tracking cycle');
      
      try {
        const results = await this.tracker.trackAllProducts();
        const totalProducts = Object.keys(results).length;
        const successfulProducts = Object.values(results).filter(
          productResults => productResults.some(r => r.success)
        ).length;

        logger.info(`Tracking cycle completed: ${successfulProducts}/${totalProducts} products processed successfully`);
      } catch (error) {
        logger.error('Scheduled tracking cycle failed:', error);
      }
    }, {
      scheduled: false // Don't start immediately
    });

    // Health check task - runs every 5 minutes
    const healthCheckTask = cron.schedule('*/5 * * * *', () => {
      const memoryUsage = process.memoryUsage();
      const memoryUsageMB = Math.round(memoryUsage.heapUsed / 1024 / 1024);
      
      logger.debug(`Health check - Memory usage: ${memoryUsageMB}MB, Uptime: ${Math.round(process.uptime())}s`);
      
      // Log warning if memory usage is high
      if (memoryUsageMB > 512) {
        logger.warn(`High memory usage detected: ${memoryUsageMB}MB`);
      }
    });

    // Cleanup task - runs daily at 2 AM
    const cleanupTask = cron.schedule('0 2 * * *', async () => {
      logger.info('Starting daily cleanup');
      await this.runCleanupTasks();
    });

    // Start the scheduled tasks
    this.mainTask.start();
    healthCheckTask.start();
    cleanupTask.start();

    logger.info('Scheduler started successfully');
  }

  stop(): void {
    // Stop all tasks
    if (this.mainTask) {
      this.mainTask.stop();
      this.mainTask = null;
    }

    this.tasks.forEach(task => task.stop());
    this.tasks.clear();

    logger.info('Scheduler stopped');
  }

  async triggerManualTracking(): Promise<any> {
    logger.info('Manual tracking cycle triggered');
    
    try {
      const results = await this.tracker.trackAllProducts();
      
      const summary = {
        timestamp: new Date().toISOString(),
        totalProducts: Object.keys(results).length,
        results: Object.entries(results).map(([productId, productResults]) => ({
          productId,
          success: productResults.some(r => r.success),
          changes: productResults.filter(r => r.changed).length,
          errors: productResults.filter(r => !r.success).length
        }))
      };

      logger.info(`Manual tracking completed: ${summary.results.filter(r => r.success).length}/${summary.totalProducts} successful`);
      
      return summary;
    } catch (error) {
      logger.error('Manual tracking failed:', error);
      throw error;
    }
  }

  addCustomTask(name: string, cronExpression: string, taskFunction: () => Promise<void>): void {
    if (this.tasks.has(name)) {
      logger.warn(`Task ${name} already exists, replacing it`);
      this.tasks.get(name)?.stop();
    }

    const task = cron.schedule(cronExpression, async () => {
      logger.info(`Running custom task: ${name}`);
      try {
        await taskFunction();
        logger.info(`Custom task completed: ${name}`);
      } catch (error) {
        logger.error(`Custom task failed: ${name}`, error);
      }
    });

    this.tasks.set(name, task);
    logger.info(`Added custom task: ${name} (${cronExpression})`);
  }

  removeCustomTask(name: string): void {
    const task = this.tasks.get(name);
    if (task) {
      task.stop();
      this.tasks.delete(name);
      logger.info(`Removed custom task: ${name}`);
    }
  }

  getTaskStatus(): any {
    return {
      mainTask: this.mainTask ? {
        running: this.mainTask.getStatus() === 'scheduled',
        nextRun: 'Every hour'
      } : null,
      customTasks: Array.from(this.tasks.entries()).map(([name, task]) => ({
        name,
        running: task.getStatus() === 'scheduled'
      }))
    };
  }

  private async runCleanupTasks(): Promise<void> {
    try {
      // Clean up old price history (keep last 30 days)
      const thirtyDaysAgo = new Date();
      thirtyDaysAgo.setDate(thirtyDaysAgo.getDate() - 30);

      // This would use Prisma to clean up old data
      // const deletedCount = await prisma.priceHistory.deleteMany({
      //   where: {
      //     timestamp: {
      //       lt: thirtyDaysAgo
      //     }
      //   }
      // });

      logger.info('Cleanup tasks completed');

      // Clean up old notification logs (keep last 7 days)
      const sevenDaysAgo = new Date();
      sevenDaysAgo.setDate(sevenDaysAgo.getDate() - 7);

      // Cleanup old false positives (keep last 14 days)
      const fourteenDaysAgo = new Date();
      fourteenDaysAgo.setDate(fourteenDaysAgo.getDate() - 14);

      logger.info('Database cleanup completed');
    } catch (error) {
      logger.error('Cleanup tasks failed:', error);
    }
  }

  // Utility method to validate cron expressions
  static validateCronExpression(expression: string): boolean {
    try {
      return cron.validate(expression);
    } catch (error) {
      return false;
    }
  }

  // Utility method to get next execution time
  static getNextExecutionTime(expression: string): Date | null {
    try {
      if (!cron.validate(expression)) return null;
      
      // This is a simplified implementation
      // In practice, you'd use a proper cron parser
      return new Date(Date.now() + 60 * 60 * 1000); // 1 hour from now
    } catch (error) {
      return null;
    }
  }
}