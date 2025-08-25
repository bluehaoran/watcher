import { chromium, Browser, BrowserContext } from 'playwright';
import { logger } from '../utils/logger';
import { config } from '../utils/config';

export interface ScrapeResult {
  success: boolean;
  content?: string;
  screenshot?: string;
  title?: string;
  error?: string;
  metadata?: Record<string, any>;
}

export interface ElementInfo {
  selector: string;
  text: string;
  html: string;
  context: string;
  boundingBox?: {
    x: number;
    y: number;
    width: number;
    height: number;
  };
}

export class WebScraper {
  private browser: Browser | null = null;
  private context: BrowserContext | null = null;

  async initialize(): Promise<void> {
    try {
      this.browser = await chromium.launch({
        headless: true,
        args: [
          '--no-sandbox',
          '--disable-setuid-sandbox',
          '--disable-dev-shm-usage',
          '--disable-gpu',
        ]
      });

      this.context = await this.browser.newContext({
        viewport: { width: 1280, height: 720 },
        userAgent: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
        ignoreHTTPSErrors: true,
      });

      logger.info('Web scraper initialized');
    } catch (error) {
      logger.error('Failed to initialize web scraper:', error);
      throw error;
    }
  }

  async scrape(url: string, selector?: string): Promise<ScrapeResult> {
    if (!this.context) {
      throw new Error('Scraper not initialized');
    }

    const page = await this.context.newPage();
    
    try {
      logger.info(`Scraping URL: ${url}`);

      // Set longer timeout for complex pages
      await page.goto(url, { 
        waitUntil: 'domcontentloaded',
        timeout: 30000 
      });

      // Wait for potential dynamic content
      await page.waitForTimeout(2000);

      const title = await page.title();
      
      let content: string | undefined;
      let screenshot: string | undefined;

      if (selector) {
        try {
          await page.waitForSelector(selector, { timeout: 10000 });
          const element = page.locator(selector).first();
          content = await element.textContent() || '';
        } catch (error) {
          logger.warn(`Selector "${selector}" not found on ${url}`);
          content = '';
        }
      } else {
        content = await page.textContent('body') || '';
      }

      // Take screenshot for debugging
      screenshot = await page.screenshot({
        type: 'png',
        quality: config.screenshotQuality,
        fullPage: false,
      }).then(buffer => buffer.toString('base64'));

      await page.close();

      return {
        success: true,
        content,
        screenshot,
        title,
        metadata: {
          url,
          selector,
          timestamp: new Date().toISOString(),
        }
      };

    } catch (error) {
      logger.error(`Failed to scrape ${url}:`, error);
      await page.close();
      
      return {
        success: false,
        error: error instanceof Error ? error.message : String(error),
        metadata: {
          url,
          selector,
          timestamp: new Date().toISOString(),
        }
      };
    }
  }

  async findElements(url: string, searchText: string): Promise<ElementInfo[]> {
    if (!this.context) {
      throw new Error('Scraper not initialized');
    }

    const page = await this.context.newPage();
    
    try {
      await page.goto(url, { 
        waitUntil: 'domcontentloaded',
        timeout: 30000 
      });

      await page.waitForTimeout(2000);

      // Find elements containing the search text
      const elements = await page.evaluate((text) => {
        const walker = document.createTreeWalker(
          document.body,
          NodeFilter.SHOW_TEXT,
          {
            acceptNode: (node) => {
              return node.textContent?.includes(text) ? 
                NodeFilter.FILTER_ACCEPT : 
                NodeFilter.FILTER_SKIP;
            }
          }
        );

        const results: ElementInfo[] = [];
        let node;

        while (node = walker.nextNode()) {
          const element = node.parentElement;
          if (!element) continue;

          // Generate CSS selector
          const selector = generateSelector(element);
          
          // Get surrounding context
          const context = element.parentElement?.innerHTML || element.innerHTML;
          
          results.push({
            selector,
            text: node.textContent || '',
            html: element.outerHTML,
            context: context.substring(0, 500) + (context.length > 500 ? '...' : ''),
          });
        }

        return results;
      }, searchText);

      // Get bounding boxes for elements
      for (const elementInfo of elements) {
        try {
          const element = page.locator(elementInfo.selector).first();
          const box = await element.boundingBox();
          if (box) {
            elementInfo.boundingBox = box;
          }
        } catch (error) {
          // Element might not be unique or accessible
          logger.debug(`Could not get bounding box for ${elementInfo.selector}`);
        }
      }

      await page.close();
      return elements;

    } catch (error) {
      logger.error(`Failed to find elements on ${url}:`, error);
      await page.close();
      throw error;
    }
  }

  async close(): Promise<void> {
    try {
      if (this.context) {
        await this.context.close();
        this.context = null;
      }
      
      if (this.browser) {
        await this.browser.close();
        this.browser = null;
      }
      
      logger.info('Web scraper closed');
    } catch (error) {
      logger.error('Failed to close web scraper:', error);
    }
  }
}

// Helper function to generate CSS selector (simplified version)
function generateSelector(element: Element): string {
  if (element.id) {
    return `#${element.id}`;
  }

  if (element.className) {
    const classes = element.className.split(' ').filter(c => c.trim());
    if (classes.length > 0) {
      return `.${classes.join('.')}`;
    }
  }

  const tagName = element.tagName.toLowerCase();
  const parent = element.parentElement;
  
  if (!parent) {
    return tagName;
  }

  const siblings = Array.from(parent.children).filter(el => 
    el.tagName.toLowerCase() === tagName
  );

  if (siblings.length === 1) {
    return `${generateSelector(parent)} > ${tagName}`;
  }

  const index = siblings.indexOf(element) + 1;
  return `${generateSelector(parent)} > ${tagName}:nth-child(${index})`;
}