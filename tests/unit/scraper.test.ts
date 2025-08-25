import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { WebScraper } from '@/core/scraper';
import { resetAllMocks } from '@tests/mocks';

// Mock Playwright - define at top level
const mockPage = {
  goto: vi.fn(),
  content: vi.fn(),
  screenshot: vi.fn(),
  title: vi.fn(),
  evaluate: vi.fn(),
  close: vi.fn(),
  waitForLoadState: vi.fn(),
  setUserAgent: vi.fn(),
  setViewportSize: vi.fn(),
  waitForSelector: vi.fn(),
  locator: vi.fn(),
};

const mockContext = {
  newPage: vi.fn(),
  close: vi.fn(),
};

const mockBrowser = {
  newContext: vi.fn(),
  close: vi.fn(),
};

// Set up the mock returns
mockContext.newPage.mockResolvedValue(mockPage);
mockBrowser.newContext.mockResolvedValue(mockContext);

vi.mock('playwright', () => ({
  chromium: {
    launch: vi.fn(() => Promise.resolve(mockBrowser)),
  },
}));

describe('WebScraper', () => {
  let webScraper: WebScraper;

  beforeEach(async () => {
    resetAllMocks();
    webScraper = new WebScraper();
    await webScraper.initialize();
  });

  afterEach(async () => {
    await webScraper.close();
  });

  describe('initialization', () => {
    it('should initialize browser successfully', async () => {
      expect(mockBrowser.newContext).toHaveBeenCalled();
    });

    it('should handle initialization failure', async () => {
      const failingScraper = new WebScraper();
      
      // Mock browser launch failure
      const { chromium } = await import('playwright');
      vi.mocked(chromium.launch).mockRejectedValueOnce(new Error('Browser launch failed'));

      await expect(failingScraper.initialize()).rejects.toThrow('Browser launch failed');
    });
  });

  describe('scrape', () => {
    beforeEach(() => {
      mockPage.goto.mockResolvedValue(undefined);
      mockPage.content.mockResolvedValue('<html><body><div class="price">$99.99</div></body></html>');
      mockPage.title.mockResolvedValue('Test Product Page');
      mockPage.screenshot.mockResolvedValue(Buffer.from('screenshot'));
      mockPage.evaluate.mockResolvedValue('$99.99');
      mockPage.waitForSelector.mockResolvedValue({});
    });

    it('should scrape page successfully with selector', async () => {
      const result = await webScraper.scrape('https://example.com/product', '.price');

      expect(result.success).toBe(true);
      expect(result.content).toBe('$99.99');
      expect(result.title).toBe('Test Product Page');
      expect(result.screenshot).toBeDefined();

      expect(mockPage.goto).toHaveBeenCalledWith('https://example.com/product', {
        waitUntil: 'networkidle',
        timeout: 30000,
      });
      expect(mockPage.waitForSelector).toHaveBeenCalledWith('.price', { timeout: 10000 });
      expect(mockPage.evaluate).toHaveBeenCalled();
    });

    it('should scrape page successfully without selector', async () => {
      mockPage.content.mockResolvedValue('<html><body><h1>Product Page</h1></body></html>');

      const result = await webScraper.scrape('https://example.com/product');

      expect(result.success).toBe(true);
      expect(result.content).toContain('Product Page');
      expect(result.title).toBe('Test Product Page');

      expect(mockPage.goto).toHaveBeenCalled();
      expect(mockPage.waitForSelector).not.toHaveBeenCalled();
    });

    it('should handle invalid URL', async () => {
      const result = await webScraper.scrape('invalid-url');

      expect(result.success).toBe(false);
      expect(result.error).toContain('Invalid URL');
    });

    it('should handle navigation timeout', async () => {
      mockPage.goto.mockRejectedValue(new Error('Navigation timeout'));

      const result = await webScraper.scrape('https://example.com/timeout');

      expect(result.success).toBe(false);
      expect(result.error).toContain('Navigation timeout');
    });

    it('should handle selector not found', async () => {
      mockPage.waitForSelector.mockRejectedValue(new Error('Selector not found'));

      const result = await webScraper.scrape('https://example.com/product', '.nonexistent');

      expect(result.success).toBe(false);
      expect(result.error).toContain('Element not found');
    });

    it('should handle page crash', async () => {
      mockPage.goto.mockRejectedValue(new Error('Page crashed'));

      const result = await webScraper.scrape('https://example.com/crash');

      expect(result.success).toBe(false);
      expect(result.error).toContain('Page crashed');
    });

    it('should extract metadata correctly', async () => {
      mockPage.evaluate.mockResolvedValueOnce('$99.99'); // selector content
      mockPage.evaluate.mockResolvedValueOnce({ // metadata
        description: 'Product description',
        image: 'https://example.com/image.jpg',
        price: '$99.99',
      });

      const result = await webScraper.scrape('https://example.com/product', '.price');

      expect(result.success).toBe(true);
      expect(result.metadata).toEqual({
        description: 'Product description',
        image: 'https://example.com/image.jpg',
        price: '$99.99',
      });
    });

    it('should handle screenshot failure gracefully', async () => {
      mockPage.screenshot.mockRejectedValue(new Error('Screenshot failed'));

      const result = await webScraper.scrape('https://example.com/product', '.price');

      expect(result.success).toBe(true);
      expect(result.screenshot).toBeUndefined();
      // Should still succeed even if screenshot fails
    });

    it('should respect user agent setting', async () => {
      await webScraper.scrape('https://example.com/product');

      expect(mockPage.setUserAgent).toHaveBeenCalledWith(
        expect.stringContaining('Mozilla/5.0')
      );
    });

    it('should set appropriate viewport size', async () => {
      await webScraper.scrape('https://example.com/product');

      expect(mockPage.setViewportSize).toHaveBeenCalledWith({
        width: 1280,
        height: 720,
      });
    });

    it('should handle XPath selectors', async () => {
      mockPage.evaluate.mockResolvedValue('$99.99');

      const result = await webScraper.scrape('https://example.com/product', '//div[@class="price"]');

      expect(result.success).toBe(true);
      expect(result.content).toBe('$99.99');
    });

    it('should handle complex selectors', async () => {
      mockPage.evaluate.mockResolvedValue('$99.99');

      const result = await webScraper.scrape(
        'https://example.com/product', 
        '.product-details .price-section span.current-price'
      );

      expect(result.success).toBe(true);
      expect(result.content).toBe('$99.99');
    });

    it('should retry on temporary failures', async () => {
      mockPage.goto
        .mockRejectedValueOnce(new Error('Temporary network error'))
        .mockResolvedValueOnce(undefined);

      const result = await webScraper.scrape('https://example.com/product', '.price');

      expect(result.success).toBe(true);
      expect(mockPage.goto).toHaveBeenCalledTimes(2);
    });
  });

  describe('findElementsContainingText', () => {
    beforeEach(() => {
      mockPage.goto.mockResolvedValue(undefined);
      mockPage.evaluate.mockResolvedValue([
        { element: '.price1', text: '$99.99', html: '<span class="price1">$99.99</span>' },
        { element: '.price2', text: '99.99', html: '<span class="price2">99.99</span>' },
        { element: '.description', text: 'Price: $99.99', html: '<p class="description">Price: $99.99</p>' },
      ]);
    });

    it('should find elements containing text', async () => {
      const result = await webScraper.findElementsContainingText('https://example.com/product', '99.99');

      expect(result.success).toBe(true);
      expect(result.elements).toHaveLength(3);
      expect(result.elements[0]).toEqual({
        element: '.price1',
        text: '$99.99',
        html: '<span class="price1">$99.99</span>',
      });
    });

    it('should handle no elements found', async () => {
      mockPage.evaluate.mockResolvedValue([]);

      const result = await webScraper.findElementsContainingText('https://example.com/product', 'nonexistent');

      expect(result.success).toBe(true);
      expect(result.elements).toHaveLength(0);
    });

    it('should handle page errors during element search', async () => {
      mockPage.goto.mockRejectedValue(new Error('Page error'));

      const result = await webScraper.findElementsContainingText('https://example.com/product', '99.99');

      expect(result.success).toBe(false);
      expect(result.error).toContain('Page error');
    });
  });

  describe('close', () => {
    it('should close browser gracefully', async () => {
      await webScraper.close();

      expect(mockContext.close).toHaveBeenCalled();
      expect(mockBrowser.close).toHaveBeenCalled();
    });

    it('should handle close errors gracefully', async () => {
      mockBrowser.close.mockRejectedValue(new Error('Close failed'));

      // Should not throw
      await expect(webScraper.close()).resolves.toBeUndefined();
    });

    it('should be safe to call close multiple times', async () => {
      await webScraper.close();
      await webScraper.close();

      // Should not error on multiple calls
      expect(mockBrowser.close).toHaveBeenCalledTimes(2);
    });
  });

  describe('error handling and edge cases', () => {
    it('should handle malformed HTML gracefully', async () => {
      mockPage.content.mockResolvedValue('<html><body><div class="price">$99.99</div>'); // Missing closing tags
      mockPage.evaluate.mockResolvedValue('$99.99');

      const result = await webScraper.scrape('https://example.com/product', '.price');

      expect(result.success).toBe(true);
      expect(result.content).toBe('$99.99');
    });

    it('should handle empty page content', async () => {
      mockPage.content.mockResolvedValue('');
      
      const result = await webScraper.scrape('https://example.com/product');

      expect(result.success).toBe(true);
      expect(result.content).toBe('');
    });

    it('should handle JavaScript-heavy pages', async () => {
      mockPage.waitForLoadState.mockResolvedValue(undefined);
      mockPage.evaluate.mockResolvedValue('$99.99');

      const result = await webScraper.scrape('https://example.com/spa', '.price');

      expect(result.success).toBe(true);
      expect(mockPage.waitForLoadState).toHaveBeenCalledWith('networkidle');
    });
  });
});