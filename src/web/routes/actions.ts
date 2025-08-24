import express from 'express';
import { prisma } from '../../core/database.js';
import { logger } from '../../utils/logger.js';

const router = express.Router();

// GET /actions/dismiss/:productId - Dismiss notification
router.get('/dismiss/:productId', async (req, res) => {
  try {
    const { productId } = req.params;
    
    // Log the dismissal action
    await prisma.notificationLog.create({
      data: {
        productId,
        type: 'action',
        status: 'actioned',
        action: 'dismissed',
      }
    });

    logger.info(`Notification dismissed for product ${productId}`);
    
    res.send(generateActionResponseHTML('dismissed', 'Notification dismissed successfully'));
  } catch (error) {
    logger.error('Failed to dismiss notification:', error);
    res.status(500).send(generateActionResponseHTML('error', 'Failed to dismiss notification'));
  }
});

// GET /actions/false-positive/:productId - Mark as false positive
router.get('/false-positive/:productId', async (req, res) => {
  try {
    const { productId } = req.params;
    
    // Log the false positive action
    await prisma.notificationLog.create({
      data: {
        productId,
        type: 'action',
        status: 'actioned',
        action: 'false_positive',
      }
    });

    // You might want to pause the product or adjust confidence scoring
    logger.info(`False positive reported for product ${productId}`);
    
    res.send(generateActionResponseHTML('false-positive', 'Thank you for the feedback! We\'ll improve our detection.'));
  } catch (error) {
    logger.error('Failed to mark as false positive:', error);
    res.status(500).send(generateActionResponseHTML('error', 'Failed to process feedback'));
  }
});

// GET /actions/purchased/:productId - Mark as purchased
router.get('/purchased/:productId', async (req, res) => {
  try {
    const { productId } = req.params;
    
    // Log the purchase action
    await prisma.notificationLog.create({
      data: {
        productId,
        type: 'action',
        status: 'actioned',
        action: 'purchased',
      }
    });

    // Optionally pause or deactivate the product
    await prisma.product.update({
      where: { id: productId },
      data: { isPaused: true }
    });

    logger.info(`Product marked as purchased: ${productId}`);
    
    res.send(generateActionResponseHTML('purchased', 'Great! Product has been paused. You can reactivate it anytime from the dashboard.'));
  } catch (error) {
    logger.error('Failed to mark as purchased:', error);
    res.status(500).send(generateActionResponseHTML('error', 'Failed to mark as purchased'));
  }
});

// POST /actions/feedback - Submit detailed feedback
router.post('/feedback', async (req, res) => {
  try {
    const { productId, action, notes, sourceId } = req.body;
    
    if (action === 'false_positive' && sourceId) {
      // Create a false positive record with details
      await prisma.falsePositive.create({
        data: {
          sourceId,
          detectedText: req.body.detectedText || '',
          detectedValue: req.body.detectedValue || {},
          actualText: req.body.actualText,
          htmlContext: req.body.htmlContext || '',
          notes: notes || '',
        }
      });
    }

    // Log the feedback
    await prisma.notificationLog.create({
      data: {
        productId,
        type: 'feedback',
        status: 'actioned',
        action,
      }
    });

    logger.info(`Feedback received for product ${productId}: ${action}`);
    
    res.json({ success: true, message: 'Feedback submitted successfully' });
  } catch (error) {
    logger.error('Failed to submit feedback:', error);
    res.status(500).json({ success: false, error: 'Failed to submit feedback' });
  }
});

function generateActionResponseHTML(action: string, message: string): string {
  const icons: { [key: string]: string } = {
    'dismissed': '✅',
    'false-positive': '🔧',
    'purchased': '🛒',
    'error': '❌'
  };

  const colors: { [key: string]: string } = {
    'dismissed': '#28a745',
    'false-positive': '#ffc107',
    'purchased': '#007bff',
    'error': '#dc3545'
  };

  return `
    <!DOCTYPE html>
    <html lang="en">
    <head>
      <meta charset="UTF-8">
      <meta name="viewport" content="width=device-width, initial-scale=1.0">
      <title>Action Complete - Price Tracker</title>
      <style>
        body { 
          font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; 
          margin: 0; padding: 20px; background: #f5f5f5; 
          display: flex; justify-content: center; align-items: center;
          min-height: 100vh;
        }
        .container { 
          background: white; border-radius: 8px; padding: 40px; 
          text-align: center; max-width: 500px;
          box-shadow: 0 4px 6px rgba(0,0,0,0.1);
        }
        .icon { 
          font-size: 4em; margin-bottom: 20px; 
          color: ${colors[action] || '#6c757d'};
        }
        .message { 
          font-size: 1.2em; margin-bottom: 30px; 
          color: #333; line-height: 1.6;
        }
        .btn { 
          padding: 12px 24px; background: #007bff; color: white; 
          text-decoration: none; border-radius: 4px; margin: 10px;
          display: inline-block;
        }
        .btn:hover { background: #0056b3; }
        .btn-secondary { background: #6c757d; }
      </style>
    </head>
    <body>
      <div class="container">
        <div class="icon">${icons[action] || '✓'}</div>
        <div class="message">${message}</div>
        <div>
          <a href="/products" class="btn">📊 Back to Dashboard</a>
          <a href="javascript:window.close()" class="btn btn-secondary">Close</a>
        </div>
        <p style="margin-top: 30px; font-size: 0.9em; color: #666;">
          This action has been recorded and will help improve our tracking accuracy.
        </p>
      </div>
    </body>
    </html>
  `;
}

export { router as actionRoutes };