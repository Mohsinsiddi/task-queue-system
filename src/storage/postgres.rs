use crate::error::{AppError, AppResult};
use crate::models::{Task, TaskPriority, TaskState};
use crate::storage::database::Database;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use log::{info, warn};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use std::time::Duration;

pub struct PostgresDatabase {
    pool: PgPool,
}

impl PostgresDatabase {
    pub async fn new(database_url: &str) -> AppResult<Self> {
        // Add connection timeout and retry logic
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(5))
            .connect(database_url)
            .await
            .map_err(|e| {
                warn!("PostgreSQL connection error: {}", e);
                AppError::DatabaseError(e)
            })?;

        Ok(Self { pool })
    }
}

#[async_trait]
impl Database for PostgresDatabase {
    async fn create_task(&self, task: &Task) -> AppResult<()> {
        // Use sqlx::query instead of the query! macro to avoid static checking issues
        sqlx::query(
            r#"
            INSERT INTO tasks (
                id, name, payload, state, priority,
                created_at, updated_at, scheduled_at,
                started_at, completed_at, attempts,
                max_attempts, last_error, worker_id,
                result, tags
            ) VALUES (
                $1, $2, $3, $4, $5,
                $6, $7, $8, $9, $10,
                $11, $12, $13, $14, $15,
                $16
            )
            "#
        )
        .bind(&task.id)
        .bind(&task.name)
        .bind(&task.payload)
        .bind(&task.state.to_string())
        .bind(&task.priority.to_string())
        .bind(&task.created_at)
        .bind(&task.updated_at)
        .bind(&task.scheduled_at)
        .bind(&task.started_at)
        .bind(&task.completed_at)
        .bind(task.attempts as i32)
        .bind(task.max_attempts as i32)
        .bind(&task.last_error)
        .bind(&task.worker_id)
        .bind(&task.result)
        .bind(&task.tags)
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
            WHERE id = $1
            "#
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => AppError::TaskNotFound(id.to_string()),
            e => AppError::DatabaseError(e),
        })?;

        // Extract values from row
        let id: String = row.try_get("id")?;
        let name: String = row.try_get("name")?;
        let payload: serde_json::Value = row.try_get("payload")?;
        let state_str: String = row.try_get("state")?;
        let priority_str: String = row.try_get("priority")?;
        let created_at: DateTime<Utc> = row.try_get("created_at")?;
        let updated_at: DateTime<Utc> = row.try_get("updated_at")?;
        let scheduled_at: Option<DateTime<Utc>> = row.try_get("scheduled_at")?;
        let started_at: Option<DateTime<Utc>> = row.try_get("started_at")?;
        let completed_at: Option<DateTime<Utc>> = row.try_get("completed_at")?;
        let attempts: i32 = row.try_get("attempts")?;
        let max_attempts: i32 = row.try_get("max_attempts")?;
        let last_error: Option<String> = row.try_get("last_error")?;
        let worker_id: Option<String> = row.try_get("worker_id")?;
        let result: Option<serde_json::Value> = row.try_get("result")?;
        let tags: Vec<String> = row.try_get("tags").unwrap_or_default();

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
            created_at,
            updated_at,
            scheduled_at,
            started_at,
            completed_at,
            attempts: attempts as u32,
            max_attempts: max_attempts as u32,
            last_error,
            worker_id,
            result,
            tags,
        })
    }

    async fn update_task(&self, task: &Task) -> AppResult<()> {
        sqlx::query(
            r#"
            UPDATE tasks SET
                name = $1,
                payload = $2,
                state = $3,
                priority = $4,
                updated_at = $5,
                scheduled_at = $6,
                started_at = $7,
                completed_at = $8,
                attempts = $9,
                max_attempts = $10,
                last_error = $11,
                worker_id = $12,
                result = $13,
                tags = $14
            WHERE id = $15
            "#
        )
        .bind(&task.name)
        .bind(&task.payload)
        .bind(&task.state.to_string())
        .bind(&task.priority.to_string())
        .bind(&task.updated_at)
        .bind(&task.scheduled_at)
        .bind(&task.started_at)
        .bind(&task.completed_at)
        .bind(task.attempts as i32)
        .bind(task.max_attempts as i32)
        .bind(&task.last_error)
        .bind(&task.worker_id)
        .bind(&task.result)
        .bind(&task.tags)
        .bind(&task.id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e))?;

        Ok(())
    }

    async fn delete_task(&self, id: &str) -> AppResult<()> {
        sqlx::query("DELETE FROM tasks WHERE id = $1")
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
            let payload: serde_json::Value = row.try_get("payload")?;
            let state_str: String = row.try_get("state")?;
            let priority_str: String = row.try_get("priority")?;
            let created_at: DateTime<Utc> = row.try_get("created_at")?;
            let updated_at: DateTime<Utc> = row.try_get("updated_at")?;
            let scheduled_at: Option<DateTime<Utc>> = row.try_get("scheduled_at")?;
            let started_at: Option<DateTime<Utc>> = row.try_get("started_at")?;
            let completed_at: Option<DateTime<Utc>> = row.try_get("completed_at")?;
            let attempts: i32 = row.try_get("attempts")?;
            let max_attempts: i32 = row.try_get("max_attempts")?;
            let last_error: Option<String> = row.try_get("last_error")?;
            let worker_id: Option<String> = row.try_get("worker_id")?;
            let result: Option<serde_json::Value> = row.try_get("result")?;
            let tags: Option<Vec<String>> = row.try_get("tags")?;

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
                created_at,
                updated_at,
                scheduled_at,
                started_at,
                completed_at,
                attempts: attempts as u32,
                max_attempts: max_attempts as u32,
                last_error,
                worker_id,
                result,
                tags: tags.unwrap_or_default(),
            });
        }

        Ok(tasks)
    }

    async fn get_scheduled_tasks(&self, before: DateTime<Utc>) -> AppResult<Vec<Task>> {
        let rows = sqlx::query(
            r#"
            SELECT
                id, name, payload, state, priority,
                created_at, updated_at, scheduled_at,
                started_at, completed_at, attempts,
                max_attempts, last_error, worker_id,
                result, tags
            FROM tasks
            WHERE state = 'scheduled' AND scheduled_at <= $1
            ORDER BY priority DESC, scheduled_at ASC
            "#
        )
        .bind(&before)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e))?;

        let mut tasks = Vec::new();
        for row in rows {
            let id: String = row.try_get("id")?;
            let name: String = row.try_get("name")?;
            let payload: serde_json::Value = row.try_get("payload")?;
            let state_str: String = row.try_get("state")?;
            let priority_str: String = row.try_get("priority")?;
            let created_at: DateTime<Utc> = row.try_get("created_at")?;
            let updated_at: DateTime<Utc> = row.try_get("updated_at")?;
            let scheduled_at: Option<DateTime<Utc>> = row.try_get("scheduled_at")?;
            let started_at: Option<DateTime<Utc>> = row.try_get("started_at")?;
            let completed_at: Option<DateTime<Utc>> = row.try_get("completed_at")?;
            let attempts: i32 = row.try_get("attempts")?;
            let max_attempts: i32 = row.try_get("max_attempts")?;
            let last_error: Option<String> = row.try_get("last_error")?;
            let worker_id: Option<String> = row.try_get("worker_id")?;
            let result: Option<serde_json::Value> = row.try_get("result")?;
            let tags: Vec<String> = row.try_get("tags").unwrap_or_default();

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
                created_at,
                updated_at,
                scheduled_at,
                started_at,
                completed_at,
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
            let payload: serde_json::Value = row.try_get("payload")?;
            let priority_str: String = row.try_get("priority")?;
            let created_at: DateTime<Utc> = row.try_get("created_at")?;
            let updated_at: DateTime<Utc> = row.try_get("updated_at")?;
            let scheduled_at: Option<DateTime<Utc>> = row.try_get("scheduled_at")?;
            let started_at: Option<DateTime<Utc>> = row.try_get("started_at")?;
            let completed_at: Option<DateTime<Utc>> = row.try_get("completed_at")?;
            let attempts: i32 = row.try_get("attempts")?;
            let max_attempts: i32 = row.try_get("max_attempts")?;
            let last_error: Option<String> = row.try_get("last_error")?;
            let worker_id: Option<String> = row.try_get("worker_id")?;
            let result: Option<serde_json::Value> = row.try_get("result")?;
            let tags: Vec<String> = row.try_get("tags").unwrap_or_default();

            let state = TaskState::Failed;
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
                created_at,
                updated_at,
                scheduled_at,
                started_at,
                completed_at,
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
        info!("Setting up PostgreSQL database...");
        
        // Create main table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                payload JSONB NOT NULL,
                state TEXT NOT NULL,
                priority TEXT NOT NULL,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL,
                scheduled_at TIMESTAMPTZ,
                started_at TIMESTAMPTZ,
                completed_at TIMESTAMPTZ,
                attempts INTEGER NOT NULL DEFAULT 0,
                max_attempts INTEGER NOT NULL DEFAULT 3,
                last_error TEXT,
                worker_id TEXT,
                result JSONB,
                tags TEXT[] NOT NULL DEFAULT '{}'
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

        info!("PostgreSQL database setup completed.");
        Ok(())
    }
}