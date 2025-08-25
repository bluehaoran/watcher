import express from 'express';
import { ProductManager } from '../../core/productManager';
import { PluginManager } from '../../plugins/PluginManager';
import { logger } from '../../utils/logger';

const router = express.Router();

// This would be injected in a real app
let productManager: ProductManager;
let pluginManager: PluginManager;

export function initializeProductRoutes(pm: ProductManager, plm: PluginManager) {
  productManager = pm;
  pluginManager = plm;
}

// GET /products - List all products
router.get('/', async (req, res) => {
  try {
    const { active, type, limit, offset } = req.query;
    
    const products = await productManager.getProducts({
      isActive: active === 'true' ? true : active === 'false' ? false : undefined,
      trackerType: type as string,
      limit: limit ? parseInt(limit as string) : undefined,
      offset: offset ? parseInt(offset as string) : undefined,
    });

    if (req.headers.accept?.includes('application/json')) {
      res.json(products);
    } else {
      // Return HTML dashboard
      res.send(generateProductsHTML(products));
    }
  } catch (error) {
    logger.error('Failed to get products:', error);
    res.status(500).json({ error: 'Failed to fetch products' });
  }
});

// GET /products/:id - Get single product
router.get('/:id', async (req, res) => {
  try {
    const product = await productManager.getProduct(req.params.id);
    
    if (!product) {
      return res.status(404).json({ error: 'Product not found' });
    }

    if (req.headers.accept?.includes('application/json')) {
      return res.json(product);
    } else {
      return res.send(generateProductDetailHTML(product));
    }
  } catch (error) {
    logger.error('Failed to get product:', error);
    return res.status(500).json({ error: 'Failed to fetch product' });
  }
});

// POST /products - Create new product
router.post('/', async (req, res) => {
  try {
    const productData = req.body;
    
    // Validate required fields
    if (!productData.name || !productData.trackerType || !productData.sources?.length) {
      return res.status(400).json({ 
        error: 'Missing required fields: name, trackerType, sources' 
      });
    }

    // Validate tracker type exists
    const tracker = pluginManager.getTracker(productData.trackerType);
    if (!tracker) {
      return res.status(400).json({ 
        error: `Invalid tracker type: ${productData.trackerType}` 
      });
    }

    const product = await productManager.createProduct(productData);
    return res.status(201).json(product);
    
  } catch (error) {
    logger.error('Failed to create product:', error);
    return res.status(500).json({ error: 'Failed to create product' });
  }
});

// PUT /products/:id - Update product
router.put('/:id', async (req, res) => {
  try {
    const product = await productManager.updateProduct(req.params.id, req.body);
    return res.json(product);
  } catch (error) {
    logger.error('Failed to update product:', error);
    return res.status(500).json({ error: 'Failed to update product' });
  }
});

// DELETE /products/:id - Delete product
router.delete('/:id', async (req, res) => {
  try {
    await productManager.deleteProduct(req.params.id);
    res.status(204).send();
  } catch (error) {
    logger.error('Failed to delete product:', error);
    res.status(500).json({ error: 'Failed to delete product' });
  }
});

// GET /products/:id/history - Get price history
router.get('/:id/history', async (req, res) => {
  try {
    // This would query price history from the database
    res.json({ message: 'History endpoint not implemented yet' });
  } catch (error) {
    logger.error('Failed to get product history:', error);
    res.status(500).json({ error: 'Failed to fetch history' });
  }
});

function generateProductsHTML(products: any[]): string {
  return `
    <!DOCTYPE html>
    <html lang="en">
    <head>
      <meta charset="UTF-8">
      <meta name="viewport" content="width=device-width, initial-scale=1.0">
      <title>Price Tracker Dashboard</title>
      <style>
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; margin: 0; padding: 20px; background: #f5f5f5; }
        .container { max-width: 1200px; margin: 0 auto; }
        .header { display: flex; justify-content: between; align-items: center; margin-bottom: 30px; }
        .btn { padding: 8px 16px; background: #007bff; color: white; text-decoration: none; border-radius: 4px; border: none; cursor: pointer; }
        .btn:hover { background: #0056b3; }
        .btn-secondary { background: #6c757d; }
        .btn-danger { background: #dc3545; }
        .card { background: white; border-radius: 8px; padding: 20px; margin-bottom: 20px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }
        .product-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(400px, 1fr)); gap: 20px; }
        .product-title { font-size: 1.2em; font-weight: bold; margin-bottom: 10px; }
        .product-meta { color: #666; font-size: 0.9em; margin-bottom: 15px; }
        .sources { margin: 15px 0; }
        .source { background: #f8f9fa; border-left: 4px solid #007bff; padding: 10px; margin: 5px 0; border-radius: 0 4px 4px 0; }
        .source.inactive { border-color: #dc3545; }
        .status { padding: 2px 8px; border-radius: 12px; font-size: 0.8em; }
        .status.active { background: #d4edda; color: #155724; }
        .status.inactive { background: #f8d7da; color: #721c24; }
        .price { font-size: 1.1em; font-weight: bold; color: #28a745; }
        .actions { margin-top: 15px; }
        .actions > * { margin-right: 10px; }
      </style>
    </head>
    <body>
      <div class="container">
        <div class="header">
          <h1>🔍 Price Tracker Dashboard</h1>
          <div>
            <a href="/products/new" class="btn">➕ Add Product</a>
            <a href="/scanner" class="btn btn-secondary">🔍 Element Scanner</a>
          </div>
        </div>
        
        <div class="product-grid">
          ${products.map(product => `
            <div class="card">
              <div class="product-title">${product.name}</div>
              <div class="product-meta">
                <span class="status ${product.isActive ? 'active' : 'inactive'}">
                  ${product.isActive ? '✓ Active' : '✗ Inactive'}
                </span>
                Type: ${product.trackerType} | 
                Sources: ${product.sources?.length || 0} |
                Last checked: ${product.lastChecked ? new Date(product.lastChecked).toLocaleString() : 'Never'}
              </div>
              
              ${product.bestValue ? `
                <div class="price">
                  Current best: ${JSON.parse(product.bestValue).amount ? '$' + JSON.parse(product.bestValue).amount.toFixed(2) : 'N/A'}
                </div>
              ` : ''}
              
              <div class="sources">
                ${product.sources?.slice(0, 3).map((source: any) => `
                  <div class="source ${source.isActive ? '' : 'inactive'}">
                    <strong>${source.storeName}</strong>
                    ${source.currentValue ? '- ' + (JSON.parse(source.currentValue).amount ? '$' + JSON.parse(source.currentValue).amount.toFixed(2) : 'N/A') : ''}
                    ${source.errorCount > 0 ? `<span style="color: #dc3545;">⚠ ${source.errorCount} errors</span>` : ''}
                  </div>
                `).join('') || '<div class="source">No sources configured</div>'}
                ${product.sources?.length > 3 ? `<div class="source">... and ${product.sources.length - 3} more</div>` : ''}
              </div>
              
              <div class="actions">
                <a href="/products/${product.id}" class="btn btn-secondary">View</a>
                <a href="/products/${product.id}/edit" class="btn btn-secondary">Edit</a>
                <button onclick="deleteProduct('${product.id}')" class="btn btn-danger">Delete</button>
              </div>
            </div>
          `).join('')}
        </div>
        
        ${products.length === 0 ? `
          <div class="card" style="text-align: center; padding: 40px;">
            <h2>No products yet</h2>
            <p>Get started by adding your first product to track</p>
            <a href="/products/new" class="btn">➕ Add Your First Product</a>
          </div>
        ` : ''}
      </div>
      
      <script>
        async function deleteProduct(id) {
          if (!confirm('Are you sure you want to delete this product?')) return;
          
          try {
            const response = await fetch(\`/products/\${id}\`, { method: 'DELETE' });
            if (response.ok) {
              location.reload();
            } else {
              alert('Failed to delete product');
            }
          } catch (error) {
            alert('Failed to delete product');
          }
        }
      </script>
    </body>
    </html>
  `;
}

function generateProductDetailHTML(product: any): string {
  return `
    <!DOCTYPE html>
    <html lang="en">
    <head>
      <meta charset="UTF-8">
      <meta name="viewport" content="width=device-width, initial-scale=1.0">
      <title>${product.name} - Price Tracker</title>
      <style>
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; margin: 0; padding: 20px; background: #f5f5f5; }
        .container { max-width: 1000px; margin: 0 auto; }
        .header { margin-bottom: 30px; }
        .btn { padding: 8px 16px; background: #007bff; color: white; text-decoration: none; border-radius: 4px; margin-right: 10px; }
        .card { background: white; border-radius: 8px; padding: 20px; margin-bottom: 20px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }
        .source { background: #f8f9fa; border: 1px solid #ddd; border-radius: 4px; padding: 15px; margin: 10px 0; }
        .status { padding: 2px 8px; border-radius: 12px; font-size: 0.8em; }
        .status.active { background: #d4edda; color: #155724; }
        .status.inactive { background: #f8d7da; color: #721c24; }
      </style>
    </head>
    <body>
      <div class="container">
        <div class="header">
          <h1>📊 ${product.name}</h1>
          <a href="/products" class="btn">← Back to Dashboard</a>
          <a href="/products/${product.id}/edit" class="btn">Edit Product</a>
        </div>
        
        <div class="card">
          <h3>Product Information</h3>
          <p><strong>Type:</strong> ${product.trackerType}</p>
          <p><strong>Status:</strong> 
            <span class="status ${product.isActive ? 'active' : 'inactive'}">
              ${product.isActive ? 'Active' : 'Inactive'}
            </span>
          </p>
          <p><strong>Check Interval:</strong> ${product.checkInterval}</p>
          <p><strong>Last Checked:</strong> ${product.lastChecked ? new Date(product.lastChecked).toLocaleString() : 'Never'}</p>
          ${product.description ? `<p><strong>Description:</strong> ${product.description}</p>` : ''}
        </div>
        
        <div class="card">
          <h3>Sources (${product.sources?.length || 0})</h3>
          ${product.sources?.map((source: any) => `
            <div class="source">
              <div style="display: flex; justify-content: space-between; align-items: start;">
                <div>
                  <h4>${source.storeName} 
                    <span class="status ${source.isActive ? 'active' : 'inactive'}">
                      ${source.isActive ? 'Active' : 'Inactive'}
                    </span>
                  </h4>
                  <p><strong>URL:</strong> <a href="${source.url}" target="_blank">${source.url}</a></p>
                  <p><strong>Selector:</strong> <code>${source.selector}</code></p>
                  ${source.currentValue ? `
                    <p><strong>Current Value:</strong> ${JSON.stringify(JSON.parse(source.currentValue), null, 2)}</p>
                  ` : ''}
                  ${source.lastChecked ? `
                    <p><strong>Last Checked:</strong> ${new Date(source.lastChecked).toLocaleString()}</p>
                  ` : ''}
                  ${source.errorCount > 0 ? `
                    <p style="color: #dc3545;"><strong>Errors:</strong> ${source.errorCount}</p>
                  ` : ''}
                </div>
              </div>
            </div>
          `).join('') || '<p>No sources configured</p>'}
        </div>
        
        <div class="card">
          <h3>Notifications (${product.notifications?.length || 0})</h3>
          ${product.notifications?.map((notif: any) => `
            <div style="border: 1px solid #ddd; border-radius: 4px; padding: 10px; margin: 5px 0;">
              <strong>${notif.notifierType}</strong> - 
              <span class="status ${notif.isEnabled ? 'active' : 'inactive'}">
                ${notif.isEnabled ? 'Enabled' : 'Disabled'}
              </span>
            </div>
          `).join('') || '<p>No notifications configured</p>'}
        </div>
      </div>
    </body>
    </html>
  `;
}

export { router as productRoutes };