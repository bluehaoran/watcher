import express from 'express';
import { ProductManager } from '../../core/productManager.js';
import { logger } from '../../utils/logger.js';

const router = express.Router();

let productManager: ProductManager;

export function initializeSourceRoutes(pm: ProductManager) {
  productManager = pm;
}

// POST /sources - Add source to product
router.post('/', async (req, res) => {
  try {
    const { productId, url, storeName, selector, selectorType } = req.body;
    
    if (!productId || !url || !selector) {
      return res.status(400).json({ 
        error: 'Missing required fields: productId, url, selector' 
      });
    }

    const source = await productManager.addSourceToProduct(productId, {
      url,
      storeName,
      selector,
      selectorType: selectorType || 'css'
    });

    res.status(201).json(source);
  } catch (error) {
    logger.error('Failed to add source:', error);
    res.status(500).json({ error: 'Failed to add source' });
  }
});

// PUT /sources/:id - Update source
router.put('/:id', async (req, res) => {
  try {
    const source = await productManager.updateSource(req.params.id, req.body);
    res.json(source);
  } catch (error) {
    logger.error('Failed to update source:', error);
    res.status(500).json({ error: 'Failed to update source' });
  }
});

// DELETE /sources/:id - Delete source
router.delete('/:id', async (req, res) => {
  try {
    await productManager.removeSourceFromProduct(req.params.id);
    res.status(204).send();
  } catch (error) {
    logger.error('Failed to delete source:', error);
    res.status(500).json({ error: 'Failed to delete source' });
  }
});

export { router as sourceRoutes };