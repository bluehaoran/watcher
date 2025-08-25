import { describe, it, expect, beforeEach, vi, MockedFunction } from 'vitest';
import { EmailNotifier } from '@/plugins/notifiers/EmailNotifier';
import { mockNotificationEvent } from '../../mocks';

// Mock nodemailer with proper TypeScript types
vi.mock('nodemailer');

// Mock config with empty defaults to test validation
vi.mock('@/utils/config', () => ({
  config: {
    smtpHost: '',
    smtpPort: 587,
    smtpSecure: false,
    smtpUser: '',
    smtpPass: '',
    smtpFrom: 'Price Tracker <test@example.com>',
    baseUrl: 'http://localhost:3000'
  }
}));

// Mock logger
vi.mock('@/utils/logger', () => ({
  logger: {
    info: vi.fn(),
    error: vi.fn(),
    warn: vi.fn(),
    debug: vi.fn()
  }
}));

describe('EmailNotifier', () => {
  let emailNotifier: EmailNotifier;
  let mockTransporter: {
    verify: MockedFunction<any>;
    sendMail: MockedFunction<any>;
  };

  beforeEach(async () => {
    vi.clearAllMocks();

    // Create mock transporter
    mockTransporter = {
      verify: vi.fn(),
      sendMail: vi.fn()
    };

    // Mock nodemailer.createTransport to return our mock transporter
    const nodemailer = await import('nodemailer');
    vi.mocked(nodemailer.default.createTransport).mockReturnValue(mockTransporter as any);

    emailNotifier = new EmailNotifier();
  });

  describe('Plugin Interface', () => {
    it('should have correct plugin metadata', () => {
      expect(emailNotifier.name).toBe('Email Notifier');
      expect(emailNotifier.type).toBe('email');
      expect(emailNotifier.description).toBe('Send notifications via email using SMTP');
    });
  });

  describe('initialize', () => {
    it('should initialize with valid configuration', async () => {
      mockTransporter.verify.mockResolvedValue(true);

      const config = {
        host: 'smtp.gmail.com',
        port: 587,
        secure: false,
        user: 'test@gmail.com',
        pass: 'password123'
      };

      await expect(emailNotifier.initialize(config)).resolves.not.toThrow();
      expect(mockTransporter.verify).toHaveBeenCalled();
    });

    it('should throw error for missing SMTP host', async () => {
      const config = {
        user: 'test@gmail.com',
        pass: 'password123'
      };

      await expect(emailNotifier.initialize(config))
        .rejects
        .toThrow('Email configuration is incomplete');
    });

    it('should throw error for missing credentials', async () => {
      const config = {
        host: 'smtp.gmail.com'
      };

      await expect(emailNotifier.initialize(config))
        .rejects
        .toThrow('Email configuration is incomplete');
    });

    it('should handle SMTP verification failure', async () => {
      mockTransporter.verify.mockRejectedValue(new Error('SMTP connection failed'));

      const config = {
        host: 'smtp.gmail.com',
        user: 'test@gmail.com',
        pass: 'password123'
      };

      await expect(emailNotifier.initialize(config))
        .rejects
        .toThrow('SMTP connection failed');
    });
  });

  describe('notify', () => {
    beforeEach(async () => {
      mockTransporter.verify.mockResolvedValue(true);
      await emailNotifier.initialize({
        host: 'smtp.gmail.com',
        user: 'test@gmail.com',
        pass: 'password123'
      });
    });

    it('should send notification successfully', async () => {
      const mockResult = {
        messageId: '<test-message-id>',
        envelope: {},
        pending: [],
        response: 'OK'
      };
      mockTransporter.sendMail.mockResolvedValue(mockResult);

      const result = await emailNotifier.notify(mockNotificationEvent);

      expect(result.success).toBe(true);
      expect(result.messageId).toBe('<test-message-id>');
      expect(mockTransporter.sendMail).toHaveBeenCalledWith(
        expect.objectContaining({
          subject: expect.any(String),
          text: expect.any(String),
          html: expect.any(String),
          from: expect.any(String),
          to: expect.any(String)
        })
      );
    });

    it('should handle email sending failure', async () => {
      mockTransporter.sendMail.mockRejectedValue(new Error('Send failed'));

      const result = await emailNotifier.notify(mockNotificationEvent);

      expect(result.success).toBe(false);
      expect(result.error).toBe('Send failed');
    });

    it('should return error if not initialized', async () => {
      const uninitializedNotifier = new EmailNotifier();
      
      const result = await uninitializedNotifier.notify(mockNotificationEvent);

      expect(result.success).toBe(false);
      expect(result.error).toBe('Email notifier not initialized');
    });
  });

  describe('test', () => {
    it('should test configuration successfully', async () => {
      mockTransporter.verify.mockResolvedValue(true);
      mockTransporter.sendMail.mockResolvedValue({
        messageId: '<test-message-id>',
        envelope: {},
        pending: [],
        response: 'OK'
      });

      const config = {
        host: 'smtp.gmail.com',
        user: 'test@gmail.com',
        pass: 'password123'
      };

      const result = await emailNotifier.test(config);

      expect(result).toBe(true);
      expect(mockTransporter.verify).toHaveBeenCalled();
      expect(mockTransporter.sendMail).toHaveBeenCalled();
    });

    it('should return false for failed test', async () => {
      mockTransporter.verify.mockRejectedValue(new Error('Connection failed'));

      const config = {
        host: 'smtp.gmail.com',
        user: 'test@gmail.com',
        pass: 'password123'
      };

      const result = await emailNotifier.test(config);

      expect(result).toBe(false);
    });
  });

  describe('getConfigSchema', () => {
    it('should return valid configuration schema', () => {
      const schema = emailNotifier.getConfigSchema();

      expect(schema.fields).toBeDefined();
      expect(Array.isArray(schema.fields)).toBe(true);
      expect(schema.fields.length).toBeGreaterThan(0);

      // Check for essential fields
      const fieldNames = schema.fields.map(f => f.name);
      expect(fieldNames).toContain('host');
      expect(fieldNames).toContain('port');
      expect(fieldNames).toContain('user');
      expect(fieldNames).toContain('pass');
    });
  });

  describe('validateConfig', () => {
    it('should validate complete configuration', () => {
      const validConfig = {
        host: 'smtp.gmail.com',
        port: 587,
        secure: false,
        user: 'test@gmail.com',
        pass: 'password123',
        from: 'test@gmail.com',
        to: 'user@example.com'
      };

      const result = emailNotifier.validateConfig(validConfig);
      expect(result).toBe(true);
    });

    it('should reject invalid configuration', () => {
      const invalidConfig = {
        host: '',
        port: 'invalid',
        user: 'not-an-email'
      };

      const result = emailNotifier.validateConfig(invalidConfig);
      expect(result).toBe(false);
    });
  });
});