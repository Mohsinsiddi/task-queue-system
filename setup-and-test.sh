#!/bin/bash
set -e  # Exit on any error

echo "===== Distributed Task Queue System Setup and Test ====="
echo ""

# Check if Docker is running
echo "Checking Docker..."
if ! docker info > /dev/null 2>&1; then
  echo "Error: Docker is not running. Please start Docker and try again."
  exit 1
fi

# Find the container name
get_postgres_container() {
  POSTGRES_CONTAINER=$(docker ps | grep postgres | grep task_queue_system | awk '{print $NF}')
  echo $POSTGRES_CONTAINER
}

# Build the project
echo "Building the project..."
cargo build --release

# Start PostgreSQL with Docker Compose if not already running
echo "Checking if PostgreSQL is already running..."
POSTGRES_CONTAINER=$(get_postgres_container)
if [ -z "$POSTGRES_CONTAINER" ]; then
  echo "Starting PostgreSQL database..."
  docker-compose up -d
  
  # Wait for container to be created
  echo "Waiting for PostgreSQL container to be created..."
  for i in {1..10}; do
    POSTGRES_CONTAINER=$(get_postgres_container)
    if [ -n "$POSTGRES_CONTAINER" ]; then
      echo "PostgreSQL container created: $POSTGRES_CONTAINER"
      break
    fi
    echo "Waiting for container to be created... ($i/10)"
    sleep 2
  done
else
  echo "PostgreSQL is already running in container: $POSTGRES_CONTAINER"
fi

POSTGRES_CONTAINER=$(get_postgres_container)
if [ -z "$POSTGRES_CONTAINER" ]; then
  echo "Error: PostgreSQL container not found after startup."
  exit 1
fi

# Wait for PostgreSQL to be ready
echo "Waiting for PostgreSQL to be ready..."
for i in {1..30}; do
  if docker exec $POSTGRES_CONTAINER pg_isready -U postgres > /dev/null 2>&1; then
    echo "PostgreSQL is ready!"
    break
  fi
  echo "Waiting for PostgreSQL to start... ($i/30)"
  sleep 2
  if [ $i -eq 30 ]; then
    echo "Error: PostgreSQL failed to start within the time limit."
    echo "Container logs:"
    docker logs $POSTGRES_CONTAINER
    exit 1
  fi
done

# Create database if it doesn't exist
echo "Setting up database..."
docker exec $POSTGRES_CONTAINER psql -U postgres -c "SELECT 1 FROM pg_database WHERE datname = 'taskqueue'" | grep -q 1 || docker exec $POSTGRES_CONTAINER psql -U postgres -c "CREATE DATABASE taskqueue;"
echo "Database setup complete."

# Start the application in background
echo "Starting the application..."
./target/release/task_queue_system &
APP_PID=$!

# Wait for the application to start
echo "Waiting for the application to start..."
for i in {1..15}; do
  if curl -s http://localhost:8080/api/v1/health > /dev/null 2>&1; then
    echo "Application is ready!"
    break
  fi
  echo "Waiting for application to start... ($i/15)"
  sleep 2
  if [ $i -eq 15 ]; then
    echo "Error: Application failed to start within the time limit."
    kill $APP_PID 2>/dev/null || true
    exit 1
  fi
done

echo ""
echo "===== Testing API Endpoints ====="
echo ""

# Check if jq is installed
if ! command -v jq &> /dev/null; then
    echo "Warning: jq is not installed. JSON responses won't be formatted nicely."
    JQ_AVAILABLE=false
else
    JQ_AVAILABLE=true
fi

# Function to make API calls with proper formatting
function call_api {
  echo "🔹 $1"
  echo "  URL: $2"
  echo "  Method: $3"
  if [ -n "$4" ]; then
    echo "  Payload: $4"
    echo "  Response:"
    RESPONSE=$(curl -s -X "$3" "$2" -H "Content-Type: application/json" -d "$4")
  else
    echo "  Response:"
    RESPONSE=$(curl -s -X "$3" "$2")
  fi
  
  if [ "$JQ_AVAILABLE" = true ]; then
    echo "$RESPONSE" | jq . 2>/dev/null || echo "$RESPONSE"
  else
    echo "$RESPONSE"
  fi
  echo ""
  sleep 1
}

# Test health endpoint
call_api "Checking health endpoint" "http://localhost:8080/api/v1/health" "GET"

# Create a high priority task
TASK1_PAYLOAD='{
  "name": "High Priority Task",
  "payload": {"action": "process", "data": "test-high"},
  "priority": "High",
  "max_attempts": 3,
  "tags": ["test", "high-priority"]
}'
call_api "Creating high priority task" "http://localhost:8080/api/v1/tasks" "POST" "$TASK1_PAYLOAD"

# Extract task ID (with or without jq)
if [ "$JQ_AVAILABLE" = true ]; then
  TASK1_ID=$(curl -s -X POST "http://localhost:8080/api/v1/tasks" -H "Content-Type: application/json" -d "$TASK1_PAYLOAD" | jq -r '.task_id' 2>/dev/null)
else
  TASK1_ID=$(curl -s -X POST "http://localhost:8080/api/v1/tasks" -H "Content-Type: application/json" -d "$TASK1_PAYLOAD" | grep -o '"task_id":"[^"]*"' | cut -d':' -f2 | tr -d '"')
fi

# Create a medium priority task
TASK2_PAYLOAD='{
  "name": "Medium Priority Task",
  "payload": {"action": "process", "data": "test-medium"},
  "priority": "Medium",
  "max_attempts": 2,
  "tags": ["test", "medium-priority"]
}'
call_api "Creating medium priority task" "http://localhost:8080/api/v1/tasks" "POST" "$TASK2_PAYLOAD"

# Extract task ID (with or without jq)
if [ "$JQ_AVAILABLE" = true ]; then
  TASK2_ID=$(curl -s -X POST "http://localhost:8080/api/v1/tasks" -H "Content-Type: application/json" -d "$TASK2_PAYLOAD" | jq -r '.task_id' 2>/dev/null)
else
  TASK2_ID=$(curl -s -X POST "http://localhost:8080/api/v1/tasks" -H "Content-Type: application/json" -d "$TASK2_PAYLOAD" | grep -o '"task_id":"[^"]*"' | cut -d':' -f2 | tr -d '"')
fi

# Create a scheduled task for 1 minute from now
FUTURE_TIME=$(date -u -v+1M +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || date -u -d "+1 minute" +"%Y-%m-%dT%H:%M:%SZ")
TASK3_PAYLOAD="{
  \"name\": \"Scheduled Task\",
  \"payload\": {\"action\": \"delayed_process\", \"data\": \"test-scheduled\"},
  \"priority\": \"Critical\",
  \"scheduled_at\": \"$FUTURE_TIME\",
  \"tags\": [\"test\", \"scheduled\"]
}"
call_api "Creating scheduled task" "http://localhost:8080/api/v1/tasks" "POST" "$TASK3_PAYLOAD"

# Get task details
if [ -n "$TASK1_ID" ]; then
  call_api "Getting task details" "http://localhost:8080/api/v1/tasks/$TASK1_ID" "GET"
else
  echo "Skipping task details check as task ID could not be parsed."
fi

# List all tasks
call_api "Listing all tasks" "http://localhost:8080/api/v1/tasks" "GET"

# Get task counts
call_api "Getting task counts" "http://localhost:8080/api/v1/tasks/counts" "GET"

# Cancel a task
if [ -n "$TASK2_ID" ]; then
  call_api "Cancelling a task" "http://localhost:8080/api/v1/tasks/$TASK2_ID/cancel" "POST"
else
  echo "Skipping task cancellation as task ID could not be parsed."
fi

# Wait for tasks to process
echo "Waiting 5 seconds for tasks to process..."
sleep 5

# Get updated task details
if [ -n "$TASK1_ID" ]; then
  call_api "Getting updated task details" "http://localhost:8080/api/v1/tasks/$TASK1_ID" "GET"
else
  echo "Skipping updated task details check as task ID could not be parsed."
fi

# Get updated task counts
call_api "Getting updated task counts" "http://localhost:8080/api/v1/tasks/counts" "GET"

echo ""
echo "===== Cleanup ====="
echo ""

# Ask if user wants to stop the application and database
read -p "Do you want to stop the application and database? (y/n): " STOP_CHOICE
if [[ $STOP_CHOICE == "y" || $STOP_CHOICE == "Y" ]]; then
  # Stop the application
  echo "Stopping the application..."
  kill $APP_PID 2>/dev/null || true

  # Stop and remove the database
  echo "Stopping and removing the database..."
  docker-compose down
  
  echo "Cleanup completed."
else
  echo "The application and database are still running."
  echo "To stop the application manually, run: kill $APP_PID"
  echo "To stop the database, run: docker-compose down"
fi

echo ""
echo "===== Test Complete! ====="