import { describe, it, expect, beforeEach, vi } from 'vitest';
import { ProductManager } from '@/core/productManager';
import { PluginManager } from '@/plugins/PluginManager';
import { mockProduct, mockSource, resetAllMocks } from '@tests/mocks';

// Mock the database module  
const mockPrisma = {
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

vi.mock('@/core/database', () => ({
  prisma: mockPrisma,
}));

describe('ProductManager', () => {
  let productManager: ProductManager;
  let mockPluginManager: PluginManager;

  beforeEach(() => {
    resetAllMocks();
    
    mockPluginManager = {
      getTracker: vi.fn().mockReturnValue({
        name: 'Price Tracker',
        type: 'price',
        parse: vi.fn(),
        format: vi.fn(),
        compare: vi.fn(),
      }),
    } as any;
    
    productManager = new ProductManager(mockPluginManager);
  });

  describe('createProduct', () => {
    it('should create a product with valid data', async () => {
      const createData = {
        name: 'Test Product',
        description: 'Test Description',
        trackerType: 'price',
        notifyOn: 'any_change',
        checkInterval: '0 0 * * *',
        sources: [
          {
            url: 'https://example.com/product',
            storeName: 'Test Store',
            selector: '.price',
            selectorType: 'css',
          },
        ],
      };

      mockPrisma.$transaction.mockImplementation(async (callback) => {
        const tx = {
          product: {
            create: vi.fn().mockResolvedValue(mockProduct),
          },
          source: {
            create: vi.fn().mockResolvedValue(mockSource),
          },
        };
        return await callback(tx);
      });

      const result = await productManager.createProduct(createData);

      expect(result).toEqual(mockProduct);
      expect(mockPrisma.$transaction).toHaveBeenCalled();
    });

    it('should validate tracker type exists', async () => {
      const createData = {
        name: 'Test Product',
        trackerType: 'invalid-type',
        notifyOn: 'any_change',
        checkInterval: '0 0 * * *',
        sources: [],
      };

      mockPluginManager.getTracker = vi.fn().mockReturnValue(null);

      await expect(productManager.createProduct(createData))
        .rejects
        .toThrow('Invalid tracker type: invalid-type');
    });

    it('should validate required sources', async () => {
      const createData = {
        name: 'Test Product',
        trackerType: 'price',
        notifyOn: 'any_change',
        checkInterval: '0 0 * * *',
        sources: [],
      };

      await expect(productManager.createProduct(createData))
        .rejects
        .toThrow('At least one source is required');
    });

    it('should handle duplicate source URLs within the same product', async () => {
      const createData = {
        name: 'Test Product',
        trackerType: 'price',
        notifyOn: 'any_change',
        checkInterval: '0 0 * * *',
        sources: [
          { url: 'https://example.com/product', selector: '.price' },
          { url: 'https://example.com/product', selector: '.price2' },
        ],
      };

      await expect(productManager.createProduct(createData))
        .rejects
        .toThrow('Duplicate source URL in product: https://example.com/product');
    });
  });

  describe('getProduct', () => {
    it('should return product with sources', async () => {
      const productWithSources = {
        ...mockProduct,
        sources: [mockSource],
      };

      mockPrisma.product.findUnique.mockResolvedValue(productWithSources);

      const result = await productManager.getProduct('test-product-1');

      expect(result).toEqual(productWithSources);
      expect(mockPrisma.product.findUnique).toHaveBeenCalledWith({
        where: { id: 'test-product-1' },
        include: {
          sources: true,
          notifications: true,
        },
      });
    });

    it('should return null for non-existent product', async () => {
      mockPrisma.product.findUnique.mockResolvedValue(null);

      const result = await productManager.getProduct('non-existent');

      expect(result).toBeNull();
    });
  });

  describe('getProducts', () => {
    it('should return all products when no filters provided', async () => {
      const products = [mockProduct];
      mockPrisma.product.findMany.mockResolvedValue(products);

      const result = await productManager.getProducts();

      expect(result).toEqual(products);
      expect(mockPrisma.product.findMany).toHaveBeenCalledWith({
        where: {},
        include: {
          sources: true,
          notifications: true,
        },
      });
    });

    it('should filter by isActive when provided', async () => {
      const products = [mockProduct];
      mockPrisma.product.findMany.mockResolvedValue(products);

      const result = await productManager.getProducts({ isActive: true });

      expect(result).toEqual(products);
      expect(mockPrisma.product.findMany).toHaveBeenCalledWith({
        where: { isActive: true },
        include: {
          sources: true,
          notifications: true,
        },
      });
    });

    it('should filter by trackerType when provided', async () => {
      const products = [mockProduct];
      mockPrisma.product.findMany.mockResolvedValue(products);

      const result = await productManager.getProducts({ trackerType: 'price' });

      expect(result).toEqual(products);
      expect(mockPrisma.product.findMany).toHaveBeenCalledWith({
        where: { trackerType: 'price' },
        include: {
          sources: true,
          notifications: true,
        },
      });
    });
  });

  describe('updateProduct', () => {
    it('should update product successfully', async () => {
      const updateData = { name: 'Updated Product Name' };
      const updatedProduct = { ...mockProduct, ...updateData };

      mockPrisma.product.update.mockResolvedValue(updatedProduct);

      const result = await productManager.updateProduct('test-product-1', updateData);

      expect(result).toEqual(updatedProduct);
      expect(mockPrisma.product.update).toHaveBeenCalledWith({
        where: { id: 'test-product-1' },
        data: updateData,
        include: {
          sources: true,
          notifications: true,
        },
      });
    });

    it('should validate tracker type when updating', async () => {
      const updateData = { trackerType: 'invalid-type' };
      mockPluginManager.getTracker = vi.fn().mockReturnValue(null);

      await expect(productManager.updateProduct('test-product-1', updateData))
        .rejects
        .toThrow('Invalid tracker type: invalid-type');
    });
  });

  describe('deleteProduct', () => {
    it('should delete product successfully', async () => {
      mockPrisma.product.delete.mockResolvedValue(mockProduct);

      await productManager.deleteProduct('test-product-1');

      expect(mockPrisma.product.delete).toHaveBeenCalledWith({
        where: { id: 'test-product-1' },
      });
    });

    it('should handle non-existent product deletion', async () => {
      mockPrisma.product.delete.mockRejectedValue(new Error('Product not found'));

      await expect(productManager.deleteProduct('non-existent'))
        .rejects
        .toThrow('Product not found');
    });
  });

  describe('addSourceToProduct', () => {
    it('should add source to existing product', async () => {
      const sourceData = {
        url: 'https://example.com/new-source',
        storeName: 'New Store',
        selector: '.price',
        selectorType: 'css',
      };

      mockPrisma.source.create.mockResolvedValue({
        ...mockSource,
        ...sourceData,
      });

      const result = await productManager.addSourceToProduct('test-product-1', sourceData);

      expect(result).toEqual({
        ...mockSource,
        ...sourceData,
      });

      expect(mockPrisma.source.create).toHaveBeenCalledWith({
        data: {
          productId: 'test-product-1',
          ...sourceData,
        },
      });
    });

    it('should handle duplicate URL for same product', async () => {
      mockPrisma.source.create.mockRejectedValue(
        new Error('Unique constraint failed on the fields: (`productId`,`url`)')
      );

      const sourceData = {
        url: 'https://example.com/duplicate',
        selector: '.price',
      };

      await expect(productManager.addSourceToProduct('test-product-1', sourceData))
        .rejects
        .toThrow('Source URL already exists for this product');
    });
  });

  describe('updateBestDeal', () => {
    it('should update best deal for product with multiple sources', async () => {
      const sources = [
        { ...mockSource, id: 'source-1', currentValue: JSON.stringify({ amount: 99.99 }) },
        { ...mockSource, id: 'source-2', currentValue: JSON.stringify({ amount: 89.99 }) },
      ];

      mockPrisma.source.findMany.mockResolvedValue(sources);

      const mockTracker = {
        compare: vi.fn().mockReturnValue({ changeType: 'increased' }),
      };
      mockPluginManager.getTracker = vi.fn().mockReturnValue(mockTracker);

      mockPrisma.product.update.mockResolvedValue(mockProduct);

      await productManager.updateBestDeal('test-product-1');

      expect(mockPrisma.product.update).toHaveBeenCalledWith({
        where: { id: 'test-product-1' },
        data: {
          bestSourceId: 'source-2',
          bestValue: JSON.stringify({ amount: 89.99 }),
        },
      });
    });

    it('should handle product with no sources', async () => {
      mockPrisma.source.findMany.mockResolvedValue([]);
      mockPrisma.product.update.mockResolvedValue(mockProduct);

      await productManager.updateBestDeal('test-product-1');

      expect(mockPrisma.product.update).toHaveBeenCalledWith({
        where: { id: 'test-product-1' },
        data: {
          bestSourceId: null,
          bestValue: null,
        },
      });
    });

    it('should handle sources with no current values', async () => {
      const sources = [
        { ...mockSource, id: 'source-1', currentValue: null },
        { ...mockSource, id: 'source-2', currentValue: null },
      ];

      mockPrisma.source.findMany.mockResolvedValue(sources);
      mockPrisma.product.update.mockResolvedValue(mockProduct);

      await productManager.updateBestDeal('test-product-1');

      expect(mockPrisma.product.update).toHaveBeenCalledWith({
        where: { id: 'test-product-1' },
        data: {
          bestSourceId: null,
          bestValue: null,
        },
      });
    });
  });
});