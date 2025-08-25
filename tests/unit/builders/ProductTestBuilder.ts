export class ProductTestDataBuilder {
  private data: any = {
    id: 'test-product-1',
    name: 'Test Product',
    description: 'Test Description',
    trackerType: 'price',
    notifyOn: 'any_change',
    checkInterval: '0 0 * * *',
    isActive: true,
    isPaused: false,
    lastChecked: null,
    nextCheck: null,
    bestSourceId: null,
    bestValue: null,
    threshold: null,
    createdAt: new Date('2025-01-01T00:00:00Z'),
    updatedAt: new Date('2025-01-01T00:00:00Z'),
    sources: [],
    notifications: [],
    comparisons: []
  };

  withId(id: string) {
    this.data.id = id;
    return this;
  }

  withName(name: string) {
    this.data.name = name;
    return this;
  }

  withDescription(description: string) {
    this.data.description = description;
    return this;
  }

  withTrackerType(type: string) {
    this.data.trackerType = type;
    return this;
  }

  withSources(sources: any[]) {
    this.data.sources = sources;
    return this;
  }

  withNotifications(notifications: any[]) {
    this.data.notifications = notifications;
    return this;
  }

  withSource(url: string, selector = '.price', storeName = 'Test Store') {
    this.data.sources.push({
      id: `source-${this.data.sources.length + 1}`,
      url,
      selector,
      selectorType: 'css',
      storeName,
      title: 'Test Product Page',
      productId: this.data.id,
      isActive: true,
      currentValue: null,
      currentText: null,
      errorCount: 0,
      createdAt: new Date(),
      updatedAt: new Date()
    });
    return this;
  }

  withActiveStatus(isActive: boolean) {
    this.data.isActive = isActive;
    return this;
  }

  build() {
    return { ...this.data };
  }
}

export class SourceTestDataBuilder {
  private data: any = {
    id: 'test-source-1',
    productId: 'test-product-1',
    url: 'https://example.com/product',
    selector: '.price',
    selectorType: 'css',
    storeName: 'Test Store',
    title: 'Test Product Page',
    isActive: true,
    currentValue: { amount: 99.99, currency: 'USD' },
    currentText: '$99.99',
    originalValue: { amount: 104.99, currency: 'USD' },
    originalText: '$104.99',
    errorCount: 0,
    lastError: null,
    lastChecked: new Date(),
    createdAt: new Date('2025-01-01T00:00:00Z'),
    updatedAt: new Date('2025-01-01T00:00:00Z')
  };

  withId(id: string) {
    this.data.id = id;
    return this;
  }

  withUrl(url: string) {
    this.data.url = url;
    return this;
  }

  withSelector(selector: string, type = 'css') {
    this.data.selector = selector;
    this.data.selectorType = type;
    return this;
  }

  withStoreName(storeName: string) {
    this.data.storeName = storeName;
    return this;
  }

  withCurrentValue(value: any, text: string) {
    this.data.currentValue = value;
    this.data.currentText = text;
    return this;
  }

  withError(errorCount: number, lastError?: string) {
    this.data.errorCount = errorCount;
    this.data.lastError = lastError;
    return this;
  }

  build() {
    return { ...this.data };
  }
}

export class CreateProductDataBuilder {
  private data: any = {
    name: 'Test Product',
    description: 'Test Description',
    trackerType: 'price',
    notifyOn: 'any_change',
    checkInterval: '0 0 * * *',
    sources: [],
    notifications: []
  };

  withName(name: string) {
    this.data.name = name;
    return this;
  }

  withTrackerType(type: string) {
    this.data.trackerType = type;
    return this;
  }

  withSource(url: string, selector = '.price', storeName = 'Test Store') {
    this.data.sources.push({
      url,
      selector,
      selectorType: 'css',
      storeName
    });
    return this;
  }

  withNotification(type: string, config: any) {
    this.data.notifications.push({ type, config });
    return this;
  }

  withEmptySources() {
    this.data.sources = [];
    return this;
  }

  withDuplicateSource(url: string) {
    this.data.sources.push(
      { url, selector: '.price1', storeName: 'Store 1' },
      { url, selector: '.price2', storeName: 'Store 2' }
    );
    return this;
  }

  build() {
    return { ...this.data };
  }
}