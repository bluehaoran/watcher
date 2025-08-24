export interface ParseResult {
  success: boolean;
  value: any;
  normalized: string;
  confidence: number;
  metadata?: Record<string, any>;
}

export interface ComparisonResult {
  changed: boolean;
  changeType: 'increased' | 'decreased' | 'unchanged';
  difference: any;
  percentChange?: number;
}

export interface ElementMatch {
  element: string;  // CSS selector
  text: string;
  html: string;
  context: string;  // Surrounding HTML
  confidence: number;
}

export interface ConfigSchema {
  fields: Array<{
    name: string;
    type: 'text' | 'number' | 'select' | 'checkbox';
    label: string;
    required: boolean;
    default?: any;
    options?: Array<{value: string; label: string}>;
  }>;
}

export abstract class TrackerPlugin {
  abstract name: string;
  abstract type: string;
  abstract description: string;
  
  abstract parse(text: string): ParseResult;
  abstract format(value: any): string;
  abstract compare(oldValue: any, newValue: any): ComparisonResult;
  abstract getSearchVariations(input: string): string[];
  abstract rankMatches(input: string, matches: ElementMatch[]): ElementMatch[];
  abstract getConfigSchema(): ConfigSchema;
  abstract validateConfig(config: any): boolean;
}