import nodemailer, { Transporter } from 'nodemailer';
import { NotifierPlugin, NotificationEvent, NotificationResult } from '../base/NotifierPlugin';
import { ConfigSchema } from '../base/TrackerPlugin';
import { logger } from '../../utils/logger';
import { config } from '../../utils/config';

export class EmailNotifier extends NotifierPlugin {
  name = 'Email Notifier';
  type = 'email';
  description = 'Send notifications via email using SMTP';

  private transporter: Transporter | null = null;

  async initialize(emailConfig: any): Promise<void> {
    const smtpConfig = {
      host: emailConfig.host || config.smtpHost,
      port: emailConfig.port || config.smtpPort || 587,
      secure: emailConfig.secure || config.smtpSecure || false,
      auth: {
        user: emailConfig.user || config.smtpUser,
        pass: emailConfig.pass || config.smtpPass,
      },
    };

    if (!smtpConfig.host || !smtpConfig.auth.user || !smtpConfig.auth.pass) {
      throw new Error('Email configuration is incomplete. Please provide SMTP host, user, and password.');
    }

    this.transporter = nodemailer.createTransport(smtpConfig);
    
    try {
      await this.transporter?.verify();
      logger.info('Email notifier initialized successfully');
    } catch (error) {
      logger.error('Failed to initialize email notifier:', error);
      throw error;
    }
  }

  async notify(event: NotificationEvent): Promise<NotificationResult> {
    if (!this.transporter) {
      return {
        success: false,
        error: 'Email notifier not initialized'
      };
    }

    try {
      const subject = this.generateSubject(event);
      const html = this.generateEmailHtml(event);
      const text = this.generateEmailText(event);

      const mailOptions = {
        from: config.smtpFrom || config.smtpUser,
        to: event.product.id, // This should be replaced with actual email addresses
        subject,
        text,
        html
      };

      const result = await this.transporter.sendMail(mailOptions);
      
      logger.info(`Email notification sent for product ${event.product.name}:`, result.messageId);
      
      return {
        success: true,
        messageId: result.messageId
      };

    } catch (error) {
      logger.error('Failed to send email notification:', error);
      return {
        success: false,
        error: error instanceof Error ? error.message : String(error)
      };
    }
  }

  async test(emailConfig: any): Promise<boolean> {
    try {
      await this.initialize(emailConfig);
      
      const testEvent: NotificationEvent = {
        product: { id: 'test', name: 'Test Product' },
        source: { id: 'test', url: 'https://example.com', storeName: 'Test Store' },
        changeType: 'decreased',
        oldValue: { amount: 100, currency: 'USD' },
        newValue: { amount: 90, currency: 'USD' },
        formattedOld: '$100.00',
        formattedNew: '$90.00',
        difference: '$10.00',
        actionUrls: {
          dismiss: `${config.baseUrl}/actions/dismiss/test`,
          falsePositive: `${config.baseUrl}/actions/false-positive/test`,
          purchased: `${config.baseUrl}/actions/purchased/test`,
          viewProduct: `${config.baseUrl}/products/test`
        }
      };

      const result = await this.notify(testEvent);
      return result.success;

    } catch (error) {
      logger.error('Email notifier test failed:', error);
      return false;
    }
  }

  getConfigSchema(): ConfigSchema {
    return {
      fields: [
        {
          name: 'host',
          type: 'text',
          label: 'SMTP Host',
          required: true,
          default: 'smtp.gmail.com'
        },
        {
          name: 'port',
          type: 'number',
          label: 'SMTP Port',
          required: true,
          default: 587
        },
        {
          name: 'secure',
          type: 'checkbox',
          label: 'Use SSL/TLS',
          required: false,
          default: false
        },
        {
          name: 'user',
          type: 'text',
          label: 'Email Username',
          required: true,
          default: ''
        },
        {
          name: 'pass',
          type: 'text',
          label: 'Email Password',
          required: true,
          default: ''
        },
        {
          name: 'from',
          type: 'text',
          label: 'From Address',
          required: false,
          default: ''
        },
        {
          name: 'to',
          type: 'text',
          label: 'To Address(es)',
          required: true,
          default: ''
        }
      ]
    };
  }

  validateConfig(emailConfig: any): boolean {
    if (!emailConfig) return false;
    
    const required = ['host', 'port', 'user', 'pass', 'to'];
    for (const field of required) {
      if (!emailConfig[field]) return false;
    }
    
    // Validate port number
    const port = parseInt(emailConfig.port);
    if (isNaN(port) || port < 1 || port > 65535) return false;
    
    // Basic email validation
    const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
    if (!emailRegex.test(emailConfig.to)) return false;
    
    return true;
  }

  private generateSubject(event: NotificationEvent): string {
    const { product, changeType, formattedNew } = event;
    
    if (event.comparison) {
      const savings = event.comparison.savings;
      if (savings && changeType === 'decreased') {
        return `💰 ${product.name} - Price Drop! Save ${savings.percentage.toFixed(1)}% (${formattedNew})`;
      }
    }
    
    switch (changeType) {
      case 'decreased':
        return `📉 ${product.name} - Price Decreased to ${formattedNew}`;
      case 'increased':
        return `📈 ${product.name} - Price Increased to ${formattedNew}`;
      default:
        return `🔔 ${product.name} - Price Changed to ${formattedNew}`;
    }
  }

  private generateEmailHtml(event: NotificationEvent): string {
    const { product, source, comparison, changeType, formattedOld, formattedNew, difference, actionUrls } = event;
    
    const changeIcon = changeType === 'decreased' ? '📉' : changeType === 'increased' ? '📈' : '🔔';
    const changeColor = changeType === 'decreased' ? '#28a745' : changeType === 'increased' ? '#dc3545' : '#6c757d';
    
    let html = `
      <!DOCTYPE html>
      <html>
      <head>
        <meta charset="utf-8">
        <style>
          body { font-family: Arial, sans-serif; line-height: 1.6; color: #333; }
          .container { max-width: 600px; margin: 0 auto; padding: 20px; }
          .header { background: #f8f9fa; padding: 20px; border-radius: 5px; margin-bottom: 20px; }
          .change { font-size: 24px; font-weight: bold; color: ${changeColor}; }
          .product { background: white; border: 1px solid #ddd; border-radius: 5px; padding: 15px; margin: 10px 0; }
          .source { font-size: 14px; color: #666; margin-bottom: 10px; }
          .actions { margin-top: 20px; text-align: center; }
          .btn { display: inline-block; padding: 10px 20px; margin: 5px; text-decoration: none; border-radius: 5px; color: white; }
          .btn-primary { background: #007bff; }
          .btn-success { background: #28a745; }
          .btn-warning { background: #ffc107; color: black; }
          .btn-secondary { background: #6c757d; }
        </style>
      </head>
      <body>
        <div class="container">
          <div class="header">
            <h2>${changeIcon} Price Alert: ${product.name}</h2>
            <div class="change">${formattedOld} → ${formattedNew}</div>
            <p>Difference: ${difference}</p>
          </div>`;

    if (comparison) {
      html += `
          <div class="product">
            <h3>🏆 Best Deal Found</h3>
            <p><strong>${comparison.best.storeName}</strong>: ${comparison.best.formattedValue}</p>
            <p><a href="${comparison.best.url}" target="_blank">View Best Deal</a></p>
            
            ${comparison.savings ? `
              <div style="background: #d4edda; border: 1px solid #c3e6cb; border-radius: 5px; padding: 10px; margin: 10px 0;">
                <strong>💰 You Save: ${comparison.savings.amount.toFixed(2)} (${comparison.savings.percentage.toFixed(1)}%)</strong>
              </div>
            ` : ''}
            
            <h4>All Sources:</h4>
            <ul>
              ${comparison.allSources.map(s => 
                `<li>${s.storeName}: ${s.formattedValue} ${s.changed ? '(Changed)' : ''}</li>`
              ).join('')}
            </ul>
          </div>`;
    } else if (source) {
      html += `
          <div class="product">
            <div class="source">Store: ${source.storeName}</div>
            <p><a href="${source.url}" target="_blank">View Product</a></p>
          </div>`;
    }

    html += `
          <div class="actions">
            <a href="${actionUrls.viewProduct}" class="btn btn-primary">View Product</a>
            <a href="${actionUrls.purchased}" class="btn btn-success">Mark as Purchased</a>
            <a href="${actionUrls.falsePositive}" class="btn btn-warning">False Positive</a>
            <a href="${actionUrls.dismiss}" class="btn btn-secondary">Dismiss</a>
          </div>
          
          <p style="font-size: 12px; color: #666; text-align: center; margin-top: 30px;">
            This email was sent by Price Tracker. 
            <a href="${actionUrls.dismiss}">Unsubscribe from this product</a>
          </p>
        </div>
      </body>
      </html>`;
    
    return html;
  }

  private generateEmailText(event: NotificationEvent): string {
    const { product, source, comparison, changeType, formattedOld, formattedNew, difference, actionUrls } = event;
    
    let text = `Price Alert: ${product.name}\n\n`;
    text += `${formattedOld} → ${formattedNew}\n`;
    text += `Difference: ${difference}\n`;
    text += `Change Type: ${changeType}\n\n`;
    
    if (comparison) {
      text += `Best Deal: ${comparison.best.storeName} - ${comparison.best.formattedValue}\n`;
      text += `Best Deal URL: ${comparison.best.url}\n\n`;
      
      if (comparison.savings) {
        text += `You Save: ${comparison.savings.amount.toFixed(2)} (${comparison.savings.percentage.toFixed(1)}%)\n\n`;
      }
      
      text += `All Sources:\n`;
      comparison.allSources.forEach(s => {
        text += `- ${s.storeName}: ${s.formattedValue} ${s.changed ? '(Changed)' : ''}\n`;
      });
    } else if (source) {
      text += `Store: ${source.storeName}\n`;
      text += `URL: ${source.url}\n`;
    }
    
    text += `\nActions:\n`;
    text += `View Product: ${actionUrls.viewProduct}\n`;
    text += `Mark as Purchased: ${actionUrls.purchased}\n`;
    text += `Report False Positive: ${actionUrls.falsePositive}\n`;
    text += `Dismiss: ${actionUrls.dismiss}\n`;
    
    return text;
  }
}