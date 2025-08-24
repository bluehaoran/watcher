import { ConfigSchema } from './TrackerPlugin.js';

export interface NotificationEvent {
  product: {
    id: string;
    name: string;
  };
  source?: {  // Specific source that triggered the notification
    id: string;
    url: string;
    storeName: string;
  };
  comparison?: {  // Comparison data when multiple sources exist
    best: {
      sourceId: string;
      storeName: string;
      value: any;
      formattedValue: string;
      url: string;
    };
    allSources: Array<{
      sourceId: string;
      storeName: string;
      value: any;
      formattedValue: string;
      url: string;
      changed: boolean;
    }>;
    savings?: {
      amount: number;
      percentage: number;
    };
  };
  changeType: 'increased' | 'decreased' | 'changed';
  oldValue: any;
  newValue: any;
  formattedOld: string;
  formattedNew: string;
  difference: string;
  threshold?: {
    type: 'absolute' | 'relative';
    value: number;
  };
  actionUrls: {
    dismiss: string;
    falsePositive: string;
    purchased: string;
    viewProduct: string;
  };
  screenshot?: string;  // Base64 or URL
}

export interface NotificationResult {
  success: boolean;
  messageId?: string;
  error?: string;
}

export abstract class NotifierPlugin {
  abstract name: string;
  abstract type: string;
  abstract description: string;
  
  abstract initialize(config: any): Promise<void>;
  abstract notify(event: NotificationEvent): Promise<NotificationResult>;
  abstract test(config: any): Promise<boolean>;
  abstract getConfigSchema(): ConfigSchema;
  abstract validateConfig(config: any): boolean;
}