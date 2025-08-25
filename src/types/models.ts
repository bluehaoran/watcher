import { BaseEntity } from '../core/filestore';

export interface Product extends BaseEntity {
  name: string;
  description?: string;
  trackerType: string;
  notifyOn: string;
  threshold?: {
    type: 'absolute' | 'relative';
    value: number;
  };
  checkInterval: string;
  lastChecked?: string;
  nextCheck?: string;
  isActive: boolean;
  isPaused: boolean;
  bestSourceId?: string;
  bestValue?: any;
}

export interface Source extends BaseEntity {
  productId: string;
  url: string;
  storeName?: string;
  title: string;
  selector: string;
  selectorType: string;
  originalValue?: any;
  currentValue?: any;
  originalText?: string;
  currentText?: string;
  isActive: boolean;
  lastChecked?: string;
  errorCount: number;
  lastError?: string;
}

export interface PriceComparison extends BaseEntity {
  productId: string;
  sources: Array<{
    sourceId: string;
    value: any;
    storeName: string;
  }>;
  bestSourceId: string;
  bestValue: any;
  worstValue: any;
  avgValue: any;
  timestamp: string;
}

export interface NotificationConfig extends BaseEntity {
  productId: string;
  notifierType: string;
  config: any;
  isEnabled: boolean;
}

export interface PriceHistory extends BaseEntity {
  sourceId: string;
  value: any;
  text: string;
  timestamp: string;
}

export interface FalsePositive extends BaseEntity {
  sourceId: string;
  detectedText: string;
  detectedValue: any;
  actualText?: string;
  htmlContext: string;
  screenshot?: string;
  notes?: string;
  timestamp: string;
}

export interface NotificationLog extends BaseEntity {
  productId: string;
  type: string;
  status: 'sent' | 'failed' | 'actioned';
  action?: 'dismissed' | 'false_positive' | 'purchased';
  error?: string;
  timestamp: string;
  actionedAt?: string;
}

export interface SystemSettings extends BaseEntity {
  key: string;
  value: any;
}

// Helper types for creating entities
export type CreateProduct = Omit<Product, 'id' | 'createdAt' | 'updatedAt' | 'isActive' | 'isPaused'> & {
  isActive?: boolean;
  isPaused?: boolean;
};

export type CreateSource = Omit<Source, 'id' | 'createdAt' | 'updatedAt' | 'isActive' | 'errorCount'> & {
  isActive?: boolean;
  errorCount?: number;
};

export type CreateNotificationConfig = Omit<NotificationConfig, 'id' | 'createdAt' | 'updatedAt' | 'isEnabled'> & {
  isEnabled?: boolean;
};

export type UpdateProduct = Partial<Omit<Product, 'id' | 'createdAt' | 'updatedAt'>>;
export type UpdateSource = Partial<Omit<Source, 'id' | 'createdAt' | 'updatedAt'>>;
export type UpdateNotificationConfig = Partial<Omit<NotificationConfig, 'id' | 'createdAt' | 'updatedAt'>>;

// Extended product with relations
export interface ProductWithSources extends Product {
  sources: Source[];
  notifications: NotificationConfig[];
}

// Query result types
export interface ProductSummary {
  id: string;
  name: string;
  trackerType: string;
  isActive: boolean;
  isPaused: boolean;
  lastChecked?: string;
  sourceCount: number;
  bestValue?: any;
  bestStoreName?: string;
}