mod api;
mod config;
mod error;
mod models;
mod queue;
mod storage;

use actix_web::{middleware, web, App, HttpServer};
use log::{error, info, warn};
use std::sync::Arc;
use std::time::Duration;

// Wait for database to be ready with retries
async fn wait_for_database(config: &config::AppConfig) -> error::AppResult<Arc<dyn storage::Database>> {
    const MAX_RETRIES: u32 = 10;
    const RETRY_DELAY: Duration = Duration::from_secs(2);
    
    let mut last_error = None;
    for attempt in 1..=MAX_RETRIES {
        info!("Database connection attempt {}/{}", attempt, MAX_RETRIES);
        
        match storage::create_database(config.get_database_url()).await {
            Ok(db) => {
                info!("Successfully connected to database");
                return Ok(db);
            }
            Err(e) => {
                warn!("Failed to connect to database: {}", e);
                last_error = Some(e);
                tokio::time::sleep(RETRY_DELAY).await;
            }
        }
    }
    
    Err(last_error.unwrap_or_else(|| error::AppError::DatabaseError(
        sqlx::Error::Configuration("Max database connection attempts reached".into())
    )))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize environment variables from .env file
    dotenv::dotenv().ok();
    
    // Initialize logger
    env_logger::init();

    // Load application configuration
    let app_config = match config::AppConfig::from_env() {
        Ok(config) => config,
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            std::process::exit(1);
        }
    };

    // Wait for database to be ready with retries
    info!("Connecting to database at {}", app_config.get_database_url());
    let db = match wait_for_database(&app_config).await {
        Ok(db) => db,
        Err(e) => {
            error!("Failed to connect to database after multiple attempts: {}", e);
            error!("Make sure the database is running and accessible.");
            std::process::exit(1);
        }
    };

    // Set up database tables, indexes, etc.
    if let Err(e) = db.setup().await {
        error!("Failed to set up database: {}", e);
        std::process::exit(1);
    }

    // Create task queue
    let task_queue = queue::TaskQueue::new(db.clone(), app_config.queue.clone());
    
    // Create shared task queue instance
    let task_queue = web::Data::new(task_queue);
    
    // Start the task queue in a separate task
    let queue_handle = task_queue.clone();
    actix_web::rt::spawn(async move {
        if let Err(e) = queue_handle.start().await {
            error!("Task queue error: {}", e);
            std::process::exit(1);
        }
    });

    // Start HTTP server
    info!("Starting server at {}:{}", app_config.server.host, app_config.server.port);
    
    HttpServer::new(move || {
        App::new()
            // Enable logger middleware
            .wrap(middleware::Logger::default())
            // Register shared data
            .app_data(task_queue.clone())
            .app_data(web::Data::new(db.clone()))
            // Configure routes
            .configure(api::configure_routes)
    })
    .bind(format!("{}:{}", app_config.server.host, app_config.server.port))?
    .run()
    .await
}