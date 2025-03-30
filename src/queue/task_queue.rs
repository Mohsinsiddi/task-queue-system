use crate::config::QueueConfig;
use crate::error::{AppError, AppResult};
use crate::models::Task;
use crate::storage::Database;
use chrono::Utc;
use crossbeam_channel::{bounded, Receiver, Sender};
use log::{debug, error, info, warn};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use uuid::Uuid;

use super::PriorityQueue;

pub struct TaskQueue {
    /// Database connection
    db: Arc<dyn Database>,
    /// Queue configuration
    config: QueueConfig,
    /// In-memory priority queue for pending tasks
    pending_queue: Arc<Mutex<PriorityQueue>>,
    /// Currently processing tasks
    processing: Arc<Mutex<HashMap<String, Task>>>,
    /// Channel for submitting tasks
    task_sender: Sender<Task>,
    /// Channel for receiving tasks
    task_receiver: Receiver<Task>,
    /// Worker ID for this queue instance
    worker_id: String,
}

impl Clone for TaskQueue {
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
            config: self.config.clone(),
            pending_queue: self.pending_queue.clone(),
            processing: self.processing.clone(),
            task_sender: self.task_sender.clone(),
            task_receiver: self.task_receiver.clone(),
            worker_id: self.worker_id.clone(),
        }
    }
}

impl TaskQueue {
    /// Create a new task queue
    pub fn new(db: Arc<dyn Database>, config: QueueConfig) -> Self {
        let (task_sender, task_receiver) = bounded(config.max_concurrent_tasks * 2);
        let worker_id = Uuid::new_v4().to_string();
        
        Self {
            db,
            config,
            pending_queue: Arc::new(Mutex::new(PriorityQueue::new())),
            processing: Arc::new(Mutex::new(HashMap::new())),
            task_sender,
            task_receiver,
            worker_id,
        }
    }

    /// Start the queue processing loop
    pub async fn start(&self) -> AppResult<()> {
        info!("Starting task queue with worker ID: {}", self.worker_id);
        
        // Load any existing pending and scheduled tasks from the database
        self.load_existing_tasks().await?;
        
        // Start the scheduler loop in a separate thread
        self.start_scheduler();
        
        // Start the retry loop in a separate thread
        self.start_retry_handler();
        
        // Start the task processing loop
        self.process_tasks().await?;
        
        Ok(())
    }

    /// Submit a new task to the queue
    pub async fn submit_task(&self, task: Task) -> AppResult<()> {
        debug!("Submitting task: {} ({})", task.name, task.id);
        
        // Save the task to the database first
        self.db.create_task(&task).await?;
        
        // If the task is scheduled for the future, don't add it to the in-memory queue
        if let Some(scheduled_at) = task.scheduled_at {
            if scheduled_at > Utc::now() {
                return Ok(());
            }
        }
        
        // Add to the in-memory queue
        if self.task_sender.send(task).is_err() {
            return Err(AppError::QueueFull);
        }
        
        Ok(())
    }

    /// Cancel a task by ID
    pub async fn cancel_task(&self, task_id: &str) -> AppResult<()> {
        let mut task = self.db.get_task(task_id).await?;
        
        // Can only cancel tasks that are not yet completed
        if matches!(task.state, crate::models::TaskState::Completed) {
            return Err(AppError::InvalidStateTransition { 
                from: task.state.to_string(), 
                to: "cancelled".to_string() 
            });
        }
        
        task.mark_cancelled();
        self.db.update_task(&task).await?;
        
        // If the task is currently processing, we need to remove it
        {
            let mut processing = self.processing.lock();
            processing.remove(task_id);
        }
        
        Ok(())
    }

    /// Get a task by ID
    pub async fn get_task(&self, task_id: &str) -> AppResult<Task> {
        self.db.get_task(task_id).await
    }

    /// Load existing pending and scheduled tasks from the database
    async fn load_existing_tasks(&self) -> AppResult<()> {
        info!("Loading existing tasks from database...");
        
        // Load pending tasks
        let tasks = self.db.get_tasks(Some("pending"), None, None, None).await?;
        let mut pending_queue = self.pending_queue.lock();
        
        for task in tasks {
            debug!("Loading pending task: {} ({})", task.name, task.id);
            pending_queue.push(task);
        }
        
        // Load scheduled tasks that are due now
        let now = Utc::now();
        let scheduled_tasks = self.db.get_scheduled_tasks(now).await?;
        
        for task in scheduled_tasks {
            debug!("Loading scheduled task: {} ({})", task.name, task.id);
            pending_queue.push(task);
        }
        
        info!("Loaded {} tasks into memory", pending_queue.len());
        
        Ok(())
    }

    /// Start the scheduler loop to check for scheduled tasks
    fn start_scheduler(&self) {
        let db = self.db.clone();
        let task_sender = self.task_sender.clone();
        
        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_secs(15));
                
                // Check for scheduled tasks that are due
                let now = Utc::now();
                
                match tokio::runtime::Runtime::new() {
                    Ok(rt) => {
                        match rt.block_on(db.get_scheduled_tasks(now)) {
                            Ok(tasks) => {
                                for task in tasks {
                                    debug!("Scheduling due task: {} ({})", task.name, task.id);
                                    
                                    if task_sender.send(task).is_err() {
                                        error!("Failed to schedule task: Queue is full");
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Error fetching scheduled tasks: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to create tokio runtime for scheduler: {}", e);
                    }
                }
            }
        });
    }

    /// Start the retry handler loop to check for failed tasks that need to be retried
    fn start_retry_handler(&self) {
        let db = self.db.clone();
        let task_sender = self.task_sender.clone();
        let retry_interval = self.config.retry_initial_interval_ms;
        
        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_millis(retry_interval));
                
                match tokio::runtime::Runtime::new() {
                    Ok(rt) => {
                        match rt.block_on(db.get_failed_tasks_for_retry()) {
                            Ok(tasks) => {
                                for mut task in tasks {
                                    debug!("Retrying failed task: {} ({}) - attempt {}/{}",
                                           task.name, task.id, task.attempts + 1, task.max_attempts);
                                    
                                    // Reset state to pending for retry
                                    task.state = crate::models::TaskState::Pending;
                                    
                                    // Update in database
                                    if let Err(e) = rt.block_on(db.update_task(&task)) {
                                        error!("Failed to update task for retry: {}", e);
                                        continue;
                                    }
                                    
                                    // Add to queue
                                    if task_sender.send(task).is_err() {
                                        error!("Failed to queue retry task: Queue is full");
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Error fetching failed tasks for retry: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to create tokio runtime for retry handler: {}", e);
                    }
                }
            }
        });
    }

    /// Main task processing loop
    async fn process_tasks(&self) -> AppResult<()> {
        info!("Starting task processing loop");
        
        loop {
            // Process all tasks in the channel
            while let Ok(task) = self.task_receiver.try_recv() {
                self.process_task(task).await?;
            }
            
            // Check if we can process more tasks
            {
                let processing = self.processing.lock();
                if processing.len() >= self.config.max_concurrent_tasks {
                    // We're at capacity, wait a bit before checking again
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                }
            }
            
            // Try to get the next task from the priority queue
            let task = {
                let mut pending_queue = self.pending_queue.lock();
                pending_queue.pop()
            };
            
            if let Some(task) = task {
                self.process_task(task).await?;
            } else {
                // No tasks in the queue, wait for tasks to be submitted
                match self.task_receiver.recv() {
                    Ok(task) => {
                        self.process_task(task).await?;
                    }
                    Err(e) => {
                        error!("Channel error: {}", e);
                        // Sleep a bit before retrying
                        tokio::time::sleep(Duration::from_millis(1000)).await;
                    }
                }
            }
        }
    }

    /// Process a single task
    async fn process_task(&self, mut task: Task) -> AppResult<()> {
        debug!("Processing task: {} ({})", task.name, task.id);
        
        // Mark the task as running
        task.mark_running(self.worker_id.clone());
        
        // Update the task in the database
        self.db.update_task(&task).await?;
        
        // Add to processing list
        {
            let mut processing = self.processing.lock();
            processing.insert(task.id.clone(), task.clone());
        }
        
        // Simulate task execution (in a real system, this would be replaced with actual task handling)
        tokio::spawn({
            let task_id = task.id.clone();
            let db = self.db.clone();
            let processing = self.processing.clone();
            let timeout = self.config.task_timeout_seconds;
            
            async move {
                debug!("Executing task: {} ({})", task.name, task.id);
                
                // In a real system, this is where you'd execute the actual task logic
                // For now, we'll just simulate task execution with a delay
                let success = tokio::time::timeout(
                    Duration::from_secs(timeout),
                    simulate_task_execution(&task)
                ).await;
                
                // Update the task based on the execution result
                let mut task = match db.get_task(&task.id).await {
                    Ok(t) => t,
                    Err(e) => {
                        error!("Failed to get task for completion: {}", e);
                        return;
                    }
                };
                
                match success {
                    Ok(result) => {
                        debug!("Task completed successfully: {} ({})", task.name, task.id);
                        task.mark_completed(Some(result));
                    }
                    Err(_) => {
                        warn!("Task timed out: {} ({})", task.name, task.id);
                        task.mark_failed(format!("Task timed out after {} seconds", timeout));
                    }
                }
                
                // Update the task in the database
                if let Err(e) = db.update_task(&task).await {
                    error!("Failed to update task after execution: {}", e);
                }
                
                // Remove from processing list
                let mut processing_guard = processing.lock();
                processing_guard.remove(&task_id);
            }
        });
        
        Ok(())
    }
}

// Simulate task execution (replace with actual task handling in a real system)
async fn simulate_task_execution(task: &Task) -> serde_json::Value {
    // Simulate different processing times based on priority
    let delay = match task.priority {
        crate::models::TaskPriority::Critical => 1,
        crate::models::TaskPriority::High => 2,
        crate::models::TaskPriority::Medium => 3,
        crate::models::TaskPriority::Low => 5,
    };
    
    tokio::time::sleep(Duration::from_secs(delay)).await;
    
    // Return a simulated result
    serde_json::json!({
        "task_id": task.id,
        "execution_time_seconds": delay,
        "result": format!("Task {} completed successfully", task.name),
        "timestamp": Utc::now().to_rfc3339()
    })
}