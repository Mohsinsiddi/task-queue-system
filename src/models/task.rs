use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskPriority {
    Low,
    Medium,
    High,
    Critical,
}

impl Default for TaskPriority {
    fn default() -> Self {
        Self::Medium
    }
}

impl fmt::Display for TaskPriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskPriority::Low => write!(f, "low"),
            TaskPriority::Medium => write!(f, "medium"),
            TaskPriority::High => write!(f, "high"),
            TaskPriority::Critical => write!(f, "critical"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskState {
    Pending,
    Scheduled,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl Default for TaskState {
    fn default() -> Self {
        Self::Pending
    }
}

impl fmt::Display for TaskState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskState::Pending => write!(f, "pending"),
            TaskState::Scheduled => write!(f, "scheduled"),
            TaskState::Running => write!(f, "running"),
            TaskState::Completed => write!(f, "completed"),
            TaskState::Failed => write!(f, "failed"),
            TaskState::Cancelled => write!(f, "cancelled"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub name: String,
    pub payload: serde_json::Value,
    pub state: TaskState,
    pub priority: TaskPriority,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub attempts: u32,
    pub max_attempts: u32,
    pub last_error: Option<String>,
    pub worker_id: Option<String>,
    pub result: Option<serde_json::Value>,
    pub tags: Vec<String>,
}

impl Task {
    pub fn new(name: String, payload: serde_json::Value) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            payload,
            state: TaskState::default(),
            priority: TaskPriority::default(),
            created_at: now,
            updated_at: now,
            scheduled_at: None,
            started_at: None,
            completed_at: None,
            attempts: 0,
            max_attempts: 3, // Default value
            last_error: None,
            worker_id: None,
            result: None,
            tags: Vec::new(),
        }
    }

    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_scheduled_time(mut self, scheduled_at: DateTime<Utc>) -> Self {
        self.scheduled_at = Some(scheduled_at);
        self.state = TaskState::Scheduled;
        self
    }

    pub fn with_max_attempts(mut self, max_attempts: u32) -> Self {
        self.max_attempts = max_attempts;
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn is_ready_to_run(&self) -> bool {
        match self.state {
            TaskState::Pending => true,
            TaskState::Scheduled => {
                if let Some(scheduled_time) = self.scheduled_at {
                    scheduled_time <= Utc::now()
                } else {
                    true
                }
            }
            _ => false,
        }
    }

    pub fn can_retry(&self) -> bool {
        matches!(self.state, TaskState::Failed) && self.attempts < self.max_attempts
    }

    pub fn mark_running(&mut self, worker_id: String) {
        self.state = TaskState::Running;
        self.worker_id = Some(worker_id);
        self.started_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    pub fn mark_completed(&mut self, result: Option<serde_json::Value>) {
        self.state = TaskState::Completed;
        self.result = result;
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    pub fn mark_failed(&mut self, error: String) {
        self.state = TaskState::Failed;
        self.last_error = Some(error);
        self.attempts += 1;
        self.updated_at = Utc::now();
    }

    pub fn mark_cancelled(&mut self) {
        self.state = TaskState::Cancelled;
        self.updated_at = Utc::now();
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTaskRequest {
    pub name: String,
    pub payload: serde_json::Value,
    pub priority: Option<TaskPriority>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub max_attempts: Option<u32>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskResponse {
    pub id: String,
    pub name: String,
    pub state: String,
    pub priority: String,
    pub created_at: DateTime<Utc>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub attempts: u32,
    pub max_attempts: u32,
    pub tags: Vec<String>,
}

impl From<Task> for TaskResponse {
    fn from(task: Task) -> Self {
        Self {
            id: task.id,
            name: task.name,
            state: task.state.to_string(),
            priority: task.priority.to_string(),
            created_at: task.created_at,
            scheduled_at: task.scheduled_at,
            completed_at: task.completed_at,
            attempts: task.attempts,
            max_attempts: task.max_attempts,
            tags: task.tags,
        }
    }
}