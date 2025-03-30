use actix_web::{HttpResponse, ResponseError};
use serde::Serialize;
use std::fmt;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Task not found with ID: {0}")]
    TaskNotFound(String),

    #[error("Task already exists with ID: {0}")]
    TaskAlreadyExists(String),

    #[error("Queue is full")]
    QueueFull,

    #[error("Worker is busy")]
    WorkerBusy,

    #[error("Invalid task state transition from {from} to {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("Task execution timed out after {0} seconds")]
    TaskTimeout(u64),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Internal server error: {0}")]
    InternalServerError(String),
}

#[derive(Serialize)]
struct ErrorResponse {
    status: String,
    message: String,
}

impl ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        let status = self.status_code();
        
        let error_response = ErrorResponse {
            status: status.to_string(),
            message: self.to_string(),
        };
        
        HttpResponse::build(status).json(error_response)
    }

    fn status_code(&self) -> actix_web::http::StatusCode {
        use actix_web::http::StatusCode;
        
        match self {
            AppError::TaskNotFound(_) => StatusCode::NOT_FOUND,
            AppError::TaskAlreadyExists(_) => StatusCode::CONFLICT,
            AppError::QueueFull => StatusCode::SERVICE_UNAVAILABLE,
            AppError::WorkerBusy => StatusCode::SERVICE_UNAVAILABLE,
            AppError::InvalidStateTransition { .. } => StatusCode::BAD_REQUEST,
            AppError::TaskTimeout(_) => StatusCode::REQUEST_TIMEOUT,
            AppError::ConfigError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;