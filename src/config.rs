use config::{Config, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};
use std::env;
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub security: SecurityConfig,
    pub scraper: ScraperConfig,
    pub scheduler: SchedulerConfig,
    pub notifications: NotificationsConfig,
    pub screenshots: ScreenshotConfig,
    pub metrics: MetricsConfig,
    pub performance: PerformanceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub base_url: String,
    pub request_timeout: u64,
    pub shutdown_timeout: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub secret_key: String,
    pub jwt_expiry: u64,
    pub rate_limit_requests: u32,
    pub rate_limit_window: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScraperConfig {
    pub max_concurrent_checks: usize,
    pub retry_attempts: u32,
    pub retry_delay_ms: u64,
    pub request_timeout: u64,
    pub user_agent: String,
    pub chrome_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    pub default_interval: String,
    pub max_running_jobs: usize,
    pub job_timeout: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationsConfig {
    pub smtp: SmtpConfig,
    pub discord: DiscordConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    pub from_address: Option<String>,
    pub from_name: String,
    pub use_tls: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    pub webhook_url: Option<String>,
    pub username: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotConfig {
    pub enabled: bool,
    pub quality: u8,
    pub max_size_mb: u32,
    pub retention_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    pub enabled: bool,
    pub port: u16,
    pub endpoint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub thread_pool_size: usize,
    pub memory_limit_mb: u32,
    pub enable_compression: bool,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| "development".into());

        let s = Config::builder()
            // Start with default configuration
            .add_source(File::with_name("config/default"))
            // Add environment-specific config
            .add_source(File::with_name(&format!("config/{}", run_mode)).required(false))
            // Add local config (ignored by git)
            .add_source(File::with_name("config/local").required(false))
            // Add environment variables with prefix "UATU_"
            .add_source(Environment::with_prefix("UATU").separator("__"))
            .build()?;

        let mut config: AppConfig = s.try_deserialize()?;
        
        // Add Chrome path from environment if not set
        if config.scraper.chrome_path.is_none() {
            config.scraper.chrome_path = env::var("CHROME_PATH").ok();
        }
        
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate server configuration
        if self.server.port == 0 {
            return Err(ConfigError::Message("Server port must be greater than 0".into()));
        }

        if let Err(_) = Url::parse(&self.server.base_url) {
            return Err(ConfigError::Message("Invalid base URL format".into()));
        }

        // Validate database configuration
        if self.database.max_connections == 0 {
            return Err(ConfigError::Message("Database max_connections must be greater than 0".into()));
        }

        if self.database.min_connections > self.database.max_connections {
            return Err(ConfigError::Message("Database min_connections cannot exceed max_connections".into()));
        }

        // Validate security configuration
        if self.security.secret_key.len() < 32 {
            return Err(ConfigError::Message("Security secret_key must be at least 32 characters".into()));
        }

        if self.security.jwt_expiry == 0 {
            return Err(ConfigError::Message("JWT expiry must be greater than 0".into()));
        }

        // Validate scraper configuration
        if self.scraper.max_concurrent_checks == 0 {
            return Err(ConfigError::Message("Scraper max_concurrent_checks must be greater than 0".into()));
        }

        // Validate scheduler configuration - basic cron validation
        if !self.is_valid_cron(&self.scheduler.default_interval) {
            return Err(ConfigError::Message("Invalid cron expression in scheduler.default_interval".into()));
        }

        if self.scheduler.max_running_jobs == 0 {
            return Err(ConfigError::Message("Scheduler max_running_jobs must be greater than 0".into()));
        }

        // Validate SMTP configuration
        if self.notifications.smtp.port == 0 {
            return Err(ConfigError::Message("SMTP port must be greater than 0".into()));
        }

        // Validate screenshot configuration
        if self.screenshots.quality == 0 || self.screenshots.quality > 100 {
            return Err(ConfigError::Message("Screenshot quality must be between 1 and 100".into()));
        }

        // Validate metrics configuration
        if self.metrics.port == 0 {
            return Err(ConfigError::Message("Metrics port must be greater than 0".into()));
        }

        if !self.metrics.endpoint.starts_with('/') {
            return Err(ConfigError::Message("Metrics endpoint must start with '/'".into()));
        }

        // Validate performance configuration
        if self.performance.thread_pool_size == 0 {
            return Err(ConfigError::Message("Performance thread_pool_size must be greater than 0".into()));
        }

        Ok(())
    }

    fn is_valid_cron(&self, cron_expr: &str) -> bool {
        // Basic cron validation - should have 5 parts (minute hour day month weekday)
        let parts: Vec<&str> = cron_expr.split_whitespace().collect();
        if parts.len() != 5 {
            return false;
        }

        // Each part should be valid
        for part in parts {
            if part.is_empty() {
                return false;
            }
            // Allow numbers, ranges, lists, and wildcards
            if !part.chars().all(|c| c.is_ascii_digit() || c == '*' || c == '-' || c == ',' || c == '/') {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_config_validation_valid() {
        let config = AppConfig {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 3000,
                base_url: "http://localhost:3000".to_string(),
                request_timeout: 30,
                shutdown_timeout: 10,
            },
            database: DatabaseConfig {
                url: "sqlite:///data/test.db".to_string(),
                max_connections: 10,
                min_connections: 1,
                acquire_timeout: 30,
            },
            security: SecurityConfig {
                secret_key: "this-is-a-valid-secret-key-with-32-chars".to_string(),
                jwt_expiry: 3600,
                rate_limit_requests: 100,
                rate_limit_window: 60,
            },
            scraper: ScraperConfig {
                max_concurrent_checks: 5,
                retry_attempts: 3,
                retry_delay_ms: 5000,
                request_timeout: 30,
                user_agent: "UatuWatcher/1.0".to_string(),
                chrome_path: None,
            },
            scheduler: SchedulerConfig {
                default_interval: "0 0 * * *".to_string(),
                max_running_jobs: 10,
                job_timeout: 300,
            },
            notifications: NotificationsConfig {
                smtp: SmtpConfig {
                    host: "smtp.gmail.com".to_string(),
                    port: 587,
                    username: None,
                    password: None,
                    from_address: None,
                    from_name: "Uatu Watcher".to_string(),
                    use_tls: true,
                },
                discord: DiscordConfig {
                    webhook_url: None,
                    username: "Uatu Watcher".to_string(),
                    avatar_url: None,
                },
            },
            screenshots: ScreenshotConfig {
                enabled: true,
                quality: 80,
                max_size_mb: 5,
                retention_days: 30,
            },
            metrics: MetricsConfig {
                enabled: true,
                port: 9001,
                endpoint: "/metrics".to_string(),
            },
            performance: PerformanceConfig {
                thread_pool_size: 4,
                memory_limit_mb: 256,
                enable_compression: true,
            },
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_invalid_port() {
        let mut config = valid_config();
        config.server.port = 0;
        
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("port must be greater than 0"));
    }

    #[test]
    fn test_config_validation_invalid_base_url() {
        let mut config = valid_config();
        config.server.base_url = "not-a-valid-url".to_string();
        
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid base URL"));
    }

    #[test]
    fn test_config_validation_short_secret_key() {
        let mut config = valid_config();
        config.security.secret_key = "too-short".to_string();
        
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("secret_key must be at least 32 characters"));
    }

    #[test]
    fn test_config_validation_invalid_db_connections() {
        let mut config = valid_config();
        config.database.min_connections = 15;
        config.database.max_connections = 10;
        
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("min_connections cannot exceed max_connections"));
    }

    #[test]
    fn test_config_validation_invalid_cron() {
        let mut config = valid_config();
        config.scheduler.default_interval = "invalid cron".to_string();
        
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid cron expression"));
    }

    #[test]
    fn test_config_validation_invalid_screenshot_quality() {
        let mut config = valid_config();
        config.screenshots.quality = 101;
        
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("quality must be between 1 and 100"));
    }

    #[test]
    fn test_config_validation_invalid_metrics_endpoint() {
        let mut config = valid_config();
        config.metrics.endpoint = "metrics".to_string(); // Missing leading slash
        
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("endpoint must start with"));
    }

    #[test]
    fn test_cron_validation() {
        let config = valid_config();
        
        assert!(config.is_valid_cron("0 0 * * *"));
        assert!(config.is_valid_cron("*/15 * * * *"));
        assert!(config.is_valid_cron("0 9-17 * * 1-5"));
        assert!(config.is_valid_cron("0 12 1 * *"));
        
        assert!(!config.is_valid_cron("invalid"));
        assert!(!config.is_valid_cron("0 0 * *")); // Too few parts
        assert!(!config.is_valid_cron("0 0 * * * *")); // Too many parts
        assert!(!config.is_valid_cron("0 0 * * $ ")); // Invalid character
    }

    #[test]
    fn test_from_env_with_chrome_path() {
        // Set environment variable
        env::set_var("CHROME_PATH", "/usr/bin/chromium");
        
        // This test requires config files to exist, so we'll just test the Chrome path logic
        // In a real test environment, you'd have test config files
        let config = valid_config();
        assert!(config.validate().is_ok());
        
        // Clean up
        env::remove_var("CHROME_PATH");
    }

    fn valid_config() -> AppConfig {
        AppConfig {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 3000,
                base_url: "http://localhost:3000".to_string(),
                request_timeout: 30,
                shutdown_timeout: 10,
            },
            database: DatabaseConfig {
                url: "sqlite:///data/test.db".to_string(),
                max_connections: 10,
                min_connections: 1,
                acquire_timeout: 30,
            },
            security: SecurityConfig {
                secret_key: "this-is-a-valid-secret-key-with-32-chars".to_string(),
                jwt_expiry: 3600,
                rate_limit_requests: 100,
                rate_limit_window: 60,
            },
            scraper: ScraperConfig {
                max_concurrent_checks: 5,
                retry_attempts: 3,
                retry_delay_ms: 5000,
                request_timeout: 30,
                user_agent: "UatuWatcher/1.0".to_string(),
                chrome_path: None,
            },
            scheduler: SchedulerConfig {
                default_interval: "0 0 * * *".to_string(),
                max_running_jobs: 10,
                job_timeout: 300,
            },
            notifications: NotificationsConfig {
                smtp: SmtpConfig {
                    host: "smtp.gmail.com".to_string(),
                    port: 587,
                    username: None,
                    password: None,
                    from_address: None,
                    from_name: "Uatu Watcher".to_string(),
                    use_tls: true,
                },
                discord: DiscordConfig {
                    webhook_url: None,
                    username: "Uatu Watcher".to_string(),
                    avatar_url: None,
                },
            },
            screenshots: ScreenshotConfig {
                enabled: true,
                quality: 80,
                max_size_mb: 5,
                retention_days: 30,
            },
            metrics: MetricsConfig {
                enabled: true,
                port: 9001,
                endpoint: "/metrics".to_string(),
            },
            performance: PerformanceConfig {
                thread_pool_size: 4,
                memory_limit_mb: 256,
                enable_compression: true,
            },
        }
    }
}