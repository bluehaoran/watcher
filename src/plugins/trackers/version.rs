use crate::plugins::traits::{
    TrackerPlugin, ParseResult, ComparisonResult, ElementMatch, ConfigSchema, ChangeType,
};
use crate::plugins::traits::tracker::{ConfigField, ConfigFieldType};
use async_trait::async_trait;
use regex::Regex;
use serde_json::json;
use std::collections::HashMap;
use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Version {
    major: u32,
    minor: u32,
    patch: u32,
    pre_release: Option<String>,
}

impl Version {
    fn parse(version_str: &str) -> Option<Self> {
        let version_regex = Regex::new(r"^(\d+)\.(\d+)\.(\d+)(?:-(.+))?$").unwrap();
        if let Some(captures) = version_regex.captures(version_str) {
            let major = captures[1].parse().ok()?;
            let minor = captures[2].parse().ok()?;
            let patch = captures[3].parse().ok()?;
            let pre_release = captures.get(4).map(|m| m.as_str().to_string());
            
            Some(Version {
                major,
                minor,
                patch,
                pre_release,
            })
        } else {
            None
        }
    }
    
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(pre) = &self.pre_release {
            write!(f, "{}.{}.{}-{}", self.major, self.minor, self.patch, pre)
        } else {
            write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
        }
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.major.cmp(&other.major) {
            Ordering::Equal => match self.minor.cmp(&other.minor) {
                Ordering::Equal => match self.patch.cmp(&other.patch) {
                    Ordering::Equal => {
                        // Pre-release versions have lower precedence
                        match (&self.pre_release, &other.pre_release) {
                            (None, None) => Ordering::Equal,
                            (None, Some(_)) => Ordering::Greater,
                            (Some(_), None) => Ordering::Less,
                            (Some(a), Some(b)) => a.cmp(b),
                        }
                    }
                    other => other,
                }
                other => other,
            }
            other => other,
        }
    }
}

pub struct VersionTracker {
    version_regex: Regex,
}

impl Default for VersionTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl VersionTracker {
    pub fn new() -> Self {
        VersionTracker {
            version_regex: Regex::new(r"v?(\d+\.\d+\.\d+(?:-[a-zA-Z0-9.-]+)?)").unwrap(),
        }
    }
    
    fn extract_version(&self, text: &str) -> Option<Version> {
        if let Some(captures) = self.version_regex.captures(text) {
            let version_str = captures.get(1)?.as_str();
            Version::parse(version_str)
        } else {
            None
        }
    }
}

#[async_trait]
impl TrackerPlugin for VersionTracker {
    fn name(&self) -> &str {
        "Version Tracker"
    }
    
    fn plugin_type(&self) -> &str {
        "version"
    }
    
    fn description(&self) -> &str {
        "Tracks semantic version changes on web pages"
    }
    
    fn parse(&self, text: &str) -> ParseResult {
        if let Some(version) = self.extract_version(text) {
            let value = json!({
                "version": version.to_string(),
                "major": version.major,
                "minor": version.minor,
                "patch": version.patch,
                "pre_release": version.pre_release
            });
            
            ParseResult {
                success: true,
                value,
                normalized: version.to_string(),
                confidence: 0.95,
                metadata: HashMap::new(),
            }
        } else {
            ParseResult {
                success: false,
                value: json!(null),
                normalized: "".to_string(),
                confidence: 0.0,
                metadata: HashMap::new(),
            }
        }
    }
    
    fn format(&self, value: &serde_json::Value) -> String {
        if let Some(version_str) = value.get("version") {
            return version_str.as_str().unwrap_or("N/A").to_string();
        }
        "N/A".to_string()
    }
    
    fn compare(&self, old_value: &serde_json::Value, new_value: &serde_json::Value) -> ComparisonResult {
        let old_version_str = old_value.get("version").and_then(|v| v.as_str()).unwrap_or("");
        let new_version_str = new_value.get("version").and_then(|v| v.as_str()).unwrap_or("");
        
        if let (Some(old_version), Some(new_version)) = (
            Version::parse(old_version_str),
            Version::parse(new_version_str)
        ) {
            let change_type = match new_version.cmp(&old_version) {
                Ordering::Greater => ChangeType::Increased,
                Ordering::Less => ChangeType::Decreased,
                Ordering::Equal => ChangeType::Unchanged,
            };
            
            ComparisonResult {
                changed: change_type != ChangeType::Unchanged,
                change_type,
                difference: json!({
                    "from": old_version.to_string(),
                    "to": new_version.to_string()
                }),
                percent_change: None, // Not applicable for versions
            }
        } else {
            ComparisonResult {
                changed: old_version_str != new_version_str,
                change_type: if old_version_str != new_version_str {
                    ChangeType::Increased // Assume any change is an update
                } else {
                    ChangeType::Unchanged
                },
                difference: json!({
                    "from": old_version_str,
                    "to": new_version_str
                }),
                percent_change: None,
            }
        }
    }
    
    fn get_search_variations(&self, input: &str) -> Vec<String> {
        let mut variations = vec![input.to_string()];
        
        // Add variations with 'v' prefix
        variations.push(format!("v{}", input));
        
        // Add variations with common version patterns
        if input.contains('.') {
            variations.push(format!("version {}", input));
            variations.push(format!("Version {}", input));
            variations.push(format!("VERSION {}", input));
        }
        
        variations
    }
    
    fn rank_matches(&self, _input: &str, matches: &[ElementMatch]) -> Vec<ElementMatch> {
        let mut ranked = matches.to_vec();
        
        // Rank by confidence, then by whether text looks like a version
        ranked.sort_by(|a, b| {
            let a_score = a.confidence + if self.extract_version(&a.text).is_some() { 0.2 } else { 0.0 };
            let b_score = b.confidence + if self.extract_version(&b.text).is_some() { 0.2 } else { 0.0 };
            b_score.partial_cmp(&a_score).unwrap_or(Ordering::Equal)
        });
        
        ranked
    }
    
    fn get_config_schema(&self) -> ConfigSchema {
        ConfigSchema {
            fields: vec![
                ConfigField {
                    name: "include_pre_release".to_string(),
                    field_type: ConfigFieldType::Checkbox,
                    label: "Include Pre-release Versions".to_string(),
                    required: false,
                    default: Some(json!(true)),
                    options: None,
                },
            ],
        }
    }
    
    fn validate_config(&self, config: &serde_json::Value) -> bool {
        if let Some(include_pre) = config.get("include_pre_release") {
            return include_pre.is_boolean();
        }
        true
    }
    
    async fn initialize(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
    
    async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        assert_eq!(
            Version::parse("1.2.3"),
            Some(Version {
                major: 1,
                minor: 2,
                patch: 3,
                pre_release: None
            })
        );
    }

    #[test]
    fn test_version_parsing_with_prerelease() {
        assert_eq!(
            Version::parse("2.0.0-beta.1"),
            Some(Version {
                major: 2,
                minor: 0,
                patch: 0,
                pre_release: Some("beta.1".to_string())
            })
        );
    }

    #[test]
    fn test_version_comparison() {
        let v1 = Version::parse("1.0.0").unwrap();
        let v2 = Version::parse("1.1.0").unwrap();
        let v3 = Version::parse("2.0.0-alpha").unwrap();
        let v4 = Version::parse("2.0.0").unwrap();
        
        assert!(v2 > v1);
        assert!(v4 > v3); // Release > pre-release
        assert!(v4 > v2);
    }

    #[test]
    fn test_version_tracker_parse() {
        let tracker = VersionTracker::new();
        let result = tracker.parse("Version 1.2.3");
        
        assert!(result.success);
        assert_eq!(result.value["version"], "1.2.3");
        assert_eq!(result.value["major"], 1);
        assert_eq!(result.value["minor"], 2);
        assert_eq!(result.value["patch"], 3);
    }

    #[test]
    fn test_version_tracker_parse_with_v_prefix() {
        let tracker = VersionTracker::new();
        let result = tracker.parse("v2.1.0-beta");
        
        assert!(result.success);
        assert_eq!(result.value["version"], "2.1.0-beta");
        assert_eq!(result.value["pre_release"], "beta");
    }

    #[test]
    fn test_version_tracker_parse_failure() {
        let tracker = VersionTracker::new();
        let result = tracker.parse("not a version");
        
        assert!(!result.success);
        assert_eq!(result.confidence, 0.0);
    }

    #[test]
    fn test_version_formatting() {
        let tracker = VersionTracker::new();
        let value = json!({
            "version": "1.5.2",
            "major": 1,
            "minor": 5,
            "patch": 2
        });
        
        let formatted = tracker.format(&value);
        assert_eq!(formatted, "1.5.2");
    }

    #[test]
    fn test_version_comparison_upgrade() {
        let tracker = VersionTracker::new();
        let old_version = json!({
            "version": "1.0.0"
        });
        let new_version = json!({
            "version": "1.1.0"
        });
        
        let result = tracker.compare(&old_version, &new_version);
        assert!(result.changed);
        assert!(matches!(result.change_type, ChangeType::Increased));
    }

    #[test]
    fn test_search_variations() {
        let tracker = VersionTracker::new();
        let variations = tracker.get_search_variations("1.2.3");
        
        assert!(variations.contains(&"1.2.3".to_string()));
        assert!(variations.contains(&"v1.2.3".to_string()));
        assert!(variations.contains(&"version 1.2.3".to_string()));
    }

    #[test]
    fn test_config_validation() {
        let tracker = VersionTracker::new();
        let valid_config = json!({"include_pre_release": true});
        let invalid_config = json!({"include_pre_release": "yes"});
        
        assert!(tracker.validate_config(&valid_config));
        assert!(!tracker.validate_config(&invalid_config));
    }
}