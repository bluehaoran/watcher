import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { WebScraper } from '@/core/scraper';
import { ScrapingResultBuilder, ElementMatchBuilder } from './builders/WebScraperTestBuilder';

// Mock external dependencies properly at boundaries
vi.mock('playwright', () => ({
  chromium: {
    launch: vi.fn()
  }
}));

vi.mock('@/utils/config', () => ({
  config: {
    baseUrl: 'http://localhost:3000',
    secretKey: 'test-secret-key-that-is-at-least-32-characters-long'
  }
}));

vi.mock('@/utils/logger', () => ({
  logger: {
    info: vi.fn(),
    error: vi.fn(),
    warn: vi.fn(),
    debug: vi.fn()
  }
}));

describe('WebScraper', () => {
  let webScraper: WebScraper;
  let mockBrowser: any;
  let mockContext: any;
  let mockPage: any;

  beforeEach(async () => {
    vi.clearAllMocks();

    // Create comprehensive mocks for Playwright
    mockPage = {
      goto: vi.fn().mockResolvedValue(undefined),
      content: vi.fn().mockResolvedValue('<html><body><div class="price">$99.99</div></body></html>'),
      screenshot: vi.fn().mockResolvedValue(Buffer.from('screenshot')),
      title: vi.fn().mockResolvedValue('Test Product Page'),
      evaluate: vi.fn().mockResolvedValue('$99.99'),
      close: vi.fn().mockResolvedValue(undefined),
      waitForLoadState: vi.fn().mockResolvedValue(undefined),
      setUserAgent: vi.fn().mockResolvedValue(undefined),
      setViewportSize: vi.fn().mockResolvedValue(undefined),
      waitForSelector: vi.fn().mockResolvedValue({}),
      waitForTimeout: vi.fn().mockResolvedValue(undefined),
      textContent: vi.fn().mockResolvedValue('Product Page'),
      locator: vi.fn().mockReturnValue({
        first: () => ({ textContent: vi.fn().mockResolvedValue('$99.99') }),
        textContent: vi.fn().mockResolvedValue('$99.99')
      })
    };

    mockContext = {
      newPage: vi.fn().mockResolvedValue(mockPage),
      close: vi.fn().mockResolvedValue(undefined)
    };

    mockBrowser = {
      newContext: vi.fn().mockResolvedValue(mockContext),
      close: vi.fn().mockResolvedValue(undefined)
    };

    // Mock Playwright launch
    const { chromium } = await import('playwright');
    vi.mocked(chromium.launch).mockResolvedValue(mockBrowser);

    webScraper = new WebScraper();
    await webScraper.initialize();
  });

  afterEach(async () => {
    if (webScraper) {
      await webScraper.close();
    }
  });

  describe('initialization', () => {
    it('should initialize browser successfully', async () => {
      expect(mockBrowser.newContext).toHaveBeenCalled();
    });

    it('should handle initialization failure gracefully', async () => {
      const failingScraper = new WebScraper();
      
      const { chromium } = await import('playwright');
      vi.mocked(chromium.launch).mockRejectedValueOnce(new Error('Browser launch failed'));

      await expect(failingScraper.initialize())
        .rejects
        .toThrow(/browser launch failed/i);
    });
  });

  describe('scrape', () => {
    it('should successfully scrape page with selector', async () => {
      // Arrange
      const expectedResult = new ScrapingResultBuilder()
        .withContent('$99.99')
        .withTitle('Test Product Page')
        .build();

      mockPage.locator.mockReturnValue({
        first: () => ({ textContent: vi.fn().mockResolvedValue('$99.99') }),
        textContent: vi.fn().mockResolvedValue('$99.99')
      });

      // Act
      const result = await webScraper.scrape('https://example.com/product', '.price');

      // Assert - focus on successful outcome
      expect(result.success).toBe(true);
      expect(result.content).toBe('$99.99');
      expect(result.title).toBe('Test Product Page');
      expect(result.screenshot).toBeDefined();

      // Verify essential browser interactions
      expect(mockPage.goto).toHaveBeenCalledWith(
        'https://example.com/product',
        expect.objectContaining({
          waitUntil: 'domcontentloaded',
          timeout: 30000
        })
      );
      expect(mockPage.waitForSelector).toHaveBeenCalledWith('.price', expect.any(Object));
    });

    it('should successfully scrape page without selector', async () => {
      // Arrange
      mockPage.textContent.mockResolvedValue('Product Page');

      // Act
      const result = await webScraper.scrape('https://example.com/product');

      // Assert
      expect(result.success).toBe(true);
      expect(result.content).toContain('Product Page');
      expect(result.title).toBe('Test Product Page');
      
      // Should not wait for selector when none provided
      expect(mockPage.waitForSelector).not.toHaveBeenCalled();
    });

    it('should handle invalid URLs gracefully', async () => {
      // Arrange - mock goto to reject with invalid URL error
      mockPage.goto.mockRejectedValue(new Error('Invalid URL'));

      // Act
      const result = await webScraper.scrape('invalid-url');

      // Assert - should fail with error message
      expect(result.success).toBe(false);
      expect(result.error).toContain('Invalid URL');
    });

    it('should handle navigation failures', async () => {
      // Arrange
      mockPage.goto.mockRejectedValue(new Error('Navigation timeout'));

      // Act
      const result = await webScraper.scrape('https://example.com/timeout');

      // Assert
      expect(result.success).toBe(false);
      expect(result.error).toContain('Navigation timeout');
    });

    it('should handle selector not found gracefully', async () => {
      // Arrange - selector timeout results in empty content, not failure
      mockPage.waitForSelector.mockRejectedValue(new Error('Selector not found'));
      mockPage.locator.mockReturnValue({
        first: () => ({ textContent: vi.fn().mockResolvedValue(null) }),
        textContent: vi.fn().mockResolvedValue(null)
      });

      // Act
      const result = await webScraper.scrape('https://example.com/product', '.nonexistent');

      // Assert - should succeed with empty content (implementation handles selector not found gracefully)
      expect(result.success).toBe(true);
      expect(result.content).toBe('');
    });

    it('should include metadata with url, selector, and timestamp', async () => {
      // Arrange
      mockPage.locator.mockReturnValue({
        first: () => ({ textContent: vi.fn().mockResolvedValue('$99.99') }),
        textContent: vi.fn().mockResolvedValue('$99.99')
      });

      // Act
      const result = await webScraper.scrape('https://example.com/product', '.price');

      // Assert - metadata contains url, selector, timestamp (actual behavior)
      expect(result.success).toBe(true);
      expect(result.metadata).toEqual(
        expect.objectContaining({
          url: 'https://example.com/product',
          selector: '.price',
          timestamp: expect.any(String)
        })
      );
    });

    it('should fail when screenshot fails', async () => {
      // Arrange
      mockPage.screenshot.mockRejectedValue(new Error('Screenshot failed'));
      mockPage.locator.mockReturnValue({
        first: () => ({ textContent: vi.fn().mockResolvedValue('$99.99') }),
        textContent: vi.fn().mockResolvedValue('$99.99')
      });

      // Act
      const result = await webScraper.scrape('https://example.com/product', '.price');

      // Assert - should fail because screenshot is required in implementation
      expect(result.success).toBe(false);
      expect(result.error).toContain('Screenshot failed');
    });

    it('should use browser context configuration', async () => {
      // Act
      await webScraper.scrape('https://example.com/product');

      // Assert - verify page creation from context (user agent/viewport set at context level)
      expect(mockContext.newPage).toHaveBeenCalled();
    });

    it('should handle different selector types', async () => {
      // Arrange
      mockPage.locator.mockReturnValue({
        first: () => ({ textContent: vi.fn().mockResolvedValue('$99.99') }),
        textContent: vi.fn().mockResolvedValue('$99.99')
      });

      // Act - test XPath selector
      const result = await webScraper.scrape('https://example.com/product', '//div[@class="price"]');

      // Assert
      expect(result.success).toBe(true);
      expect(result.content).toBe('$99.99');
    });

    it('should fail on network errors without retry', async () => {
      // Arrange - goto fails
      mockPage.goto.mockRejectedValue(new Error('Temporary network error'));

      // Act
      const result = await webScraper.scrape('https://example.com/product', '.price');

      // Assert - should fail because no retry logic is implemented
      expect(result.success).toBe(false);
      expect(result.error).toContain('Temporary network error');
      expect(mockPage.goto).toHaveBeenCalledTimes(1);
    });
  });

  describe('findElements', () => {
    it('should find elements containing specified text', async () => {
      // Arrange
      const expectedElements = [
        {
          selector: '.price1',
          text: '$99.99',
          html: '<span class="price1">$99.99</span>',
          context: '<div><span class="price1">$99.99</span></div>'
        },
        {
          selector: '.price2', 
          text: '99.99',
          html: '<span class="price2">99.99</span>',
          context: '<div><span class="price2">99.99</span></div>'
        }
      ];

      mockPage.evaluate.mockResolvedValue(expectedElements);
      mockPage.locator.mockReturnValue({
        first: () => ({ boundingBox: vi.fn().mockResolvedValue({ x: 0, y: 0, width: 100, height: 20 }) })
      });

      // Act
      const result = await webScraper.findElements('https://example.com/product', '99.99');

      // Assert - should return array of elements
      expect(result).toHaveLength(2);
      expect(result[0].text).toBe('$99.99');
      expect(result[1].text).toBe('99.99');
    });

    it('should handle no elements found', async () => {
      // Arrange
      mockPage.evaluate.mockResolvedValue([]);

      // Act
      const result = await webScraper.findElements('https://example.com/product', 'nonexistent');

      // Assert - should return empty array
      expect(result).toHaveLength(0);
    });

    it('should handle page errors during element search', async () => {
      // Arrange
      mockPage.goto.mockRejectedValue(new Error('Page error'));

      // Act & Assert - should throw error
      await expect(webScraper.findElements('https://example.com/product', '99.99'))
        .rejects
        .toThrow('Page error');
    });
  });

  describe('close', () => {
    it('should close browser resources gracefully', async () => {
      // Act
      await webScraper.close();

      // Assert - verify cleanup
      expect(mockContext.close).toHaveBeenCalled();
      expect(mockBrowser.close).toHaveBeenCalled();
    });

    it('should handle close errors gracefully', async () => {
      // Arrange
      mockBrowser.close.mockRejectedValue(new Error('Close failed'));

      // Act & Assert - should not throw
      await expect(webScraper.close()).resolves.toBeUndefined();
    });

    it('should be safe to call close multiple times', async () => {
      // Act
      await webScraper.close();
      await webScraper.close();

      // Assert - context/browser are set to null after first close
      expect(mockBrowser.close).toHaveBeenCalledTimes(1);
    });
  });

  describe('error handling and edge cases', () => {
    it('should handle malformed HTML gracefully', async () => {
      // Arrange
      mockPage.content.mockResolvedValue('<html><body><div class="price">$99.99</div>'); // Missing closing tags
      mockPage.locator.mockReturnValue({
        first: () => ({ textContent: vi.fn().mockResolvedValue('$99.99') }),
        textContent: vi.fn().mockResolvedValue('$99.99')
      });

      // Act
      const result = await webScraper.scrape('https://example.com/product', '.price');

      // Assert - should handle malformed HTML
      expect(result.success).toBe(true);
      expect(result.content).toBe('$99.99');
    });

    it('should handle empty page content', async () => {
      // Arrange
      mockPage.textContent.mockResolvedValue('');
      
      // Act
      const result = await webScraper.scrape('https://example.com/product');

      // Assert
      expect(result.success).toBe(true);
      expect(result.content).toBe('');
    });

    it('should handle JavaScript-heavy pages with timeout', async () => {
      // Arrange
      mockPage.locator.mockReturnValue({
        first: () => ({ textContent: vi.fn().mockResolvedValue('$99.99') }),
        textContent: vi.fn().mockResolvedValue('$99.99')
      });

      // Act
      const result = await webScraper.scrape('https://example.com/spa', '.price');

      // Assert
      expect(result.success).toBe(true);
      expect(mockPage.waitForTimeout).toHaveBeenCalledWith(2000);
    });
  });
});