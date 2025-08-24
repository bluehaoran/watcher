import { 
  TrackerPlugin, 
  ParseResult, 
  ComparisonResult, 
  ElementMatch, 
  ConfigSchema 
} from '../base/TrackerPlugin.js';

export class NumberTracker extends TrackerPlugin {
  name = 'Number Tracker';
  type = 'number';
  description = 'Track generic numbers (stock levels, scores, counts)';

  parse(text: string): ParseResult {
    const cleaned = text.trim();
    
    // Number patterns (in order of preference)
    const patterns = [
      // Number with units (100 items, 50%, 3.5GB)
      /([0-9]+(?:[.,][0-9]+)?)\s*([a-zA-Z%]+)/,
      // Decimal numbers
      /([0-9]+[.,][0-9]+)/,
      // Large numbers with separators
      /([0-9]+(?:[,\s][0-9]{3})+)/,
      // Simple integers
      /([0-9]+)/
    ];

    for (const pattern of patterns) {
      const match = cleaned.match(pattern);
      if (match) {
        const numericStr = match[1];
        const unit = match[2] || null;
        const value = this.parseNumericValue(numericStr);
        
        if (value !== null && !isNaN(value)) {
          const numberValue = {
            value,
            unit,
            originalText: text.trim(),
            formatted: this.formatNumber(value, unit)
          };

          return {
            success: true,
            value: numberValue,
            normalized: numberValue.formatted,
            confidence: this.calculateConfidence(text, value, unit),
            metadata: {
              hasUnit: !!unit,
              isInteger: Number.isInteger(value),
              originalFormat: match[0]
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

    return value.formatted || this.formatNumber(value.value, value.unit);
  }

  compare(oldValue: any, newValue: any): ComparisonResult {
    if (!oldValue || !newValue || oldValue.value === undefined || newValue.value === undefined) {
      return {
        changed: false,
        changeType: 'unchanged',
        difference: 0
      };
    }

    const oldNum = parseFloat(oldValue.value);
    const newNum = parseFloat(newValue.value);
    
    if (oldNum === newNum) {
      return {
        changed: false,
        changeType: 'unchanged',
        difference: 0,
        percentChange: 0
      };
    }

    const difference = newNum - oldNum;
    const percentChange = oldNum !== 0 ? Math.abs((difference / oldNum) * 100) : 100;

    return {
      changed: true,
      changeType: difference > 0 ? 'increased' : 'decreased',
      difference: Math.abs(difference),
      percentChange
    };
  }

  getSearchVariations(input: string): string[] {
    const variations = new Set<string>();
    const parsed = this.parse(input);
    
    if (parsed.success && parsed.value) {
      const { value, unit } = parsed.value;
      
      // Original
      variations.add(input);
      
      // Just the number
      variations.add(value.toString());
      variations.add(Number.isInteger(value) ? value.toString() : value.toFixed(2));
      
      // With different formatting
      if (value >= 1000) {
        variations.add(value.toLocaleString());
        variations.add(value.toLocaleString('en-US'));
      }
      
      // With/without units
      if (unit) {
        variations.add(`${value} ${unit}`);
        variations.add(`${value}${unit}`);
        variations.add(`${value.toFixed(0)} ${unit}`);
      }
      
      // Common number formats
      if (Number.isInteger(value)) {
        variations.add(value.toFixed(0));
      } else {
        variations.add(value.toFixed(1));
        variations.add(value.toFixed(2));
      }
      
    } else {
      variations.add(input);
    }
    
    return Array.from(variations);
  }

  rankMatches(input: string, matches: ElementMatch[]): ElementMatch[] {
    const parsedInput = this.parse(input);
    
    return matches.map(match => {
      let bonus = 0;
      
      // Bonus for numeric context
      if (match.html.match(/count|total|amount|quantity|level|score|rating/i)) {
        bonus += 15;
      }
      
      // Bonus for data attributes
      if (match.html.match(/data-count|data-value|data-number|data-quantity/i)) {
        bonus += 20;
      }
      
      // Bonus for exact number matches
      const matchParsed = this.parse(match.text);
      if (parsedInput.success && matchParsed.success) {
        const inputValue = parsedInput.value?.value || 0;
        const matchValue = matchParsed.value?.value || 0;
        
        if (Math.abs(inputValue - matchValue) < 0.01) {
          bonus += 25;
        }
      }
      
      // Bonus for units matching
      if (parsedInput.success && matchParsed.success) {
        const inputUnit = parsedInput.value?.unit;
        const matchUnit = matchParsed.value?.unit;
        
        if (inputUnit && matchUnit && inputUnit.toLowerCase() === matchUnit.toLowerCase()) {
          bonus += 15;
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
          name: 'numberType',
          type: 'select',
          label: 'Number Type',
          required: false,
          default: 'any',
          options: [
            { value: 'any', label: 'Any Number' },
            { value: 'integer', label: 'Integers Only' },
            { value: 'decimal', label: 'Decimals Only' },
            { value: 'percentage', label: 'Percentages' }
          ]
        },
        {
          name: 'expectedUnit',
          type: 'text',
          label: 'Expected Unit (optional)',
          required: false,
          default: ''
        },
        {
          name: 'decimalPrecision',
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
    
    if (config.numberType && !['any', 'integer', 'decimal', 'percentage'].includes(config.numberType)) {
      return false;
    }
    
    if (config.decimalPrecision && (config.decimalPrecision < 0 || config.decimalPrecision > 10)) {
      return false;
    }
    
    return true;
  }

  private parseNumericValue(str: string): number | null {
    // Handle different decimal separators and thousand separators
    let cleaned = str.replace(/\s/g, '');
    
    // Handle European format (1.234,56)
    if (cleaned.match(/^\d+\.\d{3},\d+$/)) {
      cleaned = cleaned.replace(/\./g, '').replace(/,/, '.');
    }
    // Handle US format with commas (1,234.56)
    else if (cleaned.match(/^\d+,\d{3}/)) {
      cleaned = cleaned.replace(/,/g, '');
    }
    // Handle comma as decimal separator (1234,56)
    else if (cleaned.match(/^\d+,\d+$/) && !cleaned.includes('.')) {
      cleaned = cleaned.replace(/,/, '.');
    }
    
    const parsed = parseFloat(cleaned);
    return isNaN(parsed) ? null : parsed;
  }

  private formatNumber(value: number, unit: string | null): string {
    let formatted: string;
    
    // Format based on the number size
    if (Number.isInteger(value) && value >= -999999 && value <= 999999) {
      formatted = value.toString();
    } else if (value >= 1000000) {
      formatted = (value / 1000000).toFixed(1) + 'M';
    } else if (value >= 1000) {
      formatted = (value / 1000).toFixed(1) + 'K';
    } else {
      formatted = value.toFixed(2).replace(/\.00$/, '');
    }
    
    return unit ? `${formatted} ${unit}` : formatted;
  }

  private calculateConfidence(text: string, value: number, unit: string | null): number {
    let confidence = 40; // Base confidence
    
    // Bonus for having units
    if (unit) {
      confidence += 20;
    }
    
    // Bonus for reasonable number ranges
    if (value >= 0 && value <= 1000000) {
      confidence += 15;
    }
    
    // Bonus for integer values (often more reliable)
    if (Number.isInteger(value)) {
      confidence += 10;
    }
    
    // Bonus for context clues
    if (text.match(/\b(stock|inventory|count|total|quantity|level|score|rating|available|remaining)\b/i)) {
      confidence += 15;
    }
    
    // Penalty for very large numbers (might be IDs or timestamps)
    if (value > 1000000000) {
      confidence -= 25;
    }
    
    // Penalty for very small decimals (might be rates or ratios)
    if (value < 1 && value > 0) {
      confidence -= 5;
    }
    
    return Math.max(0, Math.min(100, confidence));
  }
}