import { ElementMatch } from '../plugins/base/TrackerPlugin';
import { WebScraper, ElementInfo } from './scraper';
import { logger } from '../utils/logger';

export interface MatchRankingOptions {
  exactMatchBonus: number;
  proximityWeight: number;
  contextWeight: number;
  positionWeight: number;
}

export class ElementFinder {
  private scraper: WebScraper;
  private defaultOptions: MatchRankingOptions = {
    exactMatchBonus: 50,
    proximityWeight: 0.3,
    contextWeight: 0.2,
    positionWeight: 0.1,
  };

  constructor(scraper: WebScraper) {
    this.scraper = scraper;
  }

  async findAndRankMatches(
    url: string, 
    searchText: string,
    options: Partial<MatchRankingOptions> = {}
  ): Promise<ElementMatch[]> {
    const opts = { ...this.defaultOptions, ...options };

    try {
      // Find all elements containing the search text
      const elements = await this.scraper.findElements(url, searchText);
      
      // Convert to ElementMatch format and calculate confidence scores
      const matches: ElementMatch[] = elements.map((element, index) => ({
        element: element.selector,
        text: element.text.trim(),
        html: element.html,
        context: element.context,
        confidence: this.calculateConfidence(element, searchText, index, opts)
      }));

      // Sort by confidence (highest first)
      matches.sort((a, b) => b.confidence - a.confidence);

      logger.info(`Found ${matches.length} matches for "${searchText}" on ${url}`);
      
      return matches;

    } catch (error) {
      logger.error(`Failed to find matches for "${searchText}" on ${url}:`, error);
      return [];
    }
  }

  private calculateConfidence(
    element: ElementInfo, 
    searchText: string, 
    position: number,
    options: MatchRankingOptions
  ): number {
    let confidence = 0;

    const text = element.text.toLowerCase();
    const search = searchText.toLowerCase();

    // Exact match bonus
    if (text === search) {
      confidence += options.exactMatchBonus;
    }

    // Text similarity (using simple string matching)
    const similarity = this.calculateStringSimilarity(text, search);
    confidence += similarity * 30;

    // Proximity - how close the text matches
    const proximity = this.calculateProximity(text, search);
    confidence += proximity * options.proximityWeight * 20;

    // Context quality - prefer elements with cleaner context
    const contextQuality = this.evaluateContextQuality(element.context);
    confidence += contextQuality * options.contextWeight * 10;

    // Position penalty (later elements get lower scores)
    const positionPenalty = position * options.positionWeight * 5;
    confidence -= positionPenalty;

    // HTML structure bonus (prefer certain tags)
    const structureBonus = this.evaluateHtmlStructure(element.html);
    confidence += structureBonus;

    return Math.max(0, Math.min(100, confidence));
  }

  private calculateStringSimilarity(text1: string, text2: string): number {
    const longer = text1.length > text2.length ? text1 : text2;
    const shorter = text1.length > text2.length ? text2 : text1;
    
    if (longer.length === 0) return 1.0;
    
    const editDistance = this.levenshteinDistance(longer, shorter);
    return (longer.length - editDistance) / longer.length;
  }

  private levenshteinDistance(str1: string, str2: string): number {
    const matrix = Array(str2.length + 1).fill(null).map(() => 
      Array(str1.length + 1).fill(null)
    );

    for (let i = 0; i <= str1.length; i++) {
      matrix[0][i] = i;
    }

    for (let j = 0; j <= str2.length; j++) {
      matrix[j][0] = j;
    }

    for (let j = 1; j <= str2.length; j++) {
      for (let i = 1; i <= str1.length; i++) {
        const substitutionCost = str1[i - 1] === str2[j - 1] ? 0 : 1;
        matrix[j][i] = Math.min(
          matrix[j][i - 1] + 1, // insertion
          matrix[j - 1][i] + 1, // deletion
          matrix[j - 1][i - 1] + substitutionCost // substitution
        );
      }
    }

    return matrix[str2.length][str1.length];
  }

  private calculateProximity(text: string, searchText: string): number {
    const index = text.indexOf(searchText);
    if (index === -1) return 0;
    
    // Prefer matches at the beginning or as whole words
    if (index === 0) return 1.0;
    
    // Check if it's a word boundary
    const beforeChar = text[index - 1];
    const afterChar = text[index + searchText.length];
    
    if (/\s/.test(beforeChar) && (!afterChar || /\s/.test(afterChar))) {
      return 0.8;
    }
    
    return 0.5;
  }

  private evaluateContextQuality(context: string): number {
    let quality = 0.5; // Base quality
    
    // Prefer cleaner HTML (less nested)
    const tagCount = (context.match(/<[^>]+>/g) || []).length;
    quality += Math.max(0, 1 - tagCount / 20);
    
    // Penalize very long contexts (might be noise)
    if (context.length > 1000) {
      quality -= 0.3;
    }
    
    return Math.max(0, Math.min(1, quality));
  }

  private evaluateHtmlStructure(html: string): number {
    let bonus = 0;
    
    // Prefer certain semantic tags
    const semanticTags = ['span', 'div', 'p', 'strong', 'em'];
    const priceTags = ['price', 'cost', 'amount'];
    
    for (const tag of semanticTags) {
      if (html.includes(`<${tag}`)) {
        bonus += 2;
        break;
      }
    }
    
    for (const priceClass of priceTags) {
      if (html.includes(priceClass)) {
        bonus += 5;
        break;
      }
    }
    
    // Bonus for data attributes that might indicate price/value
    if (html.includes('data-price') || html.includes('data-value')) {
      bonus += 10;
    }
    
    return bonus;
  }

  generateSearchVariations(input: string): string[] {
    const variations = new Set<string>();
    
    // Original input
    variations.add(input);
    
    // Trimmed version
    variations.add(input.trim());
    
    // Without currency symbols
    const withoutCurrency = input.replace(/[\$€£¥₹]/g, '').trim();
    if (withoutCurrency !== input) {
      variations.add(withoutCurrency);
    }
    
    // With common currency symbols
    const currencies = ['$', '€', '£', '¥', '₹'];
    currencies.forEach(symbol => {
      variations.add(`${symbol}${withoutCurrency}`);
      variations.add(`${withoutCurrency}${symbol}`);
    });
    
    // Numbers only (for price matching)
    const numbersOnly = input.replace(/[^\d.,]/g, '');
    if (numbersOnly && numbersOnly !== input) {
      variations.add(numbersOnly);
    }
    
    // With/without commas
    variations.add(input.replace(/,/g, ''));
    
    return Array.from(variations).filter(v => v.length > 0);
  }
}