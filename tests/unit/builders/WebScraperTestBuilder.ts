export class ScrapingResultBuilder {
  private data = {
    success: true,
    content: '$99.99',
    title: 'Test Product Page',
    screenshot: Buffer.from('mock-screenshot'),
    metadata: {
      description: 'Test product description',
      image: 'https://example.com/image.jpg',
      price: '$99.99'
    },
    error: undefined as string | undefined
  };

  withSuccess(success: boolean) {
    this.data.success = success;
    return this;
  }

  withContent(content: string) {
    this.data.content = content;
    return this;
  }

  withTitle(title: string) {
    this.data.title = title;
    return this;
  }

  withScreenshot(screenshot?: Buffer) {
    this.data.screenshot = screenshot;
    return this;
  }

  withMetadata(metadata: any) {
    this.data.metadata = metadata;
    return this;
  }

  withError(error: string) {
    this.data.success = false;
    this.data.error = error;
    return this;
  }

  withEmptyContent() {
    this.data.content = '';
    return this;
  }

  build() {
    return { ...this.data };
  }
}

export class ElementMatchBuilder {
  private data = {
    element: '.price',
    text: '$99.99',
    html: '<span class="price">$99.99</span>',
    context: '<div class="product"><span class="price">$99.99</span></div>',
    confidence: 0
  };

  withElement(element: string) {
    this.data.element = element;
    return this;
  }

  withText(text: string) {
    this.data.text = text;
    return this;
  }

  withHtml(html: string) {
    this.data.html = html;
    return this;
  }

  withConfidence(confidence: number) {
    this.data.confidence = confidence;
    return this;
  }

  build() {
    return { ...this.data };
  }
}