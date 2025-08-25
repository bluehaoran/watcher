import express from 'express';
import { WebScraper } from '../../core/scraper';
import { ElementFinder } from '../../core/elementFinder';
import { PluginManager } from '../../plugins/PluginManager';
import { logger } from '../../utils/logger';

const router = express.Router();

let pluginManager: PluginManager;
let scraper: WebScraper;
let elementFinder: ElementFinder;

export function initializeScannerRoutes(pm: PluginManager) {
  pluginManager = pm;
  scraper = new WebScraper();
  elementFinder = new ElementFinder(scraper);
  
  // Initialize scraper
  scraper.initialize().catch(err => {
    logger.error('Failed to initialize scraper for scanner routes:', err);
  });
}

// GET /scanner - Element scanner interface
router.get('/', (req, res) => {
  return res.send(generateScannerHTML());
});

// POST /scanner/scan - Scan URL for elements
router.post('/scan', async (req, res) => {
  try {
    const { url, searchText, trackerType } = req.body;
    
    if (!url) {
      return res.status(400).json({ error: 'URL is required' });
    }

    logger.info(`Scanning ${url} for "${searchText}"`);

    let matches: any[] = [];
    
    if (searchText) {
      // Find elements containing the search text
      matches = await elementFinder.findAndRankMatches(url, searchText);
      
      // Use tracker plugin to rank matches if specified
      if (trackerType) {
        const tracker = pluginManager.getTracker(trackerType);
        if (tracker) {
          matches = tracker.rankMatches(searchText, matches);
        }
      }
    } else {
      // Just scrape the page for basic info
      const scrapeResult = await scraper.scrape(url);
      if (!scrapeResult.success) {
        throw new Error(scrapeResult.error || 'Failed to scrape page');
      }
    }

    return res.json({
      success: true,
      url,
      searchText,
      matches: matches.slice(0, 10), // Limit to top 10 matches
      totalMatches: matches.length
    });

  } catch (error) {
    logger.error('Scan failed:', error);
    return res.status(500).json({ 
      success: false,
      error: error instanceof Error ? error.message : String(error)
    });
  }
});

// POST /scanner/test-selector - Test a specific CSS selector
router.post('/test-selector', async (req, res) => {
  try {
    const { url, selector, trackerType } = req.body;
    
    if (!url || !selector) {
      return res.status(400).json({ error: 'URL and selector are required' });
    }

    const scrapeResult = await scraper.scrape(url, selector);
    
    if (!scrapeResult.success) {
      return res.json({
        success: false,
        error: scrapeResult.error || 'Failed to scrape'
      });
    }

    let parseResult = null;
    if (trackerType && scrapeResult.content) {
      const tracker = pluginManager.getTracker(trackerType);
      if (tracker) {
        parseResult = tracker.parse(scrapeResult.content);
      }
    }

    return res.json({
      success: true,
      content: scrapeResult.content,
      parseResult,
      screenshot: scrapeResult.screenshot?.substring(0, 100) + '...', // Truncate for response
    });

  } catch (error) {
    logger.error('Selector test failed:', error);
    return res.status(500).json({ 
      success: false,
      error: error instanceof Error ? error.message : String(error)
    });
  }
});

function generateScannerHTML(): string {
  return `
    <!DOCTYPE html>
    <html lang="en">
    <head>
      <meta charset="UTF-8">
      <meta name="viewport" content="width=device-width, initial-scale=1.0">
      <title>Element Scanner - Price Tracker</title>
      <style>
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; margin: 0; padding: 20px; background: #f5f5f5; }
        .container { max-width: 1200px; margin: 0 auto; }
        .card { background: white; border-radius: 8px; padding: 20px; margin-bottom: 20px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }
        .form-group { margin-bottom: 15px; }
        label { display: block; margin-bottom: 5px; font-weight: bold; }
        input, select, button { padding: 8px 12px; border: 1px solid #ddd; border-radius: 4px; font-size: 14px; }
        input[type="url"], input[type="text"] { width: 100%; max-width: 500px; }
        .btn { padding: 10px 20px; background: #007bff; color: white; border: none; border-radius: 4px; cursor: pointer; margin-right: 10px; }
        .btn:hover { background: #0056b3; }
        .btn:disabled { background: #6c757d; cursor: not-allowed; }
        .btn-secondary { background: #6c757d; }
        .results { margin-top: 20px; }
        .match { border: 1px solid #ddd; border-radius: 4px; padding: 15px; margin: 10px 0; background: #f8f9fa; }
        .confidence { font-weight: bold; padding: 2px 8px; border-radius: 12px; color: white; }
        .confidence.high { background: #28a745; }
        .confidence.medium { background: #ffc107; color: black; }
        .confidence.low { background: #dc3545; }
        .selector { font-family: monospace; background: #e9ecef; padding: 2px 4px; border-radius: 2px; }
        .loading { display: none; text-align: center; padding: 20px; }
        .error { color: #dc3545; background: #f8d7da; border: 1px solid #f5c6cb; padding: 10px; border-radius: 4px; margin: 10px 0; }
        .success { color: #155724; background: #d4edda; border: 1px solid #c3e6cb; padding: 10px; border-radius: 4px; margin: 10px 0; }
        .tabs { border-bottom: 1px solid #ddd; margin-bottom: 20px; }
        .tab { padding: 10px 20px; background: none; border: none; cursor: pointer; border-bottom: 2px solid transparent; }
        .tab.active { border-bottom-color: #007bff; color: #007bff; }
        .tab-content { display: none; }
        .tab-content.active { display: block; }
      </style>
    </head>
    <body>
      <div class="container">
        <div class="card">
          <h1>🔍 Element Scanner</h1>
          <p>Find and test CSS selectors for tracking values on web pages. This tool helps you identify the best elements to track for your products.</p>
        </div>

        <div class="card">
          <div class="tabs">
            <button class="tab active" onclick="showTab('search')">🔍 Search Elements</button>
            <button class="tab" onclick="showTab('test')">🧪 Test Selector</button>
          </div>

          <div id="search-tab" class="tab-content active">
            <h3>Search for Elements</h3>
            <p>Enter a URL and search text to find matching elements on the page.</p>
            
            <form id="searchForm">
              <div class="form-group">
                <label for="searchUrl">Website URL:</label>
                <input type="url" id="searchUrl" placeholder="https://example.com/product" required>
              </div>
              
              <div class="form-group">
                <label for="searchText">Search Text (price, version, number):</label>
                <input type="text" id="searchText" placeholder="$99.99" required>
              </div>
              
              <div class="form-group">
                <label for="trackerType">Tracker Type:</label>
                <select id="trackerType">
                  <option value="">Auto-detect</option>
                  <option value="price">Price</option>
                  <option value="version">Version</option>
                  <option value="number">Number</option>
                </select>
              </div>
              
              <button type="submit" class="btn">🔍 Search Elements</button>
            </form>
          </div>

          <div id="test-tab" class="tab-content">
            <h3>Test CSS Selector</h3>
            <p>Test a specific CSS selector to see what content it extracts.</p>
            
            <form id="testForm">
              <div class="form-group">
                <label for="testUrl">Website URL:</label>
                <input type="url" id="testUrl" placeholder="https://example.com/product" required>
              </div>
              
              <div class="form-group">
                <label for="testSelector">CSS Selector:</label>
                <input type="text" id="testSelector" placeholder=".price" required>
              </div>
              
              <div class="form-group">
                <label for="testTrackerType">Tracker Type:</label>
                <select id="testTrackerType">
                  <option value="">None</option>
                  <option value="price">Price</option>
                  <option value="version">Version</option>
                  <option value="number">Number</option>
                </select>
              </div>
              
              <button type="submit" class="btn">🧪 Test Selector</button>
            </form>
          </div>
        </div>

        <div class="loading" id="loading">
          <p>⏳ Scanning page... This may take a few seconds.</p>
        </div>

        <div id="results" class="results"></div>
      </div>

      <script>
        function showTab(tabName) {
          // Hide all tabs and content
          document.querySelectorAll('.tab').forEach(tab => tab.classList.remove('active'));
          document.querySelectorAll('.tab-content').forEach(content => content.classList.remove('active'));
          
          // Show selected tab and content
          event.target.classList.add('active');
          document.getElementById(tabName + '-tab').classList.add('active');
        }

        document.getElementById('searchForm').addEventListener('submit', async function(e) {
          e.preventDefault();
          
          const url = document.getElementById('searchUrl').value;
          const searchText = document.getElementById('searchText').value;
          const trackerType = document.getElementById('trackerType').value;
          
          showLoading(true);
          clearResults();
          
          try {
            const response = await fetch('/scanner/scan', {
              method: 'POST',
              headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify({ url, searchText, trackerType })
            });
            
            const result = await response.json();
            showResults(result);
          } catch (error) {
            showError('Failed to scan page: ' + error.message);
          } finally {
            showLoading(false);
          }
        });

        document.getElementById('testForm').addEventListener('submit', async function(e) {
          e.preventDefault();
          
          const url = document.getElementById('testUrl').value;
          const selector = document.getElementById('testSelector').value;
          const trackerType = document.getElementById('testTrackerType').value;
          
          showLoading(true);
          clearResults();
          
          try {
            const response = await fetch('/scanner/test-selector', {
              method: 'POST',
              headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify({ url, selector, trackerType })
            });
            
            const result = await response.json();
            showTestResults(result, selector);
          } catch (error) {
            showError('Failed to test selector: ' + error.message);
          } finally {
            showLoading(false);
          }
        });

        function showLoading(show) {
          document.getElementById('loading').style.display = show ? 'block' : 'none';
        }

        function clearResults() {
          document.getElementById('results').innerHTML = '';
        }

        function showError(message) {
          document.getElementById('results').innerHTML = 
            '<div class="error">❌ Error: ' + message + '</div>';
        }

        function showResults(result) {
          const resultsDiv = document.getElementById('results');
          
          if (!result.success) {
            showError(result.error);
            return;
          }

          let html = '<div class="card"><h3>🎯 Found ' + result.totalMatches + ' matches</h3>';
          
          if (result.matches && result.matches.length > 0) {
            html += result.matches.map(match => {
              const confidenceClass = match.confidence >= 70 ? 'high' : match.confidence >= 40 ? 'medium' : 'low';
              
              return \`
                <div class="match">
                  <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 10px;">
                    <span class="confidence \${confidenceClass}">\${match.confidence}% confidence</span>
                    <button class="btn btn-secondary" onclick="useSelector('\${match.element}')">Use This Selector</button>
                  </div>
                  <div><strong>Selector:</strong> <span class="selector">\${match.element}</span></div>
                  <div><strong>Text:</strong> \${match.text}</div>
                  <div><strong>HTML:</strong> <code>\${match.html.substring(0, 100)}...</code></div>
                </div>
              \`;
            }).join('');
          } else {
            html += '<p>No matches found. Try adjusting your search text.</p>';
          }
          
          html += '</div>';
          resultsDiv.innerHTML = html;
        }

        function showTestResults(result, selector) {
          const resultsDiv = document.getElementById('results');
          
          if (!result.success) {
            showError(result.error);
            return;
          }

          let html = '<div class="card">';
          html += '<div class="success">✅ Selector test successful!</div>';
          html += '<div><strong>Selector:</strong> <span class="selector">' + selector + '</span></div>';
          html += '<div><strong>Extracted Text:</strong> ' + (result.content || 'No content') + '</div>';
          
          if (result.parseResult) {
            html += '<div><strong>Parsed Value:</strong> ' + JSON.stringify(result.parseResult, null, 2) + '</div>';
          }
          
          html += '</div>';
          resultsDiv.innerHTML = html;
        }

        function useSelector(selector) {
          // Switch to test tab and populate the selector
          showTab('test');
          document.getElementById('testSelector').value = selector;
          
          // Copy URL if available
          const searchUrl = document.getElementById('searchUrl').value;
          if (searchUrl) {
            document.getElementById('testUrl').value = searchUrl;
          }
        }
      </script>
    </body>
    </html>
  `;
}

export { router as scannerRoutes };