import { vi } from 'vitest';

// Mock environment variables for testing
process.env.NODE_ENV = 'test';
process.env.DATABASE_URL = 'file:./test.db';
process.env.SECRET_KEY = 'test-secret-key';
process.env.BASE_URL = 'http://localhost:3000';
process.env.LOG_LEVEL = 'error'; // Suppress logs during tests

// Mock external dependencies
vi.mock('playwright', () => ({
  chromium: {
    launch: vi.fn().mockResolvedValue({
      newContext: vi.fn().mockResolvedValue({
        newPage: vi.fn().mockResolvedValue({
          goto: vi.fn().mockResolvedValue(undefined),
          content: vi.fn().mockResolvedValue('<html>Mock content</html>'),
          screenshot: vi.fn().mockResolvedValue(Buffer.from('mock-screenshot')),
          close: vi.fn().mockResolvedValue(undefined),
          evaluate: vi.fn().mockResolvedValue(undefined),
          title: vi.fn().mockResolvedValue('Mock Title'),
        }),
        close: vi.fn().mockResolvedValue(undefined),
      }),
      close: vi.fn().mockResolvedValue(undefined),
    }),
  },
}));

vi.mock('nodemailer', () => ({
  default: {
    createTransport: vi.fn().mockReturnValue({
      verify: vi.fn().mockResolvedValue(true),
      sendMail: vi.fn().mockResolvedValue({
        messageId: 'mock-message-id',
      }),
    }),
  },
}));