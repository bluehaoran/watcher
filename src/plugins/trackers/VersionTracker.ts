import { 
  TrackerPlugin, 
  ParseResult, 
  ComparisonResult, 
  ElementMatch, 
  ConfigSchema 
} from '../base/TrackerPlugin.js';

export class VersionTracker extends TrackerPlugin {
  name = 'Version Tracker';
  type = 'version';
  description = 'Track semantic versions (e.g., 1.20.1, 2.0.0-beta)';

  parse(text: string): ParseResult {
    const cleaned = text.trim();
    
    // Semantic version patterns
    const patterns = [
      // Full semantic version with pre-release (1.2.3-alpha.1)
      /v?(\d+)\.(\d+)\.(\d+)(?:-([a-zA-Z0-9\-\.]+))?(?:\+([a-zA-Z0-9\-\.]+))?/,
      // Major.Minor.Patch (1.2.3)
      /v?(\d+)\.(\d+)\.(\d+)/,
      // Major.Minor (1.2)
      /v?(\d+)\.(\d+)/,
      // Just major version
      /v?(\d+)(?:\s*(?:\.0\.0)?)/
    ];

    for (const pattern of patterns) {
      const match = cleaned.match(pattern);
      if (match) {
        const major = parseInt(match[1]);
        const minor = parseInt(match[2]) || 0;
        const patch = parseInt(match[3]) || 0;
        const prerelease = match[4] || null;
        const build = match[5] || null;

        const version = {
          major,
          minor,
          patch,
          prerelease,
          build,
          raw: match[0],
          originalText: text.trim()
        };

        return {
          success: true,
          value: version,
          normalized: this.formatVersion(version),
          confidence: this.calculateConfidence(text, version),
          metadata: {
            hasPrerelease: !!prerelease,
            hasBuild: !!build,
            originalFormat: match[0]
          }
        };
      }
    }

    return {
      success: false,
      value: null,
      normalized: text,
      confidence: 0
    };
  }

  format(value: any): string {
    if (!value || typeof value !== 'object') {
      return String(value || '');
    }

    return this.formatVersion(value);
  }

  compare(oldValue: any, newValue: any): ComparisonResult {
    if (!oldValue || !newValue) {
      return {
        changed: false,
        changeType: 'unchanged',
        difference: 0
      };
    }

    const comparison = this.compareVersions(oldValue, newValue);
    
    if (comparison === 0) {
      return {
        changed: false,
        changeType: 'unchanged',
        difference: 0,
        percentChange: 0
      };
    }

    return {
      changed: true,
      changeType: comparison > 0 ? 'increased' : 'decreased',
      difference: Math.abs(comparison),
      percentChange: this.calculateVersionChangePercentage(oldValue, newValue)
    };
  }

  getSearchVariations(input: string): string[] {
    const variations = new Set<string>();
    const parsed = this.parse(input);
    
    if (parsed.success && parsed.value) {
      const version = parsed.value;
      
      // Original
      variations.add(input);
      
      // Different formats
      variations.add(`v${version.major}.${version.minor}.${version.patch}`);
      variations.add(`${version.major}.${version.minor}.${version.patch}`);
      variations.add(`${version.major}.${version.minor}`);
      variations.add(`${version.major}`);
      
      // With/without 'v' prefix
      variations.add(`v${version.raw}`);
      variations.add(version.raw.replace(/^v/, ''));
      
      // With pre-release info
      if (version.prerelease) {
        variations.add(`${version.major}.${version.minor}.${version.patch}-${version.prerelease}`);
      }
    } else {
      variations.add(input);
    }
    
    return Array.from(variations);
  }

  rankMatches(input: string, matches: ElementMatch[]): ElementMatch[] {
    const parsedInput = this.parse(input);
    
    return matches.map(match => {
      let bonus = 0;
      
      // Bonus for version-related context
      if (match.html.match(/version|release|tag|build|update/i)) {
        bonus += 15;
      }
      
      // Bonus for GitHub/GitLab release contexts
      if (match.html.match(/github|gitlab|release|tag/i)) {
        bonus += 10;
      }
      
      // Bonus for semantic version format
      const matchParsed = this.parse(match.text);
      if (matchParsed.success) {
        bonus += 20;
        
        // Extra bonus if versions match exactly
        if (parsedInput.success && this.compareVersions(parsedInput.value, matchParsed.value) === 0) {
          bonus += 25;
        }
      }
      
      return {
        ...match,
        confidence: Math.min(100, match.confidence + bonus)
      };
    }).sort((a, b) => b.confidence - a.confidence);
  }

  getConfigSchema(): ConfigSchema {
    return {
      fields: [
        {
          name: 'trackPrerelease',
          type: 'checkbox',
          label: 'Track Pre-release Versions',
          required: false,
          default: true
        },
        {
          name: 'notificationLevel',
          type: 'select',
          label: 'Notify On',
          required: false,
          default: 'minor',
          options: [
            { value: 'major', label: 'Major versions only (1.x.x → 2.x.x)' },
            { value: 'minor', label: 'Minor versions and above (x.1.x → x.2.x)' },
            { value: 'patch', label: 'All version changes' }
          ]
        },
        {
          name: 'ignoreDowngrades',
          type: 'checkbox',
          label: 'Ignore Version Downgrades',
          required: false,
          default: true
        }
      ]
    };
  }

  validateConfig(config: any): boolean {
    if (!config) return true;
    
    if (config.notificationLevel && !['major', 'minor', 'patch'].includes(config.notificationLevel)) {
      return false;
    }
    
    return true;
  }

  private formatVersion(version: any): string {
    if (!version) return '';
    
    let formatted = `${version.major}.${version.minor}.${version.patch}`;
    
    if (version.prerelease) {
      formatted += `-${version.prerelease}`;
    }
    
    if (version.build) {
      formatted += `+${version.build}`;
    }
    
    return formatted;
  }

  private compareVersions(v1: any, v2: any): number {
    if (!v1 || !v2) return 0;
    
    // Compare major
    if (v1.major !== v2.major) {
      return v2.major - v1.major;
    }
    
    // Compare minor
    if (v1.minor !== v2.minor) {
      return v2.minor - v1.minor;
    }
    
    // Compare patch
    if (v1.patch !== v2.patch) {
      return v2.patch - v1.patch;
    }
    
    // Compare prerelease
    return this.comparePrereleases(v1.prerelease, v2.prerelease);
  }

  private comparePrereleases(pre1: string | null, pre2: string | null): number {
    // No prerelease is higher than any prerelease
    if (!pre1 && !pre2) return 0;
    if (!pre1 && pre2) return 1;
    if (pre1 && !pre2) return -1;
    
    // Both have prereleases, compare lexicographically
    if (pre1! > pre2!) return 1;
    if (pre1! < pre2!) return -1;
    return 0;
  }

  private calculateVersionChangePercentage(oldVersion: any, newVersion: any): number {
    if (!oldVersion || !newVersion) return 0;
    
    // Simple heuristic for version change magnitude
    const oldWeight = oldVersion.major * 10000 + oldVersion.minor * 100 + oldVersion.patch;
    const newWeight = newVersion.major * 10000 + newVersion.minor * 100 + newVersion.patch;
    
    if (oldWeight === 0) return 100;
    
    return Math.abs((newWeight - oldWeight) / oldWeight) * 100;
  }

  private calculateConfidence(text: string, version: any): number {
    let confidence = 60; // Base confidence for valid semantic version
    
    // Bonus for complete semantic version (x.y.z)
    if (version.patch !== undefined && version.patch !== null) {
      confidence += 15;
    }
    
    // Bonus for 'v' prefix
    if (text.startsWith('v') || text.startsWith('V')) {
      confidence += 10;
    }
    
    // Bonus for prerelease info
    if (version.prerelease) {
      confidence += 5;
    }
    
    // Penalty for very high version numbers (might not be versions)
    if (version.major > 100) {
      confidence -= 15;
    }
    
    // Context bonus
    if (text.match(/version|release|tag|build/i)) {
      confidence += 10;
    }
    
    return Math.max(0, Math.min(100, confidence));
  }
}