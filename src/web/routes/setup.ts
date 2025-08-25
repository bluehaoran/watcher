import express from 'express';
import { PluginManager } from '../../plugins/PluginManager';
import { logger } from '../../utils/logger';

const router = express.Router();

let pluginManager: PluginManager;

export function initializeSetupRoutes(pm: PluginManager) {
  pluginManager = pm;
}

// GET /setup - Setup wizard home
router.get('/', (req, res) => {
  res.send(generateSetupHTML());
});

// GET /setup/trackers - Get available tracker types
router.get('/trackers', (req, res) => {
  try {
    const trackers = pluginManager.getAvailableTrackers().map(tracker => ({
      type: tracker.type,
      name: tracker.name,
      description: tracker.description,
      configSchema: tracker.getConfigSchema()
    }));
    
    res.json(trackers);
  } catch (error) {
    logger.error('Failed to get trackers:', error);
    res.status(500).json({ error: 'Failed to fetch trackers' });
  }
});

// GET /setup/notifiers - Get available notifier types
router.get('/notifiers', (req, res) => {
  try {
    const notifiers = pluginManager.getAvailableNotifiers().map(notifier => ({
      type: notifier.type,
      name: notifier.name,
      description: notifier.description,
      configSchema: notifier.getConfigSchema()
    }));
    
    res.json(notifiers);
  } catch (error) {
    logger.error('Failed to get notifiers:', error);
    res.status(500).json({ error: 'Failed to fetch notifiers' });
  }
});

// POST /setup/test-notifier - Test notifier configuration
router.post('/test-notifier', async (req, res) => {
  try {
    const { type, config } = req.body;
    
    const notifier = pluginManager.getNotifier(type);
    if (!notifier) {
      return res.status(400).json({ error: `Notifier type not found: ${type}` });
    }

    const isValid = await notifier.test(config);
    
    return res.json({ 
      success: isValid,
      message: isValid ? 'Configuration test successful' : 'Configuration test failed'
    });

  } catch (error) {
    logger.error('Failed to test notifier:', error);
    return res.status(500).json({ 
      success: false, 
      error: 'Test failed',
      message: error instanceof Error ? error.message : String(error)
    });
  }
});

function generateSetupHTML(): string {
  return `
    <!DOCTYPE html>
    <html lang="en">
    <head>
      <meta charset="UTF-8">
      <meta name="viewport" content="width=device-width, initial-scale=1.0">
      <title>Setup - Price Tracker</title>
      <style>
        body { 
          font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; 
          margin: 0; padding: 20px; background: #f5f5f5; 
        }
        .container { 
          max-width: 800px; margin: 0 auto; 
        }
        .card { 
          background: white; border-radius: 8px; padding: 30px; 
          margin-bottom: 20px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); 
        }
        .btn { 
          padding: 12px 24px; background: #007bff; color: white; 
          text-decoration: none; border-radius: 4px; border: none; 
          cursor: pointer; font-size: 16px; margin-right: 10px;
        }
        .btn:hover { background: #0056b3; }
        .btn-secondary { background: #6c757d; }
        .btn-success { background: #28a745; }
        .step { 
          border: 2px solid #007bff; border-radius: 8px; 
          padding: 20px; margin: 15px 0; 
        }
        .step h3 { margin-top: 0; color: #007bff; }
        .feature-grid { 
          display: grid; grid-template-columns: repeat(auto-fit, minmax(250px, 1fr)); 
          gap: 20px; margin: 20px 0; 
        }
        .feature { 
          background: #f8f9fa; border-radius: 6px; padding: 15px; 
          border-left: 4px solid #007bff; 
        }
        .hero { 
          text-align: center; padding: 40px 20px; 
          background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
          color: white; border-radius: 8px; margin-bottom: 30px;
        }
        .hero h1 { margin: 0 0 10px 0; font-size: 2.5em; }
      </style>
    </head>
    <body>
      <div class="container">
        <div class="hero">
          <h1>🔍 Price Tracker</h1>
          <p style="font-size: 1.2em; margin: 0;">Monitor prices and versions across multiple sources</p>
        </div>

        <div class="card">
          <h2>Welcome to Price Tracker!</h2>
          <p>This powerful tool helps you track prices, software versions, and other values across multiple websites. Get notified when prices drop, versions update, or any tracked value changes.</p>
          
          <div class="feature-grid">
            <div class="feature">
              <h4>💰 Price Tracking</h4>
              <p>Track product prices across multiple stores with multi-currency support</p>
            </div>
            <div class="feature">
              <h4>🔄 Version Monitoring</h4>
              <p>Monitor software versions and get notified of updates</p>
            </div>
            <div class="feature">
              <h4>📊 Number Tracking</h4>
              <p>Track any numeric value like stock levels, scores, or ratings</p>
            </div>
            <div class="feature">
              <h4>🔔 Smart Notifications</h4>
              <p>Email and Discord notifications with customizable triggers</p>
            </div>
            <div class="feature">
              <h4>🎯 Visual Element Picker</h4>
              <p>Easy-to-use element selector with live preview</p>
            </div>
            <div class="feature">
              <h4>🐳 Docker Ready</h4>
              <p>Containerized application with easy deployment</p>
            </div>
          </div>
        </div>

        <div class="card">
          <h2>Getting Started</h2>
          
          <div class="step">
            <h3>Step 1: Add Your First Product</h3>
            <p>Start by adding a product you want to track. You can track prices, versions, or any numeric value.</p>
            <a href="/products/new" class="btn">➕ Add Product</a>
          </div>

          <div class="step">
            <h3>Step 2: Configure Element Selection</h3>
            <p>Use our visual element picker to select exactly what you want to track on each webpage.</p>
            <a href="/scanner" class="btn btn-secondary">🔍 Element Scanner</a>
          </div>

          <div class="step">
            <h3>Step 3: Set Up Notifications</h3>
            <p>Configure email or Discord notifications to get alerts when your tracked values change.</p>
            <a href="#" onclick="showNotifierConfig()" class="btn btn-success">⚙️ Configure Notifications</a>
          </div>
        </div>

        <div class="card">
          <h2>Quick Actions</h2>
          <a href="/products" class="btn">📊 View Dashboard</a>
          <a href="/products/new" class="btn btn-secondary">➕ Add Product</a>
          <a href="/scanner" class="btn btn-secondary">🔍 Test Element Scanner</a>
        </div>
      </div>

      <div id="notifierModal" style="display: none; position: fixed; top: 0; left: 0; width: 100%; height: 100%; background: rgba(0,0,0,0.5); z-index: 1000;">
        <div style="position: absolute; top: 50%; left: 50%; transform: translate(-50%, -50%); background: white; padding: 30px; border-radius: 8px; max-width: 500px; width: 90%;">
          <h3>Configure Notifications</h3>
          <p>Set up your notification preferences to receive alerts when tracked values change.</p>
          
          <div style="margin: 20px 0;">
            <h4>Available Notifiers:</h4>
            <div id="notifierList">Loading...</div>
          </div>
          
          <div style="text-align: right; margin-top: 20px;">
            <button onclick="hideNotifierConfig()" class="btn btn-secondary">Close</button>
          </div>
        </div>
      </div>

      <script>
        async function showNotifierConfig() {
          document.getElementById('notifierModal').style.display = 'block';
          
          try {
            const response = await fetch('/setup/notifiers');
            const notifiers = await response.json();
            
            const listHtml = notifiers.map(notifier => \`
              <div style="border: 1px solid #ddd; border-radius: 4px; padding: 15px; margin: 10px 0;">
                <h5>\${notifier.name}</h5>
                <p>\${notifier.description}</p>
                <small>Type: \${notifier.type}</small>
              </div>
            \`).join('');
            
            document.getElementById('notifierList').innerHTML = listHtml;
          } catch (error) {
            document.getElementById('notifierList').innerHTML = '<p style="color: #dc3545;">Failed to load notifiers</p>';
          }
        }

        function hideNotifierConfig() {
          document.getElementById('notifierModal').style.display = 'none';
        }

        // Close modal when clicking outside
        document.getElementById('notifierModal').addEventListener('click', function(e) {
          if (e.target === this) {
            hideNotifierConfig();
          }
        });
      </script>
    </body>
    </html>
  `;
}

export { router as setupRoutes };