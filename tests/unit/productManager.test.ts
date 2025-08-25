import { describe, it, expect, beforeEach, vi } from 'vitest';
import { ProductManager } from '@/core/productManager';
import { PluginManager } from '@/plugins/PluginManager';
import { ProductTestDataBuilder, CreateProductDataBuilder, SourceTestDataBuilder } from './builders/ProductTestBuilder';

// Mock database - focus on behavior, not implementation details
vi.mock('@/core/database', () => ({
  db: {
    products: {
      create: vi.fn(),
      findById: vi.fn(),
      findMany: vi.fn(),
      update: vi.fn(),
      delete: vi.fn(),
    },
    sources: {
      create: vi.fn(),
      findById: vi.fn(),
      findMany: vi.fn(),
      update: vi.fn(),
      delete: vi.fn(),
    },
    history: {
      create: vi.fn(),
      findMany: vi.fn(),
    },
    notifications: {
      create: vi.fn(),
    },
    comparisons: {
      create: vi.fn(),
    },
    getProductWithSources: vi.fn(),
    getProductsWithSources: vi.fn(),
    deleteProduct: vi.fn(),
    deleteSource: vi.fn(),
    transaction: vi.fn(),
  },
}));

// Mock logger to avoid noise in tests
vi.mock('@/utils/logger', () => ({
  logger: {
    info: vi.fn(),
    error: vi.fn(),
    warn: vi.fn(),
    debug: vi.fn()
  }
}));

describe('ProductManager', () => {
  let productManager: ProductManager;
  let mockPluginManager: PluginManager;
  let mockDb: any;

  beforeEach(async () => {
    vi.clearAllMocks();
    
    const { db } = await import('@/core/database');
    mockDb = vi.mocked(db);
    
    // Mock plugin manager with realistic behavior
    mockPluginManager = {
      getTracker: vi.fn().mockReturnValue({
        name: 'Price Tracker',
        type: 'price',
        parse: vi.fn().mockReturnValue({ success: true, value: { amount: 99.99, currency: 'USD' } }),
        format: vi.fn().mockReturnValue('$99.99'),
        compare: vi.fn().mockReturnValue({ changed: false, changeType: 'unchanged' }),
      }),
    } as any;
    
    productManager = new ProductManager(mockPluginManager);
  });

  describe('createProduct', () => {
    it('should create a product with sources successfully', async () => {
      // Arrange
      const createData = new CreateProductDataBuilder()
        .withName('Gaming Laptop')
        .withSource('https://store.com/laptop', '.price')
        .build();

      const expectedProduct = new ProductTestDataBuilder()
        .withName('Gaming Laptop')
        .withSource('https://store.com/laptop', '.price')
        .build();

      // Mock transaction behavior - focus on the outcome
      mockDb.transaction.mockImplementation(async (callback) => {
        return await callback();
      });
      
      mockDb.products.create.mockResolvedValue(expectedProduct);
      mockDb.sources.create.mockResolvedValue(expectedProduct.sources[0]);
      mockDb.notifications.create.mockResolvedValue({});

      // Act
      const result = await productManager.createProduct(createData);

      // Assert - focus on behavior and outcomes
      expect(result).toBeDefined();
      expect(result.name).toBe('Gaming Laptop');
      expect(result.trackerType).toBe('price');
      expect(mockDb.transaction).toHaveBeenCalled();
      expect(mockDb.products.create).toHaveBeenCalled();
      expect(mockDb.sources.create).toHaveBeenCalled();
    });

    it('should create product even with unknown tracker type (no validation implemented)', async () => {
      // Arrange
      const createData = new CreateProductDataBuilder()
        .withTrackerType('unknown-type')
        .build();

      const expectedProduct = new ProductTestDataBuilder()
        .withName('Test Product')
        .withTrackerType('unknown-type')
        .build();

      mockPluginManager.getTracker = vi.fn().mockReturnValue(null);

      mockDb.transaction.mockImplementation(async (callback) => {
        return await callback();
      });
      
      mockDb.products.create.mockResolvedValue(expectedProduct);
      mockDb.sources.create.mockResolvedValue({});
      mockDb.notifications.create.mockResolvedValue({});

      // Act - should succeed because validation is not implemented
      const result = await productManager.createProduct(createData);

      // Assert - focus on what actually happens
      expect(result).toBeDefined();
      expect(result.trackerType).toBe('unknown-type');
    });

    it('should create product with empty sources (no validation implemented)', async () => {
      // Arrange
      const createData = new CreateProductDataBuilder()
        .withEmptySources()
        .build();

      const expectedProduct = new ProductTestDataBuilder()
        .withSources([])
        .build();

      mockDb.transaction.mockImplementation(async (callback) => {
        return await callback();
      });
      
      mockDb.products.create.mockResolvedValue(expectedProduct);
      mockDb.sources.create.mockResolvedValue({});
      mockDb.notifications.create.mockResolvedValue({});

      // Act - should succeed because validation is not implemented
      const result = await productManager.createProduct(createData);

      // Assert
      expect(result).toBeDefined();
      expect(result.sources).toEqual([]);
    });

    it('should create product with duplicate URLs (no validation implemented)', async () => {
      // Arrange
      const createData = new CreateProductDataBuilder()
        .withDuplicateSource('https://example.com/product')
        .build();

      const expectedProduct = new ProductTestDataBuilder()
        .withSource('https://example.com/product')
        .withSource('https://example.com/product')
        .build();

      mockDb.transaction.mockImplementation(async (callback) => {
        return await callback();
      });
      
      mockDb.products.create.mockResolvedValue(expectedProduct);
      mockDb.sources.create.mockResolvedValue({});
      mockDb.notifications.create.mockResolvedValue({});

      // Act - should succeed because validation is not implemented
      const result = await productManager.createProduct(createData);

      // Assert
      expect(result).toBeDefined();
      expect(result.sources).toHaveLength(2);
    });
  });

  describe('getProduct', () => {
    it('should return product with sources and notifications', async () => {
      // Arrange
      const expectedProduct = new ProductTestDataBuilder()
        .withName('Test Product')
        .withSource('https://example.com/product')
        .build();

      mockDb.getProductWithSources.mockResolvedValue(expectedProduct);

      // Act
      const result = await productManager.getProduct('test-product-1');

      // Assert - focus on what we get back, not how it's fetched
      expect(result).toBeDefined();
      expect(result.name).toBe('Test Product');
      expect(result.sources).toBeDefined();
      expect(result.notifications).toBeDefined();
      expect(mockDb.getProductWithSources).toHaveBeenCalledWith('test-product-1');
    });

    it('should return null for non-existent product', async () => {
      // Arrange
      mockDb.getProductWithSources.mockResolvedValue(null);

      // Act
      const result = await productManager.getProduct('non-existent');

      // Assert
      expect(result).toBeNull();
    });
  });

  describe('getProducts', () => {
    it('should return all active products by default', async () => {
      // Arrange
      const expectedProducts = [
        new ProductTestDataBuilder().withName('Product 1').build(),
        new ProductTestDataBuilder().withId('test-product-2').withName('Product 2').build()
      ];

      mockDb.products.findMany.mockResolvedValue(expectedProducts);
      mockDb.getProductWithSources.mockImplementation((id) => 
        expectedProducts.find(p => p.id === id)
      );

      // Act
      const result = await productManager.getProducts();

      // Assert - test behavior, not exact parameters
      expect(result).toHaveLength(2);
      expect(result[0].name).toBe('Product 1');
      expect(result[1].name).toBe('Product 2');
      expect(mockDb.products.findMany).toHaveBeenCalledWith(
        expect.any(Function),
        expect.objectContaining({
          sortBy: 'createdAt',
          sortOrder: 'desc'
        })
      );
    });

    it('should filter by active status when provided', async () => {
      // Arrange
      const inactiveProducts = [
        new ProductTestDataBuilder().withActiveStatus(false).build()
      ];

      mockDb.products.findMany.mockResolvedValue(inactiveProducts);
      mockDb.getProductWithSources.mockImplementation((id) => 
        inactiveProducts.find(p => p.id === id)
      );

      // Act
      const result = await productManager.getProducts({ isActive: false });

      // Assert
      expect(result).toHaveLength(1);
      expect(result[0].isActive).toBe(false);
      expect(mockDb.products.findMany).toHaveBeenCalledWith(
        expect.any(Function),
        expect.any(Object)
      );
    });

    it('should filter by tracker type when provided', async () => {
      // Arrange
      const versionProducts = [
        new ProductTestDataBuilder().withTrackerType('version').build()
      ];

      mockDb.products.findMany.mockResolvedValue(versionProducts);
      mockDb.getProductWithSources.mockImplementation((id) => 
        versionProducts.find(p => p.id === id)
      );

      // Act
      const result = await productManager.getProducts({ trackerType: 'version' });

      // Assert
      expect(result).toHaveLength(1);
      expect(result[0].trackerType).toBe('version');
      expect(mockDb.products.findMany).toHaveBeenCalledWith(
        expect.any(Function),
        expect.any(Object)
      );
    });
  });

  describe('updateProduct', () => {
    it('should update product successfully', async () => {
      // Arrange
      const updateData = { name: 'Updated Product Name' };
      const updatedProduct = new ProductTestDataBuilder()
        .withName('Updated Product Name')
        .build();

      mockDb.products.update.mockResolvedValue(updatedProduct);

      // Act
      const result = await productManager.updateProduct('test-product-1', updateData);

      // Assert - focus on the outcome
      expect(result).toBeDefined();
      expect(result.name).toBe('Updated Product Name');
      expect(mockDb.products.update).toHaveBeenCalledWith(
        'test-product-1',
        expect.objectContaining(updateData)
      );
    });

    it('should update product with unknown tracker type (no validation implemented)', async () => {
      // Arrange
      const updateData = { trackerType: 'invalid-type' };
      const updatedProduct = new ProductTestDataBuilder()
        .withTrackerType('invalid-type')
        .build();
      
      mockPluginManager.getTracker = vi.fn().mockReturnValue(null);
      mockDb.products.update.mockResolvedValue(updatedProduct);

      // Act - should succeed because validation is not implemented
      const result = await productManager.updateProduct('test-product-1', updateData);

      // Assert
      expect(result).toBeDefined();
      expect(result.trackerType).toBe('invalid-type');
    });
  });

  describe('deleteProduct', () => {
    it('should delete product successfully', async () => {
      // Arrange
      mockDb.deleteProduct.mockResolvedValue(undefined);

      // Act
      const result = await productManager.deleteProduct('test-product-1');

      // Assert - deleteProduct doesn't return a value
      expect(result).toBeUndefined();
      expect(mockDb.deleteProduct).toHaveBeenCalledWith('test-product-1');
    });

    it('should propagate delete errors (no error handling implemented)', async () => {
      // Arrange
      mockDb.deleteProduct.mockRejectedValue(new Error('Record not found'));

      // Act & Assert - should throw because error handling is not implemented
      await expect(productManager.deleteProduct('non-existent'))
        .rejects
        .toThrow(/record not found/i);
    });
  });

  describe('addSourceToProduct', () => {
    it('should add source to existing product', async () => {
      // Arrange
      const sourceData = {
        url: 'https://example.com/new-source',
        selector: '.price',
        storeName: 'New Store'
      };

      const newSource = new SourceTestDataBuilder()
        .withUrl('https://example.com/new-source')
        .withStoreName('New Store')
        .build();

      mockDb.sources.create.mockResolvedValue(newSource);

      // Act
      const result = await productManager.addSourceToProduct('test-product-1', sourceData);

      // Assert - test the outcome
      expect(result).toBeDefined();
      expect(result.url).toBe('https://example.com/new-source');
      expect(result.storeName).toBe('New Store');
      expect(mockDb.sources.create).toHaveBeenCalledWith(
        expect.objectContaining({
          productId: 'test-product-1',
          url: 'https://example.com/new-source',
          storeName: 'New Store',
          errorCount: 0,
          isActive: true
        })
      );
    });

    it('should handle duplicate URL for same product', async () => {
      // Arrange
      const sourceData = {
        url: 'https://example.com/existing-source',
        selector: '.price',
        storeName: 'Existing Store'
      };

      // Mock FileStore unique constraint error
      const fileStoreError = new Error('Unique constraint failed');
      mockDb.sources.create.mockRejectedValue(fileStoreError);

      // Act & Assert
      await expect(
        productManager.addSourceToProduct('test-product-1', sourceData)
      ).rejects.toThrow(/unique constraint|already exists/i);
    });
  });

  describe('updateBestDeal', () => {
    it('should update best deal for product with multiple sources', async () => {
      // Arrange
      const productWithSources = new ProductTestDataBuilder()
        .withSource('https://store1.com/product')
        .withSource('https://store2.com/product')
        .build();

      // Mock sources with different prices - currentValue is now direct objects
      productWithSources.sources[0].currentValue = { amount: 99.99, currency: 'USD' };
      productWithSources.sources[1].currentValue = { amount: 89.99, currency: 'USD' };

      mockDb.getProductWithSources.mockResolvedValue(productWithSources);
      mockDb.products.update.mockResolvedValue({
        ...productWithSources,
        bestSourceId: 'source-2',
        bestValue: { amount: 89.99, currency: 'USD' }
      });
      
      // Mock comparisons create
      mockDb.comparisons.create.mockResolvedValue({});

      // Act
      await productManager.updateBestDeal('test-product-1');

      // Assert - test that best deal logic works
      expect(mockDb.products.update).toHaveBeenCalledWith(
        'test-product-1',
        expect.objectContaining({
          bestSourceId: expect.any(String),
          bestValue: expect.any(Object)
        })
      );
    });

    it('should handle product with no sources (no update performed)', async () => {
      // Arrange
      const productWithoutSources = new ProductTestDataBuilder()
        .withSources([])
        .build();

      mockDb.getProductWithSources.mockResolvedValue(productWithoutSources);

      // Act
      await productManager.updateBestDeal('test-product-1');

      // Assert - should not update product when no sources (implementation doesn't handle this case)
      expect(mockDb.products.update).not.toHaveBeenCalled();
    });

    it('should handle non-existent product', async () => {
      // Arrange
      mockDb.getProductWithSources.mockResolvedValue(null);

      // Act & Assert
      await expect(productManager.updateBestDeal('non-existent'))
        .rejects
        .toThrow(/product not found/i);
    });
  });
});