import { vi } from 'vitest';

// Mock data factories
export const mockProduct = {
  id: 'test-product-1',
  name: 'Test Product',
  description: 'Test Description',
  trackerType: 'price',
  notifyOn: 'any_change',
  threshold: null,
  checkInterval: '0 0 * * *',
  lastChecked: null,
  nextCheck: null,
  isActive: true,
  isPaused: false,
  bestSourceId: null,
  bestValue: null,
  createdAt: new Date(),
  updatedAt: new Date(),
  sources: [],
  notifications: [],
  comparisons: [],
};

export const mockSource = {
  id: 'test-source-1',
  productId: 'test-product-1',
  url: 'https://example.com/product',
  storeName: 'Test Store',
  title: 'Test Product Page',
  selector: '.price',
  selectorType: 'css',
  originalValue: null,
  currentValue: null,
  originalText: null,
  currentText: null,
  isActive: true,
  lastChecked: null,
  errorCount: 0,
  lastError: null,
  createdAt: new Date(),
  updatedAt: new Date(),
  history: [],
  falsePositives: [],
};

export const mockPriceValue = {
  amount: 99.99,
  currency: 'USD',
  formatted: '$99.99',
};

export const mockVersionValue = {
  version: '1.2.3',
  major: 1,
  minor: 2,
  patch: 3,
  prerelease: null,
  build: null,
};

export const mockScrapeResult = {
  success: true,
  content: '$99.99',
  screenshot: 'base64-screenshot-data',
  title: 'Product Page',
  metadata: {},
};

export const mockParseResult = {
  success: true,
  value: mockPriceValue,
  normalized: '99.99',
  confidence: 0.95,
  metadata: { currency: 'USD' },
};

export const mockComparisonResult = {
  changed: true,
  changeType: 'decreased' as const,
  difference: -5.00,
  percentChange: -4.76,
};

export const mockNotificationEvent = {
  product: {
    id: 'test-product-1',
    name: 'Test Product',
  },
  source: {
    id: 'test-source-1',
    url: 'https://example.com/product',
    storeName: 'Test Store',
  },
  changeType: 'decreased' as const,
  oldValue: { amount: 104.99, currency: 'USD' },
  newValue: { amount: 99.99, currency: 'USD' },
  formattedOld: '$104.99',
  formattedNew: '$99.99',
  difference: '-$5.00',
  threshold: undefined,
  actionUrls: {
    dismiss: 'http://localhost:3000/actions/dismiss/test-product-1',
    falsePositive: 'http://localhost:3000/actions/false-positive/test-product-1',
    purchased: 'http://localhost:3000/actions/purchased/test-product-1',
    viewProduct: 'http://localhost:3000/products/test-product-1',
  },
};

// Mock fetch for Discord webhook testing
export const mockFetch = vi.fn();
global.fetch = mockFetch;

// Helper functions for test setup
export function createMockPrismaClient() {
  return {
    product: {
      create: vi.fn(),
      findUnique: vi.fn(),
      findMany: vi.fn(),
      update: vi.fn(),
      delete: vi.fn(),
    },
    source: {
      create: vi.fn(),
      findUnique: vi.fn(),
      findMany: vi.fn(),
      update: vi.fn(),
      delete: vi.fn(),
    },
    priceHistory: {
      create: vi.fn(),
      findMany: vi.fn(),
    },
    notificationLog: {
      create: vi.fn(),
      findMany: vi.fn(),
    },
    $transaction: vi.fn(),
    $connect: vi.fn(),
    $disconnect: vi.fn(),
  };
}

export function resetAllMocks() {
  vi.clearAllMocks();
  mockFetch.mockReset();
}