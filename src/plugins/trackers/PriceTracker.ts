import { 
  TrackerPlugin, 
  ParseResult, 
  ComparisonResult, 
  ElementMatch, 
  ConfigSchema 
} from '../base/TrackerPlugin.js';

export class PriceTracker extends TrackerPlugin {
  name = 'Price Tracker';
  type = 'price';
  description = 'Track prices with multi-currency support';

  private currencySymbols = ['$', '€', '£', '¥', '₹', '₽', 'kr', 'R', 'C$', 'A$'];
  private currencyNames = ['USD', 'EUR', 'GBP', 'JPY', 'INR', 'RUB', 'SEK', 'ZAR', 'CAD', 'AUD'];

  parse(text: string): ParseResult {
    const cleaned = text.trim().toLowerCase();
    
    // Try to extract price with various patterns
    const patterns = [
      // Standard currency symbols
      /[\$€£¥₹₽]\s*([0-9]+(?:[,\s][0-9]{3})*(?:[.,][0-9]{1,2})?)/,
      // Price with currency after
      /([0-9]+(?:[,\s][0-9]{3})*(?:[.,][0-9]{1,2})?)\s*[\$€£¥₹₽]/,
      // Just numbers with decimal
      /([0-9]+(?:[,\s][0-9]{3})*[.,][0-9]{1,2})/,
      // Large numbers with commas
      /([0-9]+(?:[,][0-9]{3})+)/,
      // Simple numbers
      /([0-9]+(?:[.][0-9]+)?)/
    ];

    for (const pattern of patterns) {
      const match = cleaned.match(pattern);
      if (match) {
        const numericStr = match[1] || match[0];
        const value = this.parseNumericValue(numericStr);
        
        if (value !== null && value > 0) {
          // Detect currency
          const currency = this.detectCurrency(text);
          
          return {
            success: true,
            value: {
              amount: value,
              currency: currency || 'USD',
              originalText: text.trim()
            },
            normalized: `${currency || '$'}${value.toFixed(2)}`,
            confidence: this.calculateConfidence(text, value),
            metadata: {
              originalFormat: numericStr,
              detectedCurrency: currency
            }
          };
        }
      }
    }

    return {
      success: false,
      value: null,
      normalized: text,
      confidence: 0
    };
  }

  format(value: any): string {
    if (!value || typeof value !== 'object') {
      return String(value || '');
    }

    const { amount, currency } = value;
    const symbol = this.getCurrencySymbol(currency);
    
    return `${symbol}${amount.toFixed(2)}`;
  }

  compare(oldValue: any, newValue: any): ComparisonResult {
    if (!oldValue || !newValue || !oldValue.amount || !newValue.amount) {
      return {
        changed: false,
        changeType: 'unchanged',
        difference: 0
      };
    }

    const oldAmount = parseFloat(oldValue.amount);
    const newAmount = parseFloat(newValue.amount);
    
    if (oldAmount === newAmount) {
      return {
        changed: false,
        changeType: 'unchanged',
        difference: 0,
        percentChange: 0
      };
    }

    const difference = newAmount - oldAmount;
    const percentChange = ((newAmount - oldAmount) / oldAmount) * 100;

    return {
      changed: true,
      changeType: difference > 0 ? 'increased' : 'decreased',
      difference: Math.abs(difference),
      percentChange: Math.abs(percentChange)
    };
  }

  getSearchVariations(input: string): string[] {
    const variations = new Set<string>();
    
    // Original
    variations.add(input);
    
    // Parse the input to get numeric value
    const parsed = this.parse(input);
    if (parsed.success && parsed.value?.amount) {
      const amount = parsed.value.amount;
      
      // Different currency formats
      this.currencySymbols.forEach(symbol => {
        variations.add(`${symbol}${amount}`);
        variations.add(`${symbol}${amount.toFixed(2)}`);
        variations.add(`${amount}${symbol}`);
      });
      
      // With/without decimals
      variations.add(amount.toString());
      variations.add(amount.toFixed(2));
      
      // With thousand separators
      variations.add(amount.toLocaleString());
      variations.add(amount.toLocaleString('en-US'));
    }
    
    return Array.from(variations);
  }

  rankMatches(input: string, matches: ElementMatch[]): ElementMatch[] {
    const parsedInput = this.parse(input);
    
    return matches.map(match => {
      let bonus = 0;
      
      // Bonus for price-related classes/attributes
      if (match.html.match(/class.*price|data-price|price.*class/i)) {
        bonus += 20;
      }
      
      // Bonus for currency symbols
      if (this.currencySymbols.some(symbol => match.text.includes(symbol))) {
        bonus += 15;
      }
      
      // Bonus if parsed values match closely
      const matchParsed = this.parse(match.text);
      if (parsedInput.success && matchParsed.success) {
        const inputAmount = parsedInput.value?.amount || 0;
        const matchAmount = matchParsed.value?.amount || 0;
        
        if (Math.abs(inputAmount - matchAmount) < 0.01) {
          bonus += 30;
        }
      }
      
      return {
        ...match,
        confidence: Math.min(100, match.confidence + bonus)
      };
    }).sort((a, b) => b.confidence - a.confidence);
  }

  getConfigSchema(): ConfigSchema {
    return {
      fields: [
        {
          name: 'defaultCurrency',
          type: 'select',
          label: 'Default Currency',
          required: false,
          default: 'USD',
          options: [
            { value: 'USD', label: 'US Dollar ($)' },
            { value: 'EUR', label: 'Euro (€)' },
            { value: 'GBP', label: 'British Pound (£)' },
            { value: 'JPY', label: 'Japanese Yen (¥)' },
            { value: 'CAD', label: 'Canadian Dollar (C$)' },
            { value: 'AUD', label: 'Australian Dollar (A$)' }
          ]
        },
        {
          name: 'precision',
          type: 'number',
          label: 'Decimal Precision',
          required: false,
          default: 2
        },
        {
          name: 'ignoreSmallChanges',
          type: 'checkbox',
          label: 'Ignore changes less than 1%',
          required: false,
          default: false
        }
      ]
    };
  }

  validateConfig(config: any): boolean {
    if (!config) return true;
    
    if (config.precision && (config.precision < 0 || config.precision > 4)) {
      return false;
    }
    
    if (config.defaultCurrency && !this.currencyNames.includes(config.defaultCurrency)) {
      return false;
    }
    
    return true;
  }

  private parseNumericValue(str: string): number | null {
    // Remove spaces and normalize decimal separators
    const cleaned = str.replace(/\s/g, '').replace(/,/g, '');
    const parsed = parseFloat(cleaned);
    
    return isNaN(parsed) ? null : parsed;
  }

  private detectCurrency(text: string): string | null {
    // Check for currency symbols
    for (let i = 0; i < this.currencySymbols.length; i++) {
      if (text.includes(this.currencySymbols[i])) {
        return this.currencyNames[i] || 'USD';
      }
    }
    
    // Check for currency names/codes
    const upperText = text.toUpperCase();
    for (const currency of this.currencyNames) {
      if (upperText.includes(currency)) {
        return currency;
      }
    }
    
    return null;
  }

  private getCurrencySymbol(currency: string): string {
    const index = this.currencyNames.indexOf(currency);
    return index >= 0 ? this.currencySymbols[index] : '$';
  }

  private calculateConfidence(text: string, value: number): number {
    let confidence = 50; // Base confidence
    
    // Bonus for currency symbols
    if (this.currencySymbols.some(symbol => text.includes(symbol))) {
      confidence += 20;
    }
    
    // Bonus for reasonable price values
    if (value > 0 && value < 100000) {
      confidence += 15;
    }
    
    // Bonus for decimal precision
    if (text.match(/\.[0-9]{2}/)) {
      confidence += 10;
    }
    
    // Penalty for very large numbers (might not be prices)
    if (value > 1000000) {
      confidence -= 20;
    }
    
    return Math.max(0, Math.min(100, confidence));
  }
}