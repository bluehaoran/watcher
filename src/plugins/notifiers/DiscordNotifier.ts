import { NotifierPlugin, NotificationEvent, NotificationResult } from '../base/NotifierPlugin';
import { ConfigSchema } from '../base/TrackerPlugin';
import { logger } from '../../utils/logger';
import { config } from '../../utils/config';

interface DiscordEmbed {
  title: string;
  description?: string;
  color: number;
  fields: Array<{
    name: string;
    value: string;
    inline?: boolean;
  }>;
  footer?: {
    text: string;
  };
  timestamp?: string;
  image?: {
    url: string;
  };
}

interface DiscordWebhookPayload {
  content?: string;
  embeds: DiscordEmbed[];
}

export class DiscordNotifier extends NotifierPlugin {
  name = 'Discord Notifier';
  type = 'discord';
  description = 'Send notifications to Discord channels via webhooks';

  private webhookUrl: string | null = null;

  async initialize(discordConfig: any): Promise<void> {
    this.webhookUrl = discordConfig.webhookUrl || config.discordWebhook;
    
    if (!this.webhookUrl) {
      throw new Error('Discord webhook URL is required');
    }

    // Validate webhook URL format
    if (!this.webhookUrl.includes('discord.com/api/webhooks/')) {
      throw new Error('Invalid Discord webhook URL format');
    }

    logger.info('Discord notifier initialized successfully');
  }

  async notify(event: NotificationEvent): Promise<NotificationResult> {
    if (!this.webhookUrl) {
      return {
        success: false,
        error: 'Discord notifier not initialized'
      };
    }

    try {
      const payload = this.generateDiscordPayload(event);
      
      const response = await fetch(this.webhookUrl, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(payload)
      });

      if (!response.ok) {
        const errorText = await response.text();
        throw new Error(`Discord API error: ${response.status} - ${errorText}`);
      }

      logger.info(`Discord notification sent for product ${event.product.name}`);
      
      return {
        success: true,
        messageId: response.headers.get('x-ratelimit-reset') || undefined
      };

    } catch (error) {
      logger.error('Failed to send Discord notification:', error);
      return {
        success: false,
        error: error instanceof Error ? error.message : String(error)
      };
    }
  }

  async test(discordConfig: any): Promise<boolean> {
    try {
      await this.initialize(discordConfig);
      
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
      logger.error('Discord notifier test failed:', error);
      return false;
    }
  }

  getConfigSchema(): ConfigSchema {
    return {
      fields: [
        {
          name: 'webhookUrl',
          type: 'text',
          label: 'Discord Webhook URL',
          required: true,
          default: ''
        },
        {
          name: 'username',
          type: 'text',
          label: 'Bot Username (optional)',
          required: false,
          default: 'Price Tracker'
        },
        {
          name: 'mentionRole',
          type: 'text',
          label: 'Role to Mention (optional)',
          required: false,
          default: ''
        },
        {
          name: 'includeImage',
          type: 'checkbox',
          label: 'Include Screenshot (if available)',
          required: false,
          default: true
        }
      ]
    };
  }

  validateConfig(discordConfig: any): boolean {
    if (!discordConfig) return false;
    
    if (!discordConfig.webhookUrl) return false;
    
    // Basic webhook URL validation
    if (!discordConfig.webhookUrl.includes('discord.com/api/webhooks/')) {
      return false;
    }
    
    return true;
  }

  private generateDiscordPayload(event: NotificationEvent): DiscordWebhookPayload {
    const { product, source, comparison, changeType, formattedOld, formattedNew, difference } = event;
    
    const embed: DiscordEmbed = {
      title: `🔔 ${product.name}`,
      color: this.getColorForChangeType(changeType),
      fields: [],
      footer: {
        text: 'Price Tracker'
      },
      timestamp: new Date().toISOString()
    };

    // Main price change info
    embed.fields.push({
      name: '💰 Price Change',
      value: `${formattedOld} → **${formattedNew}**\nDifference: ${difference}`,
      inline: false
    });

    if (comparison) {
      // Best deal information
      embed.fields.push({
        name: '🏆 Best Deal',
        value: `**${comparison.best.storeName}**: ${comparison.best.formattedValue}\n[View Deal](${comparison.best.url})`,
        inline: true
      });

      // Savings information
      if (comparison.savings) {
        const savingsEmoji = comparison.savings.percentage >= 20 ? '🔥' : '💸';
        embed.fields.push({
          name: `${savingsEmoji} Savings`,
          value: `${comparison.savings.amount.toFixed(2)} (${comparison.savings.percentage.toFixed(1)}%)`,
          inline: true
        });
      }

      // All sources comparison
      const sourcesList = comparison.allSources
        .slice(0, 5) // Limit to 5 sources to avoid Discord embed limits
        .map(s => `• **${s.storeName}**: ${s.formattedValue} ${s.changed ? '✓' : ''}`)
        .join('\n');
      
      embed.fields.push({
        name: '📊 All Sources',
        value: sourcesList + (comparison.allSources.length > 5 ? '\n... and more' : ''),
        inline: false
      });

    } else if (source) {
      embed.fields.push({
        name: '🏪 Store',
        value: `[${source.storeName}](${source.url})`,
        inline: true
      });
    }

    // Action buttons as links
    const actionLinks = [
      `[View Product](${event.actionUrls.viewProduct})`,
      `[Purchased](${event.actionUrls.purchased})`,
      `[False Positive](${event.actionUrls.falsePositive})`,
      `[Dismiss](${event.actionUrls.dismiss})`
    ].join(' • ');

    embed.fields.push({
      name: '⚡ Actions',
      value: actionLinks,
      inline: false
    });

    // Add screenshot if available
    if (event.screenshot) {
      embed.image = {
        url: event.screenshot.startsWith('http') ? event.screenshot : `data:image/png;base64,${event.screenshot}`
      };
    }

    const payload: DiscordWebhookPayload = {
      embeds: [embed]
    };

    // Add mention if configured
    // Note: This would need to be passed in via config
    // if (discordConfig.mentionRole) {
    //   payload.content = `<@&${discordConfig.mentionRole}>`;
    // }

    return payload;
  }

  private getColorForChangeType(changeType: string): number {
    switch (changeType) {
      case 'decreased':
        return 0x28a745; // Green
      case 'increased':
        return 0xdc3545; // Red
      default:
        return 0x007bff; // Blue
    }
  }

  private formatChangeIcon(changeType: string): string {
    switch (changeType) {
      case 'decreased':
        return '📉';
      case 'increased':
        return '📈';
      default:
        return '🔔';
    }
  }
}