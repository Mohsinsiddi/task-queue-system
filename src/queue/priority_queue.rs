use crate::models::{Task, TaskPriority};
use std::cmp::Ordering;
use std::collections::BinaryHeap;

// Wrapper to make Task comparable for priority queue
#[derive(Clone)]
struct PrioritizedTask {
    task: Task,
}

// Implement PartialEq manually to avoid issues with serde_json::Value
impl PartialEq for PrioritizedTask {
    fn eq(&self, other: &Self) -> bool {
        self.task.id == other.task.id
    }
}

// Implement Eq manually as well
impl Eq for PrioritizedTask {}

// Define ordering for priority queue
impl Ord for PrioritizedTask {
    fn cmp(&self, other: &Self) -> Ordering {
        // First compare by priority (higher priority comes first)
        let priority_ordering = match (&self.task.priority, &other.task.priority) {
            (TaskPriority::Critical, TaskPriority::Critical) => Ordering::Equal,
            (TaskPriority::Critical, _) => Ordering::Greater,
            (_, TaskPriority::Critical) => Ordering::Less,
            (TaskPriority::High, TaskPriority::High) => Ordering::Equal,
            (TaskPriority::High, _) => Ordering::Greater,
            (_, TaskPriority::High) => Ordering::Less,
            (TaskPriority::Medium, TaskPriority::Medium) => Ordering::Equal,
            (TaskPriority::Medium, _) => Ordering::Greater,
            (_, TaskPriority::Medium) => Ordering::Less,
            (TaskPriority::Low, TaskPriority::Low) => Ordering::Equal,
        };

        if priority_ordering != Ordering::Equal {
            return priority_ordering;
        }

        // Then by creation time (older tasks come first)
        self.task.created_at.cmp(&other.task.created_at)
    }
}

impl PartialOrd for PrioritizedTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A priority queue for tasks based on task priority and creation time
pub struct PriorityQueue {
    heap: BinaryHeap<PrioritizedTask>,
}

impl PriorityQueue {
    /// Create a new empty priority queue
    pub fn new() -> Self {
        Self {
            heap: BinaryHeap::new(),
        }
    }

    /// Push a task into the queue
    pub fn push(&mut self, task: Task) {
        self.heap.push(PrioritizedTask { task });
    }

    /// Pop the highest priority task from the queue
    pub fn pop(&mut self) -> Option<Task> {
        self.heap.pop().map(|prioritized| prioritized.task)
    }

    /// Peek at the highest priority task without removing it
    pub fn peek(&self) -> Option<&Task> {
        self.heap.peek().map(|prioritized| &prioritized.task)
    }

    /// Check if the queue is empty
    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }

    /// Get the number of tasks in the queue
    pub fn len(&self) -> usize {
        self.heap.len()
    }

    /// Clear all tasks from the queue
    pub fn clear(&mut self) {
        self.heap.clear();
    }
}

impl Default for PriorityQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    #[test]
    fn test_priority_ordering() {
        let mut queue = PriorityQueue::new();

        // Create tasks with different priorities
        let low_priority = Task::new(
            "low".to_string(),
            serde_json::json!({"data": "low priority"}),
        )
        .with_priority(TaskPriority::Low);

        let medium_priority = Task::new(
            "medium".to_string(),
            serde_json::json!({"data": "medium priority"}),
        )
        .with_priority(TaskPriority::Medium);

        let high_priority = Task::new(
            "high".to_string(),
            serde_json::json!({"data": "high priority"}),
        )
        .with_priority(TaskPriority::High);

        let critical_priority = Task::new(
            "critical".to_string(),
            serde_json::json!({"data": "critical priority"}),
        )
        .with_priority(TaskPriority::Critical);

        // Add them in reverse order
        queue.push(low_priority);
        queue.push(medium_priority);
        queue.push(high_priority);
        queue.push(critical_priority);

        // They should come out in priority order
        assert_eq!(queue.pop().unwrap().priority, TaskPriority::Critical);
        assert_eq!(queue.pop().unwrap().priority, TaskPriority::High);
        assert_eq!(queue.pop().unwrap().priority, TaskPriority::Medium);
        assert_eq!(queue.pop().unwrap().priority, TaskPriority::Low);
    }

    #[test]
    fn test_creation_time_ordering() {
        let mut queue = PriorityQueue::new();

        // Create tasks with same priority but different creation times
        let mut task1 = Task::new(
            "task1".to_string(),
            serde_json::json!({"data": "task1"}),
        );
        task1.created_at = Utc::now() - Duration::minutes(10);

        let mut task2 = Task::new(
            "task2".to_string(),
            serde_json::json!({"data": "task2"}),
        );
        task2.created_at = Utc::now() - Duration::minutes(5);

        let mut task3 = Task::new(
            "task3".to_string(),
            serde_json::json!({"data": "task3"}),
        );
        task3.created_at = Utc::now();

        // Add them in reverse order
        queue.push(task3.clone());
        queue.push(task2.clone());
        queue.push(task1.clone());

        // They should come out in order of creation time (oldest first)
        assert_eq!(queue.pop().unwrap().id, task1.id);
        assert_eq!(queue.pop().unwrap().id, task2.id);
        assert_eq!(queue.pop().unwrap().id, task3.id);
    }
}