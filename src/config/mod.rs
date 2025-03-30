use crate::error::{AppError, AppResult};
use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub database_type: String,
    pub database_url: String,
    pub sqlite_database_url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct QueueConfig {
    pub max_concurrent_tasks: usize,
    pub task_timeout_seconds: u64,
    pub retry_max_attempts: u32,
    pub retry_initial_interval_ms: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub queue: QueueConfig,
}

impl AppConfig {
    pub fn from_env() -> AppResult<Self> {
        // Load .env file if it exists
        dotenv::dotenv().ok();

        // Initialize configuration
        let config_builder = Config::builder()
            // Start with default configuration
            .set_default("server.host", "127.0.0.1")?
            .set_default("server.port", 8080)?
            .set_default("database.database_type", "postgres")?
            .set_default("database.database_url", "postgres://postgres:postgres@localhost:5432/taskqueue")?
            .set_default("database.sqlite_database_url", "sqlite:./taskqueue.db")?
            .set_default("queue.max_concurrent_tasks", 10)?
            .set_default("queue.task_timeout_seconds", 300)?
            .set_default("queue.retry_max_attempts", 3)?
            .set_default("queue.retry_initial_interval_ms", 1000)?
            // Add configuration from config.toml if it exists
            .add_source(File::with_name("config").required(false))
            // Add configuration from environment variables (with prefix APP_)
            .add_source(Environment::with_prefix("APP").separator("_"));

        // Build the configuration
        let config = config_builder.build()?;

        // Deserialize the configuration into AppConfig
        let app_config: AppConfig = config.try_deserialize()?;

        Ok(app_config)
    }

    pub fn get_database_url(&self) -> &str {
        match self.database.database_type.as_str() {
            "sqlite" => &self.database.sqlite_database_url,
            _ => &self.database.database_url,
        }
    }
}

impl From<ConfigError> for AppError {
    fn from(error: ConfigError) -> Self {
        AppError::ConfigError(error.to_string())
    }
}