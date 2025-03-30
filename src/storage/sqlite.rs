use crate::error::{AppError, AppResult};
use crate::models::{Task, TaskPriority, TaskState};
use crate::storage::database::Database;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use log::{info, warn};
use sqlx::{sqlite::SqlitePoolOptions, Row, SqlitePool};
use std::time::Duration;

pub struct SqliteDatabase {
    pool: SqlitePool,
}

impl SqliteDatabase {
    pub async fn new(database_url: &str) -> AppResult<Self> {
        // Add connection timeout and retry logic
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(5))
            .connect(database_url)
            .await
            .map_err(|e| {
                warn!("SQLite connection error: {}", e);
                AppError::DatabaseError(e)
            })?;

        Ok(Self { pool })
    }
}

#[async_trait]
impl Database for SqliteDatabase {
    async fn create_task(&self, task: &Task) -> AppResult<()> {
        let tags = serde_json::to_string(&task.tags).unwrap_or_else(|_| "[]".to_string());
        
        sqlx::query(
            r#"
            INSERT INTO tasks (
                id, name, payload, state, priority,
                created_at, updated_at, scheduled_at,
                started_at, completed_at, attempts,
                max_attempts, last_error, worker_id,
                result, tags
            ) VALUES (
                ?, ?, ?, ?, ?,
                ?, ?, ?, ?, ?,
                ?, ?, ?, ?, ?,
                ?
            )
            "#
        )
        .bind(&task.id)
        .bind(&task.name)
        .bind(&task.payload.to_string())
        .bind(&task.state.to_string())
        .bind(&task.priority.to_string())
        .bind(task.created_at.timestamp())
        .bind(task.updated_at.timestamp())
        .bind(task.scheduled_at.map(|dt| dt.timestamp()))
        .bind(task.started_at.map(|dt| dt.timestamp()))
        .bind(task.completed_at.map(|dt| dt.timestamp()))
        .bind(task.attempts as i32)
        .bind(task.max_attempts as i32)
        .bind(&task.last_error)
        .bind(&task.worker_id)
        .bind(task.result.as_ref().map(|r| r.to_string()))
        .bind(&tags)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e))?;

        Ok(())
    }

    async fn get_task(&self, id: &str) -> AppResult<Task> {
        let row = sqlx::query(
            r#"
            SELECT
                id, name, payload, state, priority,
                created_at, updated_at, scheduled_at,
                started_at, completed_at, attempts,
                max_attempts, last_error, worker_id,
                result, tags
            FROM tasks
            WHERE id = ?
            "#
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => AppError::TaskNotFound(id.to_string()),
            e => AppError::DatabaseError(e),
        })?;

        let id: String = row.try_get("id")?;
        let name: String = row.try_get("name")?;
        let payload_str: String = row.try_get("payload")?;
        let state_str: String = row.try_get("state")?;
        let priority_str: String = row.try_get("priority")?;
        let created_at: i64 = row.try_get("created_at")?;
        let updated_at: i64 = row.try_get("updated_at")?;
        let scheduled_at: Option<i64> = row.try_get("scheduled_at")?;
        let started_at: Option<i64> = row.try_get("started_at")?;
        let completed_at: Option<i64> = row.try_get("completed_at")?;
        let attempts: i32 = row.try_get("attempts")?;
        let max_attempts: i32 = row.try_get("max_attempts")?;
        let last_error: Option<String> = row.try_get("last_error")?;
        let worker_id: Option<String> = row.try_get("worker_id")?;
        let result_str: Option<String> = row.try_get("result")?;
        let tags_str: Option<String> = row.try_get("tags")?;

        let payload: serde_json::Value = serde_json::from_str(&payload_str)
            .map_err(|e| AppError::SerializationError(e))?;

        let result = match result_str {
            Some(r) => Some(serde_json::from_str(&r).map_err(|e| AppError::SerializationError(e))?),
            None => None,
        };

        let tags: Vec<String> = match tags_str {
            Some(t) => serde_json::from_str(&t).unwrap_or_else(|_| Vec::new()),
            None => Vec::new(),
        };

        let state = match state_str.as_str() {
            "pending" => TaskState::Pending,
            "scheduled" => TaskState::Scheduled,
            "running" => TaskState::Running,
            "completed" => TaskState::Completed,
            "failed" => TaskState::Failed,
            "cancelled" => TaskState::Cancelled,
            _ => TaskState::Pending,
        };

        let priority = match priority_str.as_str() {
            "low" => TaskPriority::Low,
            "medium" => TaskPriority::Medium,
            "high" => TaskPriority::High,
            "critical" => TaskPriority::Critical,
            _ => TaskPriority::Medium,
        };

        Ok(Task {
            id,
            name,
            payload,
            state,
            priority,
            created_at: DateTime::from_timestamp(created_at, 0)
                .unwrap_or_else(|| Utc::now()),
            updated_at: DateTime::from_timestamp(updated_at, 0)
                .unwrap_or_else(|| Utc::now()),
            scheduled_at: scheduled_at.map(|ts| 
                DateTime::from_timestamp(ts, 0).unwrap_or_else(|| Utc::now())
            ),
            started_at: started_at.map(|ts| 
                DateTime::from_timestamp(ts, 0).unwrap_or_else(|| Utc::now())
            ),
            completed_at: completed_at.map(|ts| 
                DateTime::from_timestamp(ts, 0).unwrap_or_else(|| Utc::now())
            ),
            attempts: attempts as u32,
            max_attempts: max_attempts as u32,
            last_error,
            worker_id,
            result,
            tags,
        })
    }
    
    async fn update_task(&self, task: &Task) -> AppResult<()> {
        let tags = serde_json::to_string(&task.tags).unwrap_or_else(|_| "[]".to_string());
        
        sqlx::query(
            r#"
            UPDATE tasks SET
                name = ?,
                payload = ?,
                state = ?,
                priority = ?,
                updated_at = ?,
                scheduled_at = ?,
                started_at = ?,
                completed_at = ?,
                attempts = ?,
                max_attempts = ?,
                last_error = ?,
                worker_id = ?,
                result = ?,
                tags = ?
            WHERE id = ?
            "#
        )
        .bind(&task.name)
        .bind(&task.payload.to_string())
        .bind(&task.state.to_string())
        .bind(&task.priority.to_string())
        .bind(task.updated_at.timestamp())
        .bind(task.scheduled_at.map(|dt| dt.timestamp()))
        .bind(task.started_at.map(|dt| dt.timestamp()))
        .bind(task.completed_at.map(|dt| dt.timestamp()))
        .bind(task.attempts as i32)
        .bind(task.max_attempts as i32)
        .bind(&task.last_error)
        .bind(&task.worker_id)
        .bind(task.result.as_ref().map(|r| r.to_string()))
        .bind(&tags)
        .bind(&task.id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e))?;

        Ok(())
    }

    async fn delete_task(&self, id: &str) -> AppResult<()> {
        sqlx::query("DELETE FROM tasks WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(e))?;

        Ok(())
    }

    async fn get_tasks(
        &self,
        state: Option<&str>,
        priority: Option<&str>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> AppResult<Vec<Task>> {
        let mut query = "SELECT * FROM tasks".to_string();
        let mut conditions = Vec::new();

        if let Some(state) = state {
            conditions.push(format!("state = '{}'", state));
        }

        if let Some(priority) = priority {
            conditions.push(format!("priority = '{}'", priority));
        }

        if !conditions.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(&conditions.join(" AND "));
        }

        query.push_str(" ORDER BY created_at DESC");

        if let Some(limit) = limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }

        if let Some(offset) = offset {
            query.push_str(&format!(" OFFSET {}", offset));
        }

        let rows = sqlx::query(&query)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(e))?;

        let mut tasks = Vec::new();
        for row in rows {
            let id: String = row.try_get("id")?;
            let name: String = row.try_get("name")?;
            let payload_str: String = row.try_get("payload")?;
            let state_str: String = row.try_get("state")?;
            let priority_str: String = row.try_get("priority")?;
            let created_at: i64 = row.try_get("created_at")?;
            let updated_at: i64 = row.try_get("updated_at")?;
            let scheduled_at: Option<i64> = row.try_get("scheduled_at")?;
            let started_at: Option<i64> = row.try_get("started_at")?;
            let completed_at: Option<i64> = row.try_get("completed_at")?;
            let attempts: i32 = row.try_get("attempts")?;
            let max_attempts: i32 = row.try_get("max_attempts")?;
            let last_error: Option<String> = row.try_get("last_error")?;
            let worker_id: Option<String> = row.try_get("worker_id")?;
            let result_str: Option<String> = row.try_get("result")?;
            let tags_str: Option<String> = row.try_get("tags")?;

            let payload: serde_json::Value = serde_json::from_str(&payload_str)
                .unwrap_or_else(|_| serde_json::Value::Null);

            let result = match result_str {
                Some(r) => Some(serde_json::from_str(&r).unwrap_or_else(|_| serde_json::Value::Null)),
                None => None,
            };

            let tags: Vec<String> = match tags_str {
                Some(t) => serde_json::from_str(&t).unwrap_or_else(|_| Vec::new()),
                None => Vec::new(),
            };

            let state = match state_str.as_str() {
                "pending" => TaskState::Pending,
                "scheduled" => TaskState::Scheduled,
                "running" => TaskState::Running,
                "completed" => TaskState::Completed,
                "failed" => TaskState::Failed,
                "cancelled" => TaskState::Cancelled,
                _ => TaskState::Pending,
            };

            let priority = match priority_str.as_str() {
                "low" => TaskPriority::Low,
                "medium" => TaskPriority::Medium,
                "high" => TaskPriority::High,
                "critical" => TaskPriority::Critical,
                _ => TaskPriority::Medium,
            };

            tasks.push(Task {
                id,
                name,
                payload,
                state,
                priority,
                created_at: DateTime::from_timestamp(created_at, 0)
                    .unwrap_or_else(|| Utc::now()),
                updated_at: DateTime::from_timestamp(updated_at, 0)
                    .unwrap_or_else(|| Utc::now()),
                scheduled_at: scheduled_at.map(|ts| 
                    DateTime::from_timestamp(ts, 0).unwrap_or_else(|| Utc::now())
                ),
                started_at: started_at.map(|ts| 
                    DateTime::from_timestamp(ts, 0).unwrap_or_else(|| Utc::now())
                ),
                completed_at: completed_at.map(|ts| 
                    DateTime::from_timestamp(ts, 0).unwrap_or_else(|| Utc::now())
                ),
                attempts: attempts as u32,
                max_attempts: max_attempts as u32,
                last_error,
                worker_id,
                result,
                tags,
            });
        }

        Ok(tasks)
    }

    async fn get_scheduled_tasks(&self, before: DateTime<Utc>) -> AppResult<Vec<Task>> {
        let before_timestamp = before.timestamp();
        
        let rows = sqlx::query(
            r#"
            SELECT
                id, name, payload, state, priority,
                created_at, updated_at, scheduled_at,
                started_at, completed_at, attempts,
                max_attempts, last_error, worker_id,
                result, tags
            FROM tasks
            WHERE state = 'scheduled' AND scheduled_at <= ?
            ORDER BY priority DESC, scheduled_at ASC
            "#
        )
        .bind(before_timestamp)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e))?;

        let mut tasks = Vec::new();
        for row in rows {
            let id: String = row.try_get("id")?;
            let name: String = row.try_get("name")?;
            let payload_str: String = row.try_get("payload")?;
            let state_str: String = row.try_get("state")?;
            let priority_str: String = row.try_get("priority")?;
            let created_at: i64 = row.try_get("created_at")?;
            let updated_at: i64 = row.try_get("updated_at")?;
            let scheduled_at: Option<i64> = row.try_get("scheduled_at")?;
            let started_at: Option<i64> = row.try_get("started_at")?;
            let completed_at: Option<i64> = row.try_get("completed_at")?;
            let attempts: i32 = row.try_get("attempts")?;
            let max_attempts: i32 = row.try_get("max_attempts")?;
            let last_error: Option<String> = row.try_get("last_error")?;
            let worker_id: Option<String> = row.try_get("worker_id")?;
            let result_str: Option<String> = row.try_get("result")?;
            let tags_str: Option<String> = row.try_get("tags")?;

            let payload: serde_json::Value = serde_json::from_str(&payload_str)
                .unwrap_or_else(|_| serde_json::Value::Null);

            let result = match result_str {
                Some(r) => Some(serde_json::from_str(&r).unwrap_or_else(|_| serde_json::Value::Null)),
                None => None,
            };

            let tags: Vec<String> = match tags_str {
                Some(t) => serde_json::from_str(&t).unwrap_or_else(|_| Vec::new()),
                None => Vec::new(),
            };

            let state = TaskState::Scheduled;
            let priority = match priority_str.as_str() {
                "low" => TaskPriority::Low,
                "medium" => TaskPriority::Medium,
                "high" => TaskPriority::High,
                "critical" => TaskPriority::Critical,
                _ => TaskPriority::Medium,
            };

            tasks.push(Task {
                id,
                name,
                payload,
                state,
                priority,
                created_at: DateTime::from_timestamp(created_at, 0)
                    .unwrap_or_else(|| Utc::now()),
                updated_at: DateTime::from_timestamp(updated_at, 0)
                    .unwrap_or_else(|| Utc::now()),
                scheduled_at: scheduled_at.map(|ts| 
                    DateTime::from_timestamp(ts, 0).unwrap_or_else(|| Utc::now())
                ),
                started_at: started_at.map(|ts| 
                    DateTime::from_timestamp(ts, 0).unwrap_or_else(|| Utc::now())
                ),
                completed_at: completed_at.map(|ts| 
                    DateTime::from_timestamp(ts, 0).unwrap_or_else(|| Utc::now())
                ),
                attempts: attempts as u32,
                max_attempts: max_attempts as u32,
                last_error,
                worker_id,
                result,
                tags,
            });
        }

        Ok(tasks)
    }

    async fn get_failed_tasks_for_retry(&self) -> AppResult<Vec<Task>> {
        let rows = sqlx::query(
            r#"
            SELECT
                id, name, payload, state, priority,
                created_at, updated_at, scheduled_at,
                started_at, completed_at, attempts,
                max_attempts, last_error, worker_id,
                result, tags
            FROM tasks
            WHERE state = 'failed' AND attempts < max_attempts
            ORDER BY priority DESC, updated_at ASC
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e))?;

        let mut tasks = Vec::new();
        for row in rows {
            let id: String = row.try_get("id")?;
            let name: String = row.try_get("name")?;
            let payload_str: String = row.try_get("payload")?;
            let priority_str: String = row.try_get("priority")?;
            let created_at: i64 = row.try_get("created_at")?;
            let updated_at: i64 = row.try_get("updated_at")?;
            let scheduled_at: Option<i64> = row.try_get("scheduled_at")?;
            let started_at: Option<i64> = row.try_get("started_at")?;
            let completed_at: Option<i64> = row.try_get("completed_at")?;
            let attempts: i32 = row.try_get("attempts")?;
            let max_attempts: i32 = row.try_get("max_attempts")?;
            let last_error: Option<String> = row.try_get("last_error")?;
            let worker_id: Option<String> = row.try_get("worker_id")?;
            let result_str: Option<String> = row.try_get("result")?;
            let tags_str: Option<String> = row.try_get("tags")?;

            let payload: serde_json::Value = serde_json::from_str(&payload_str)
                .unwrap_or_else(|_| serde_json::Value::Null);

            let result = match result_str {
                Some(r) => Some(serde_json::from_str(&r).unwrap_or_else(|_| serde_json::Value::Null)),
                None => None,
            };

            let tags: Vec<String> = match tags_str {
                Some(t) => serde_json::from_str(&t).unwrap_or_else(|_| Vec::new()),
                None => Vec::new(),
            };

            tasks.push(Task {
                id,
                name,
                payload,
                state: TaskState::Failed,
                priority: match priority_str.as_str() {
                    "low" => TaskPriority::Low,
                    "medium" => TaskPriority::Medium,
                    "high" => TaskPriority::High,
                    "critical" => TaskPriority::Critical,
                    _ => TaskPriority::Medium,
                },
                created_at: DateTime::from_timestamp(created_at, 0)
                    .unwrap_or_else(|| Utc::now()),
                updated_at: DateTime::from_timestamp(updated_at, 0)
                    .unwrap_or_else(|| Utc::now()),
                scheduled_at: scheduled_at.map(|ts| 
                    DateTime::from_timestamp(ts, 0).unwrap_or_else(|| Utc::now())
                ),
                started_at: started_at.map(|ts| 
                    DateTime::from_timestamp(ts, 0).unwrap_or_else(|| Utc::now())
                ),
                completed_at: completed_at.map(|ts| 
                    DateTime::from_timestamp(ts, 0).unwrap_or_else(|| Utc::now())
                ),
                attempts: attempts as u32,
                max_attempts: max_attempts as u32,
                last_error,
                worker_id,
                result,
                tags,
            });
        }

        Ok(tasks)
    }

    async fn count_tasks_by_state(&self) -> AppResult<Vec<(String, i64)>> {
        let rows = sqlx::query(
            r#"
            SELECT state, COUNT(*) as count
            FROM tasks
            GROUP BY state
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e))?;

        let mut counts = Vec::new();
        for row in rows {
            let state: String = row.try_get("state")?;
            let count: i64 = row.try_get("count")?;
            counts.push((state, count));
        }

        Ok(counts)
    }

    async fn count_tasks_by_priority(&self) -> AppResult<Vec<(String, i64)>> {
        let rows = sqlx::query(
            r#"
            SELECT priority, COUNT(*) as count
            FROM tasks
            GROUP BY priority
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e))?;

        let mut counts = Vec::new();
        for row in rows {
            let priority: String = row.try_get("priority")?;
            let count: i64 = row.try_get("count")?;
            counts.push((priority, count));
        }

        Ok(counts)
    }

    async fn setup(&self) -> AppResult<()> {
        info!("Setting up SQLite database...");
        
        // Create main table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                payload TEXT NOT NULL,
                state TEXT NOT NULL,
                priority TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                scheduled_at INTEGER,
                started_at INTEGER,
                completed_at INTEGER,
                attempts INTEGER NOT NULL DEFAULT 0,
                max_attempts INTEGER NOT NULL DEFAULT 3,
                last_error TEXT,
                worker_id TEXT,
                result TEXT,
                tags TEXT
            )
            "#
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e))?;

        // Create indexes - run each separately to avoid issues if one fails
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_tasks_state ON tasks (state)"
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e))?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_tasks_priority ON tasks (priority)"
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e))?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_tasks_scheduled_at ON tasks (scheduled_at)"
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e))?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_tasks_created_at ON tasks (created_at)"
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e))?;

        info!("SQLite database setup completed.");
        Ok(())
    }
}