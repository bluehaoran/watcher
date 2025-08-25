import { TrackerPlugin } from './base/TrackerPlugin';
import { NotifierPlugin } from './base/NotifierPlugin';

export class PluginManager {
  private trackerPlugins = new Map<string, TrackerPlugin>();
  private notifierPlugins = new Map<string, NotifierPlugin>();

  registerTracker(plugin: TrackerPlugin): void {
    this.trackerPlugins.set(plugin.type, plugin);
  }

  registerNotifier(plugin: NotifierPlugin): void {
    this.notifierPlugins.set(plugin.type, plugin);
  }

  getTracker(type: string): TrackerPlugin | undefined {
    return this.trackerPlugins.get(type);
  }

  getNotifier(type: string): NotifierPlugin | undefined {
    return this.notifierPlugins.get(type);
  }

  getAvailableTrackers(): TrackerPlugin[] {
    return Array.from(this.trackerPlugins.values());
  }

  getAvailableNotifiers(): NotifierPlugin[] {
    return Array.from(this.notifierPlugins.values());
  }

  async loadDefaultPlugins(): Promise<void> {
    // Dynamic imports for default plugins
    const { PriceTracker } = await import('./trackers/PriceTracker.js');
    const { VersionTracker } = await import('./trackers/VersionTracker.js');
    const { NumberTracker } = await import('./trackers/NumberTracker.js');
    const { EmailNotifier } = await import('./notifiers/EmailNotifier.js');
    const { DiscordNotifier } = await import('./notifiers/DiscordNotifier.js');

    // Register tracker plugins
    this.registerTracker(new PriceTracker());
    this.registerTracker(new VersionTracker());
    this.registerTracker(new NumberTracker());

    // Register notifier plugins
    this.registerNotifier(new EmailNotifier());
    this.registerNotifier(new DiscordNotifier());
  }
}