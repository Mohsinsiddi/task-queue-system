use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppResult;
use crate::models::{CreateTaskRequest, Task, TaskPriority, TaskResponse, TaskState};
use crate::queue::TaskQueue;

// Task list response
#[derive(Serialize)]
struct TaskListResponse {
    tasks: Vec<TaskResponse>,
    total: usize,
}

// Task status counts
#[derive(Serialize)]
struct TaskStatusCounts {
    counts: Vec<TaskStatusCount>,
}

#[derive(Serialize)]
struct TaskStatusCount {
    status: String,
    count: i64,
}

// Task creation response
#[derive(Serialize)]
struct TaskCreationResponse {
    task_id: String,
    status: String,
}

// Filter query parameters
#[derive(Deserialize)]
struct TaskFilterParams {
    state: Option<String>,
    priority: Option<String>,
    limit: Option<u32>,
    offset: Option<u32>,
}

// Create a new task
async fn create_task(
    task_queue: web::Data<TaskQueue>,
    req: web::Json<CreateTaskRequest>,
) -> AppResult<impl Responder> {
    let request = req.into_inner();
    
    // Create a new task
    let mut task = Task::new(request.name, request.payload);
    
    // Set priority if provided
    if let Some(priority) = request.priority {
        task = task.with_priority(priority);
    }
    
    // Set scheduled time if provided
    if let Some(scheduled_at) = request.scheduled_at {
        task = task.with_scheduled_time(scheduled_at);
    }
    
    // Set max attempts if provided
    if let Some(max_attempts) = request.max_attempts {
        task = task.with_max_attempts(max_attempts);
    }
    
    // Set tags if provided
    if let Some(tags) = request.tags {
        task = task.with_tags(tags);
    }
    
    // Submit task to the queue
    task_queue.submit_task(task.clone()).await?;
    
    Ok(HttpResponse::Created().json(TaskCreationResponse {
        task_id: task.id,
        status: task.state.to_string(),
    }))
}

// Get a task by ID
async fn get_task(
    task_queue: web::Data<TaskQueue>,
    path: web::Path<String>,
) -> AppResult<impl Responder> {
    let task_id = path.into_inner();
    let task = task_queue.get_task(&task_id).await?;
    
    Ok(HttpResponse::Ok().json(TaskResponse::from(task)))
}

// Cancel a task
async fn cancel_task(
    task_queue: web::Data<TaskQueue>,
    path: web::Path<String>,
) -> AppResult<impl Responder> {
    let task_id = path.into_inner();
    task_queue.cancel_task(&task_id).await?;
    
    Ok(HttpResponse::Ok().json(TaskCreationResponse {
        task_id,
        status: TaskState::Cancelled.to_string(),
    }))
}

// List tasks with optional filtering
async fn list_tasks(
    task_queue: web::Data<TaskQueue>,
    db: web::Data<std::sync::Arc<dyn crate::storage::Database>>,
    query: web::Query<TaskFilterParams>,
) -> AppResult<impl Responder> {
    let state_filter = query.state.as_deref();
    let priority_filter = query.priority.as_deref();
    let limit = query.limit;
    let offset = query.offset;
    
    let tasks = db.get_tasks(state_filter, priority_filter, limit, offset).await?;
    let total = tasks.len();
    
    let task_responses: Vec<TaskResponse> = tasks.into_iter().map(TaskResponse::from).collect();
    
    Ok(HttpResponse::Ok().json(TaskListResponse {
        tasks: task_responses,
        total,
    }))
}

// Get task status counts
async fn get_task_counts(
    db: web::Data<std::sync::Arc<dyn crate::storage::Database>>,
) -> AppResult<impl Responder> {
    let counts = db.count_tasks_by_state().await?;
    
    let status_counts = counts
        .into_iter()
        .map(|(status, count)| TaskStatusCount { status, count })
        .collect();
    
    Ok(HttpResponse::Ok().json(TaskStatusCounts {
        counts: status_counts,
    }))
}

// Health check endpoint
async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "timestamp": Utc::now().to_rfc3339(),
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

// Configure all routes
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg
        .service(
            web::scope("/api/v1")
                // Task management endpoints
                .service(
                    web::scope("/tasks")
                        .route("", web::post().to(create_task))
                        .route("", web::get().to(list_tasks))
                        .route("/counts", web::get().to(get_task_counts))
                        .route("/{id}", web::get().to(get_task))
                        .route("/{id}/cancel", web::post().to(cancel_task))
                )
                // Health check
                .route("/health", web::get().to(health_check))
        );
}