import { describe, it, expect, beforeEach, vi } from 'vitest';
import { EmailNotifier } from '@/plugins/notifiers/EmailNotifier';
import { mockNotificationEvent, resetAllMocks } from '@tests/mocks';

// Mock nodemailer - define at top level
const mockTransporter = {
  verify: vi.fn(),
  sendMail: vi.fn(),
};

const mockNodemailer = {
  createTransport: vi.fn(),
};

// Set up the mock return
mockNodemailer.createTransport.mockReturnValue(mockTransporter);

vi.mock('nodemailer', () => ({
  default: mockNodemailer,
}));

describe('EmailNotifier', () => {
  let emailNotifier: EmailNotifier;
  let validConfig: any;

  beforeEach(() => {
    resetAllMocks();
    emailNotifier = new EmailNotifier();
    
    validConfig = {
      host: 'smtp.gmail.com',
      port: 587,
      secure: false,
      auth: {
        user: 'test@gmail.com',
        pass: 'password123',
      },
      from: 'Price Tracker <test@gmail.com>',
      to: 'user@example.com',
    };
  });

  describe('initialize', () => {
    it('should initialize with valid SMTP configuration', async () => {
      mockTransporter.verify.mockResolvedValue(true);

      await expect(emailNotifier.initialize(validConfig)).resolves.not.toThrow();
      expect(mockTransporter.verify).toHaveBeenCalled();
    });

    it('should reject incomplete configuration', async () => {
      const incompleteConfig = {
        host: 'smtp.gmail.com',
        // Missing auth
      };

      await expect(emailNotifier.initialize(incompleteConfig))
        .rejects
        .toThrow('Email configuration is incomplete');
    });

    it('should reject configuration with missing host', async () => {
      const configWithoutHost = {
        ...validConfig,
        host: undefined,
      };

      await expect(emailNotifier.initialize(configWithoutHost))
        .rejects
        .toThrow('Email configuration is incomplete');
    });

    it('should reject configuration with missing auth', async () => {
      const configWithoutAuth = {
        ...validConfig,
        auth: {
          user: 'test@gmail.com',
          // Missing pass
        },
      };

      await expect(emailNotifier.initialize(configWithoutAuth))
        .rejects
        .toThrow('Email configuration is incomplete');
    });

    it('should handle SMTP connection failure', async () => {
      mockTransporter.verify.mockRejectedValue(new Error('Connection failed'));

      await expect(emailNotifier.initialize(validConfig))
        .rejects
        .toThrow('Connection failed');
    });

    it('should handle invalid SMTP credentials', async () => {
      mockTransporter.verify.mockRejectedValue(new Error('Authentication failed'));

      await expect(emailNotifier.initialize(validConfig))
        .rejects
        .toThrow('Authentication failed');
    });
  });

  describe('notify', () => {
    beforeEach(async () => {
      mockTransporter.verify.mockResolvedValue(true);
      await emailNotifier.initialize(validConfig);
    });

    it('should send email notification successfully', async () => {
      mockTransporter.sendMail.mockResolvedValue({
        messageId: '<test-message-id>',
        accepted: ['user@example.com'],
        rejected: [],
      });

      const result = await emailNotifier.notify(mockNotificationEvent);

      expect(result.success).toBe(true);
      expect(result.messageId).toBe('<test-message-id>');
      expect(mockTransporter.sendMail).toHaveBeenCalledWith({
        from: validConfig.from,
        to: validConfig.to,
        subject: expect.stringContaining('Test Product'),
        html: expect.stringContaining('$99.99'),
        text: expect.stringContaining('Test Product'),
      });
    });

    it('should handle single source notification', async () => {
      mockTransporter.sendMail.mockResolvedValue({
        messageId: '<test-message-id>',
        accepted: ['user@example.com'],
        rejected: [],
      });

      const result = await emailNotifier.notify(mockNotificationEvent);

      expect(result.success).toBe(true);
      
      const emailCall = mockTransporter.sendMail.mock.calls[0][0];
      expect(emailCall.subject).toContain('Price Alert');
      expect(emailCall.html).toContain('Test Store');
      expect(emailCall.html).toContain('$99.99');
      expect(emailCall.html).toContain('decreased');
    });

    it('should handle multi-source comparison notification', async () => {
      const multiSourceEvent = {
        ...mockNotificationEvent,
        comparison: {
          best: {
            sourceId: 'source-1',
            storeName: 'Best Store',
            value: { amount: 89.99, currency: 'USD' },
            formattedValue: '$89.99',
            url: 'https://beststore.com/product',
          },
          allSources: [
            {
              sourceId: 'source-1',
              storeName: 'Best Store',
              value: { amount: 89.99, currency: 'USD' },
              formattedValue: '$89.99',
              url: 'https://beststore.com/product',
              changed: false,
            },
            {
              sourceId: 'source-2',
              storeName: 'Other Store',
              value: { amount: 99.99, currency: 'USD' },
              formattedValue: '$99.99',
              url: 'https://otherstore.com/product',
              changed: true,
            },
          ],
          savings: {
            amount: 10.00,
            percentage: 10.0,
          },
        },
      };

      mockTransporter.sendMail.mockResolvedValue({
        messageId: '<test-message-id>',
        accepted: ['user@example.com'],
        rejected: [],
      });

      const result = await emailNotifier.notify(multiSourceEvent);

      expect(result.success).toBe(true);
      
      const emailCall = mockTransporter.sendMail.mock.calls[0][0];
      expect(emailCall.html).toContain('Best Price');
      expect(emailCall.html).toContain('$89.99');
      expect(emailCall.html).toContain('Best Store');
      expect(emailCall.html).toContain('Save $10.00');
      expect(emailCall.html).toContain('Other Store');
    });

    it('should include action buttons in email', async () => {
      mockTransporter.sendMail.mockResolvedValue({
        messageId: '<test-message-id>',
        accepted: ['user@example.com'],
        rejected: [],
      });

      const result = await emailNotifier.notify(mockNotificationEvent);

      expect(result.success).toBe(true);
      
      const emailCall = mockTransporter.sendMail.mock.calls[0][0];
      expect(emailCall.html).toContain(mockNotificationEvent.actionUrls.viewProduct);
      expect(emailCall.html).toContain(mockNotificationEvent.actionUrls.dismiss);
      expect(emailCall.html).toContain(mockNotificationEvent.actionUrls.falsePositive);
      expect(emailCall.html).toContain('View Product');
      expect(emailCall.html).toContain('Dismiss');
      expect(emailCall.html).toContain('Mark as False Positive');
    });

    it('should handle email sending failure', async () => {
      mockTransporter.sendMail.mockRejectedValue(new Error('Send failed'));

      const result = await emailNotifier.notify(mockNotificationEvent);

      expect(result.success).toBe(false);
      expect(result.error).toContain('Send failed');
    });

    it('should handle SMTP authentication error during send', async () => {
      mockTransporter.sendMail.mockRejectedValue(new Error('Invalid credentials'));

      const result = await emailNotifier.notify(mockNotificationEvent);

      expect(result.success).toBe(false);
      expect(result.error).toContain('Invalid credentials');
    });

    it('should generate appropriate subject line for price decrease', async () => {
      mockTransporter.sendMail.mockResolvedValue({
        messageId: '<test-message-id>',
        accepted: ['user@example.com'],
        rejected: [],
      });

      await emailNotifier.notify({
        ...mockNotificationEvent,
        changeType: 'decreased',
      });

      const emailCall = mockTransporter.sendMail.mock.calls[0][0];
      expect(emailCall.subject).toContain('Price Drop');
      expect(emailCall.subject).toContain('Test Product');
    });

    it('should generate appropriate subject line for price increase', async () => {
      mockTransporter.sendMail.mockResolvedValue({
        messageId: '<test-message-id>',
        accepted: ['user@example.com'],
        rejected: [],
      });

      await emailNotifier.notify({
        ...mockNotificationEvent,
        changeType: 'increased',
      });

      const emailCall = mockTransporter.sendMail.mock.calls[0][0];
      expect(emailCall.subject).toContain('Price Alert');
      expect(emailCall.subject).toContain('Test Product');
    });

    it('should include plain text version of email', async () => {
      mockTransporter.sendMail.mockResolvedValue({
        messageId: '<test-message-id>',
        accepted: ['user@example.com'],
        rejected: [],
      });

      await emailNotifier.notify(mockNotificationEvent);

      const emailCall = mockTransporter.sendMail.mock.calls[0][0];
      expect(emailCall.text).toBeDefined();
      expect(emailCall.text).toContain('Test Product');
      expect(emailCall.text).toContain('$99.99');
      expect(emailCall.text).toContain('decreased');
      // Plain text should not contain HTML tags
      expect(emailCall.text).not.toMatch(/<[^>]+>/);
    });
  });

  describe('test', () => {
    it('should test valid configuration successfully', async () => {
      mockTransporter.verify.mockResolvedValue(true);
      mockTransporter.sendMail.mockResolvedValue({
        messageId: '<test-message-id>',
        accepted: ['user@example.com'],
        rejected: [],
      });

      const result = await emailNotifier.test(validConfig);

      expect(result).toBe(true);
      expect(mockTransporter.verify).toHaveBeenCalled();
      expect(mockTransporter.sendMail).toHaveBeenCalledWith({
        from: validConfig.from,
        to: validConfig.to,
        subject: 'Price Tracker - Test Email',
        text: expect.stringContaining('This is a test email'),
        html: expect.stringContaining('This is a test email'),
      });
    });

    it('should fail test with invalid configuration', async () => {
      mockTransporter.verify.mockRejectedValue(new Error('Connection failed'));

      const result = await emailNotifier.test(validConfig);

      expect(result).toBe(false);
    });

    it('should fail test when send fails', async () => {
      mockTransporter.verify.mockResolvedValue(true);
      mockTransporter.sendMail.mockRejectedValue(new Error('Send failed'));

      const result = await emailNotifier.test(validConfig);

      expect(result).toBe(false);
    });
  });

  describe('getConfigSchema', () => {
    it('should return valid configuration schema', () => {
      const schema = emailNotifier.getConfigSchema();

      expect(schema.fields).toBeDefined();
      expect(Array.isArray(schema.fields)).toBe(true);
      expect(schema.fields.length).toBeGreaterThan(0);

      // Check for required fields
      const requiredFields = ['host', 'port', 'user', 'pass', 'from', 'to'];
      requiredFields.forEach(fieldName => {
        const field = schema.fields.find(f => f.name === fieldName);
        expect(field, `Field ${fieldName} should exist`).toBeDefined();
        expect(field.required, `Field ${fieldName} should be required`).toBe(true);
      });
    });
  });

  describe('validateConfig', () => {
    it('should validate complete valid configuration', () => {
      const result = emailNotifier.validateConfig(validConfig);
      expect(result).toBe(true);
    });

    it('should reject configuration missing host', () => {
      const invalidConfig = {
        ...validConfig,
        host: undefined,
      };

      const result = emailNotifier.validateConfig(invalidConfig);
      expect(result).toBe(false);
    });

    it('should reject configuration missing auth', () => {
      const invalidConfig = {
        ...validConfig,
        auth: undefined,
      };

      const result = emailNotifier.validateConfig(invalidConfig);
      expect(result).toBe(false);
    });

    it('should reject configuration with invalid email format', () => {
      const invalidConfig = {
        ...validConfig,
        from: 'invalid-email',
      };

      const result = emailNotifier.validateConfig(invalidConfig);
      expect(result).toBe(false);
    });

    it('should reject configuration with invalid port', () => {
      const invalidConfig = {
        ...validConfig,
        port: 'not-a-number',
      };

      const result = emailNotifier.validateConfig(invalidConfig);
      expect(result).toBe(false);
    });

    it('should accept configuration with SSL enabled', () => {
      const sslConfig = {
        ...validConfig,
        port: 465,
        secure: true,
      };

      const result = emailNotifier.validateConfig(sslConfig);
      expect(result).toBe(true);
    });

    it('should accept configuration with multiple recipients', () => {
      const multiRecipientConfig = {
        ...validConfig,
        to: 'user1@example.com, user2@example.com',
      };

      const result = emailNotifier.validateConfig(multiRecipientConfig);
      expect(result).toBe(true);
    });
  });
});