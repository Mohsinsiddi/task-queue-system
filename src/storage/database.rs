use crate::error::AppResult;
use crate::models::Task;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::sync::Arc;

// Trait defining the database operations
#[async_trait]
pub trait Database: Send + Sync {
    /// Create a new task in the database
    async fn create_task(&self, task: &Task) -> AppResult<()>;
    
    /// Get a task by ID
    async fn get_task(&self, id: &str) -> AppResult<Task>;
    
    /// Update an existing task
    async fn update_task(&self, task: &Task) -> AppResult<()>;
    
    /// Delete a task by ID
    async fn delete_task(&self, id: &str) -> AppResult<()>;
    
    /// Get all tasks with optional filtering
    async fn get_tasks(
        &self,
        state: Option<&str>,
        priority: Option<&str>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> AppResult<Vec<Task>>;
    
    /// Get tasks scheduled to run before the given time
    async fn get_scheduled_tasks(&self, before: DateTime<Utc>) -> AppResult<Vec<Task>>;
    
    /// Get tasks that have failed and can be retried
    async fn get_failed_tasks_for_retry(&self) -> AppResult<Vec<Task>>;
    
    /// Count tasks by state
    async fn count_tasks_by_state(&self) -> AppResult<Vec<(String, i64)>>;
    
    /// Count tasks by priority
    async fn count_tasks_by_priority(&self) -> AppResult<Vec<(String, i64)>>;
    
    /// Setup database (create tables, etc.)
    async fn setup(&self) -> AppResult<()>;
}

// Factory function to create a database instance based on URL
pub async fn create_database(database_url: &str) -> AppResult<Arc<dyn Database>> {
    if database_url.starts_with("sqlite:") {
        let db = super::sqlite::SqliteDatabase::new(database_url).await?;
        Ok(Arc::new(db))
    } else {
        // Default to PostgreSQL
        let db = super::postgres::PostgresDatabase::new(database_url).await?;
        Ok(Arc::new(db))
    }
}