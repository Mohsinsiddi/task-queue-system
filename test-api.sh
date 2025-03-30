#!/bin/bash
# Enhanced API Testing Script for Distributed Task Queue System with Detailed Task Logging

API_BASE="http://localhost:8080/api/v1"
JQ_AVAILABLE=false
LOG_FILE="task_queue_test_$(date +%Y%m%d_%H%M%S).log"

# Initialize log file
echo "=============== TASK QUEUE SYSTEM TEST LOG ===============" > "$LOG_FILE"
echo "Started at: $(date)" >> "$LOG_FILE"
echo "" >> "$LOG_FILE"

# Check if jq is available
if command -v jq &> /dev/null; then
    JQ_AVAILABLE=true
    echo "jq is available, JSON will be formatted."
    echo "jq is available, JSON will be formatted." >> "$LOG_FILE"
else
    echo "jq is not installed. JSON output will not be formatted."
    echo "Install jq for better output: 'brew install jq' or 'apt-get install jq'"
    echo "jq is not installed. JSON output will not be formatted." >> "$LOG_FILE"
fi

# Print formatted JSON and log it
print_json() {
    if [ "$JQ_AVAILABLE" = true ]; then
        # Check if the input is valid JSON before attempting to format it
        if echo "$1" | jq . &>/dev/null; then
            echo "$1" | jq .
            echo "$1" | jq . >> "$LOG_FILE"
        else
            echo "Warning: Invalid JSON format. Displaying raw response:"
            echo "Warning: Invalid JSON format. Displaying raw response:" >> "$LOG_FILE"
            echo "$1"
            echo "$1" >> "$LOG_FILE"
        fi
    else
        echo "$1"
        echo "$1" >> "$LOG_FILE"
    fi
}

    # Function to make an API call and log the results
call_api() {
    local name="$1"
    local endpoint="$2"
    local method="$3"
    local payload="$4"
    
    echo "===== $name ====="
    echo "===== $name =====" >> "$LOG_FILE"
    echo "Endpoint: $endpoint"
    echo "Endpoint: $endpoint" >> "$LOG_FILE"
    echo "Method: $method"
    echo "Method: $method" >> "$LOG_FILE"
    echo "Time: $(date)"
    echo "Time: $(date)" >> "$LOG_FILE"
    
    if [ -n "$payload" ]; then
        echo "Payload: $payload"
        echo "Payload: $payload" >> "$LOG_FILE"
        RESPONSE=$(curl -s -X "$method" "$API_BASE$endpoint" -H "Content-Type: application/json" -d "$payload")
    else
        RESPONSE=$(curl -s -X "$method" "$API_BASE$endpoint")
    fi
    
    echo "Response:"
    echo "Response:" >> "$LOG_FILE"
    print_json "$RESPONSE"
    echo ""
    echo "" >> "$LOG_FILE"
    
    # Save to variable with sanitized name, escaping special characters to avoid shell interpretation issues
    local var_name=$(echo "$name" | tr '[:upper:]' '[:lower:]' | tr ' ' '_')
    # Use printf to properly escape the response to avoid issues with special characters
    ESCAPED_RESPONSE=$(printf '%s' "$RESPONSE" | sed 's/"/\\"/g')
    eval "${var_name}_response=\"$ESCAPED_RESPONSE\""
}

# Function to log task details
log_task_details() {
    local task_id="$1"
    local task_name="$2"
    
    echo "===== Detailed Log for Task: $task_name (ID: $task_id) ====="
    echo "===== Detailed Log for Task: $task_name (ID: $task_id) =====" >> "$LOG_FILE"
    
    # Get task details
    local TASK_DETAILS=$(curl -s -X "GET" "$API_BASE/tasks/$task_id")
    
    # Extract key information using jq if available
    if [ "$JQ_AVAILABLE" = true ]; then
        # Add error handling for jq parsing
        local jq_err
        local status=$(echo "$TASK_DETAILS" | jq -r '.status' 2>/dev/null)
        jq_err=$?
        
        if [ $jq_err -ne 0 ]; then
            echo "Warning: Error parsing task details with jq. Displaying raw response instead."
            echo "Warning: Error parsing task details with jq. Displaying raw response instead." >> "$LOG_FILE"
            echo "$TASK_DETAILS"
            echo "$TASK_DETAILS" >> "$LOG_FILE"
            return
        fi
        
        local priority=$(echo "$TASK_DETAILS" | jq -r '.priority' 2>/dev/null || echo "Unknown")
        local created_at=$(echo "$TASK_DETAILS" | jq -r '.created_at' 2>/dev/null || echo "Unknown")
        local scheduled_at=$(echo "$TASK_DETAILS" | jq -r '.scheduled_at' 2>/dev/null || echo "null")
        local started_at=$(echo "$TASK_DETAILS" | jq -r '.started_at' 2>/dev/null || echo "null")
        local completed_at=$(echo "$TASK_DETAILS" | jq -r '.completed_at' 2>/dev/null || echo "null")
        local max_attempts=$(echo "$TASK_DETAILS" | jq -r '.max_attempts' 2>/dev/null || echo "Unknown")
        local attempt_count=$(echo "$TASK_DETAILS" | jq -r '.attempt_count' 2>/dev/null || echo "Unknown")
        local payload=$(echo "$TASK_DETAILS" | jq -r '.payload' 2>/dev/null || echo "Unknown")
        local tags=$(echo "$TASK_DETAILS" | jq -r '.tags | join(", ")' 2>/dev/null || echo "Unknown")
        
        echo "Status: $status"
        echo "Status: $status" >> "$LOG_FILE"
        echo "Priority: $priority"
        echo "Priority: $priority" >> "$LOG_FILE"
        echo "Created at: $created_at"
        echo "Created at: $created_at" >> "$LOG_FILE"
        
        if [ "$scheduled_at" != "null" ]; then
            echo "Scheduled to be processed at: $scheduled_at"
            echo "Scheduled to be processed at: $scheduled_at" >> "$LOG_FILE"
            
            # Calculate time until scheduled processing
            if [ "$(uname)" == "Darwin" ]; then
                # macOS
                now=$(date -u +%s)
                scheduled=$(date -j -u -f "%Y-%m-%dT%H:%M:%SZ" "$scheduled_at" +%s 2>/dev/null)
            else
                # Linux
                now=$(date -u +%s)
                scheduled=$(date -u -d "$scheduled_at" +%s 2>/dev/null)
            fi
            
            if [ -n "$scheduled" ]; then
                seconds_until=$((scheduled - now))
                if [ $seconds_until -gt 0 ]; then
                    echo "Time until processing: $seconds_until seconds"
                    echo "Time until processing: $seconds_until seconds" >> "$LOG_FILE"
                else
                    echo "Should be processed now (scheduled time has passed)"
                    echo "Should be processed now (scheduled time has passed)" >> "$LOG_FILE"
                fi
            fi
        else
            echo "No scheduled time (will be processed based on priority and queue)"
            echo "No scheduled time (will be processed based on priority and queue)" >> "$LOG_FILE"
        fi
        
        if [ "$started_at" != "null" ]; then
            echo "Started processing at: $started_at"
            echo "Started processing at: $started_at" >> "$LOG_FILE"
        fi
        
        if [ "$completed_at" != "null" ]; then
            echo "Completed at: $completed_at"
            echo "Completed at: $completed_at" >> "$LOG_FILE"
        fi
        
        echo "Max attempts: $max_attempts"
        echo "Max attempts: $max_attempts" >> "$LOG_FILE"
        echo "Attempt count: $attempt_count"
        echo "Attempt count: $attempt_count" >> "$LOG_FILE"
        echo "Payload: $payload"
        echo "Payload: $payload" >> "$LOG_FILE"
        echo "Tags: $tags"
        echo "Tags: $tags" >> "$LOG_FILE"
    else
        # Fallback if jq is not available
        echo "Task details (raw):"
        echo "Task details (raw):" >> "$LOG_FILE"
        echo "$TASK_DETAILS"
        echo "$TASK_DETAILS" >> "$LOG_FILE"
    fi
    
    echo ""
    echo "" >> "$LOG_FILE"
}

# Function to monitor task progress
monitor_task_progress() {
    local task_id="$1"
    local task_name="$2"
    local checks="${3:-3}"  # Default to 3 checks if not specified
    local interval="${4:-10}"  # Default to 10 seconds interval
    
    echo "===== Monitoring Progress for Task: $task_name (ID: $task_id) ====="
    echo "===== Monitoring Progress for Task: $task_name (ID: $task_id) =====" >> "$LOG_FILE"
    echo "Will check $checks times with $interval second intervals"
    echo "Will check $checks times with $interval second intervals" >> "$LOG_FILE"
    echo ""
    echo "" >> "$LOG_FILE"
    
    for (( i=1; i<=$checks; i++ )); do
        echo "Check $i of $checks at $(date)"
        echo "Check $i of $checks at $(date)" >> "$LOG_FILE"
        
        local TASK_DETAILS=$(curl -s -X "GET" "$API_BASE/tasks/$task_id")
        
        if [ "$JQ_AVAILABLE" = true ]; then
            # Add error handling for jq parsing
            local jq_err
            local status=$(echo "$TASK_DETAILS" | jq -r '.status' 2>/dev/null)
            jq_err=$?
            
            if [ $jq_err -ne 0 ]; then
                echo "Warning: Error parsing task details with jq. Displaying raw response instead."
                echo "Warning: Error parsing task details with jq. Displaying raw response instead." >> "$LOG_FILE"
                echo "$TASK_DETAILS"
                echo "$TASK_DETAILS" >> "$LOG_FILE"
                continue
            fi
            
            local started_at=$(echo "$TASK_DETAILS" | jq -r '.started_at' 2>/dev/null || echo "null")
            local completed_at=$(echo "$TASK_DETAILS" | jq -r '.completed_at' 2>/dev/null || echo "null")
            
            echo "Status: $status"
            echo "Status: $status" >> "$LOG_FILE"
            
            if [ "$started_at" != "null" ]; then
                echo "Started processing at: $started_at"
                echo "Started processing at: $started_at" >> "$LOG_FILE"
            fi
            
            if [ "$completed_at" != "null" ]; then
                echo "Completed at: $completed_at"
                echo "Completed at: $completed_at" >> "$LOG_FILE"
                break  # Task is completed, no need to check more
            fi
        else
            echo "$TASK_DETAILS"
            echo "$TASK_DETAILS" >> "$LOG_FILE"
        fi
        
        echo ""
        echo "" >> "$LOG_FILE"
        
        # Don't sleep after the last check
        if [ $i -lt $checks ]; then
            echo "Waiting $interval seconds until next check..."
            echo "Waiting $interval seconds until next check..." >> "$LOG_FILE"
            sleep $interval
        fi
    done
}

echo "===================================================="
echo "      Distributed Task Queue System API Tests       "
echo "===================================================="
echo ""

# 1. Health Check
call_api "Health Check" "/health" "GET"

# 2. Create a high priority task
HIGH_PRIORITY_TASK='{
    "name": "High Priority Task",
    "payload": {"action": "process", "data": "test-high"},
    "priority": "High",
    "max_attempts": 3,
    "tags": ["test", "high-priority"]
}'
call_api "Create High Priority Task" "/tasks" "POST" "$HIGH_PRIORITY_TASK"

    # Extract the task ID with error handling
if [ "$JQ_AVAILABLE" = true ]; then
    HIGH_TASK_ID=$(echo "$create_high_priority_task_response" | jq -r '.task_id' 2>/dev/null)
    # Check if extraction failed
    if [ -z "$HIGH_TASK_ID" ] || [ "$HIGH_TASK_ID" = "null" ]; then
        echo "Warning: Failed to extract task ID using jq, falling back to grep method"
        HIGH_TASK_ID=$(echo "$create_high_priority_task_response" | grep -o '"task_id":"[^"]*"' | cut -d':' -f2 | tr -d '"')
    fi
else
    HIGH_TASK_ID=$(echo "$create_high_priority_task_response" | grep -o '"task_id":"[^"]*"' | cut -d':' -f2 | tr -d '"')
fi
echo "Extracted high priority task ID: $HIGH_TASK_ID"
echo "Extracted high priority task ID: $HIGH_TASK_ID" >> "$LOG_FILE"
echo ""
echo "" >> "$LOG_FILE"

# Log high priority task details
log_task_details "$HIGH_TASK_ID" "High Priority Task"

# 3. Create a medium priority task
MEDIUM_PRIORITY_TASK='{
    "name": "Medium Priority Task",
    "payload": {"action": "process", "data": "test-medium"},
    "priority": "Medium",
    "max_attempts": 2,
    "tags": ["test", "medium-priority"]
}'
call_api "Create Medium Priority Task" "/tasks" "POST" "$MEDIUM_PRIORITY_TASK"

    # Extract the task ID with error handling
if [ "$JQ_AVAILABLE" = true ]; then
    MEDIUM_TASK_ID=$(echo "$create_medium_priority_task_response" | jq -r '.task_id' 2>/dev/null)
    # Check if extraction failed
    if [ -z "$MEDIUM_TASK_ID" ] || [ "$MEDIUM_TASK_ID" = "null" ]; then
        echo "Warning: Failed to extract task ID using jq, falling back to grep method"
        MEDIUM_TASK_ID=$(echo "$create_medium_priority_task_response" | grep -o '"task_id":"[^"]*"' | cut -d':' -f2 | tr -d '"')
    fi
else
    MEDIUM_TASK_ID=$(echo "$create_medium_priority_task_response" | grep -o '"task_id":"[^"]*"' | cut -d':' -f2 | tr -d '"')
fi
echo "Extracted medium priority task ID: $MEDIUM_TASK_ID"
echo "Extracted medium priority task ID: $MEDIUM_TASK_ID" >> "$LOG_FILE"
echo ""
echo "" >> "$LOG_FILE"

# Log medium priority task details
log_task_details "$MEDIUM_TASK_ID" "Medium Priority Task"

# 4. Create a scheduled task for 1 minute from now
FUTURE_TIME=$(date -u -v+1M +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || date -u -d "+1 minute" +"%Y-%m-%dT%H:%M:%SZ")
SCHEDULED_TASK="{
    \"name\": \"Scheduled Task\",
    \"payload\": {\"action\": \"delayed_process\", \"data\": \"test-scheduled\"},
    \"priority\": \"Critical\",
    \"scheduled_at\": \"$FUTURE_TIME\",
    \"tags\": [\"test\", \"scheduled\"]
}"
call_api "Create Scheduled Task" "/tasks" "POST" "$SCHEDULED_TASK"

    # Extract the task ID with error handling
if [ "$JQ_AVAILABLE" = true ]; then
    SCHEDULED_TASK_ID=$(echo "$create_scheduled_task_response" | jq -r '.task_id' 2>/dev/null)
    # Check if extraction failed
    if [ -z "$SCHEDULED_TASK_ID" ] || [ "$SCHEDULED_TASK_ID" = "null" ]; then
        echo "Warning: Failed to extract task ID using jq, falling back to grep method"
        SCHEDULED_TASK_ID=$(echo "$create_scheduled_task_response" | grep -o '"task_id":"[^"]*"' | cut -d':' -f2 | tr -d '"')
    fi
else
    SCHEDULED_TASK_ID=$(echo "$create_scheduled_task_response" | grep -o '"task_id":"[^"]*"' | cut -d':' -f2 | tr -d '"')
fi
echo "Extracted scheduled task ID: $SCHEDULED_TASK_ID"
echo "Extracted scheduled task ID: $SCHEDULED_TASK_ID" >> "$LOG_FILE"
echo ""
echo "" >> "$LOG_FILE"

# Log scheduled task details
log_task_details "$SCHEDULED_TASK_ID" "Scheduled Task"

# 5. Create a low priority task
LOW_PRIORITY_TASK='{
    "name": "Low Priority Task",
    "payload": {"action": "process", "data": "test-low"},
    "priority": "Low",
    "max_attempts": 1,
    "tags": ["test", "low-priority"]
}'
call_api "Create Low Priority Task" "/tasks" "POST" "$LOW_PRIORITY_TASK"

    # Extract the task ID with error handling
if [ "$JQ_AVAILABLE" = true ]; then
    LOW_TASK_ID=$(echo "$create_low_priority_task_response" | jq -r '.task_id' 2>/dev/null)
    # Check if extraction failed
    if [ -z "$LOW_TASK_ID" ] || [ "$LOW_TASK_ID" = "null" ]; then
        echo "Warning: Failed to extract task ID using jq, falling back to grep method"
        LOW_TASK_ID=$(echo "$create_low_priority_task_response" | grep -o '"task_id":"[^"]*"' | cut -d':' -f2 | tr -d '"')
    fi
else
    LOW_TASK_ID=$(echo "$create_low_priority_task_response" | grep -o '"task_id":"[^"]*"' | cut -d':' -f2 | tr -d '"')
fi
echo "Extracted low priority task ID: $LOW_TASK_ID"
echo "Extracted low priority task ID: $LOW_TASK_ID" >> "$LOG_FILE"
echo ""
echo "" >> "$LOG_FILE"

# Log low priority task details
log_task_details "$LOW_TASK_ID" "Low Priority Task"

# 6. Get all task details
call_api "Get High Priority Task" "/tasks/$HIGH_TASK_ID" "GET"
call_api "Get Medium Priority Task" "/tasks/$MEDIUM_TASK_ID" "GET"
call_api "Get Scheduled Task" "/tasks/$SCHEDULED_TASK_ID" "GET"
call_api "Get Low Priority Task" "/tasks/$LOW_TASK_ID" "GET"

# 7. List all tasks
call_api "List All Tasks" "/tasks" "GET"

# 8. Get task counts
call_api "Get Task Counts" "/tasks/counts" "GET"

# 9. Cancel medium priority task
call_api "Cancel Medium Priority Task" "/tasks/$MEDIUM_TASK_ID/cancel" "POST"

# 10. Check medium priority task status after cancellation
call_api "Check Cancelled Task Status" "/tasks/$MEDIUM_TASK_ID" "GET"

# 11. Get updated task counts
call_api "Get Updated Task Counts" "/tasks/counts" "GET"

# Monitor tasks progress
echo "Starting task monitoring..."
echo "Starting task monitoring..." >> "$LOG_FILE"

# Monitor high priority task (should start processing quickly)
monitor_task_progress "$HIGH_TASK_ID" "High Priority Task" 3 5

# Monitor scheduled task (should wait until scheduled time)
monitor_task_progress "$SCHEDULED_TASK_ID" "Scheduled Task" 4 20

# Monitor low priority task (should be processed after high priority task)
monitor_task_progress "$LOW_TASK_ID" "Low Priority Task" 3 5

# Get final status for all tasks
echo "===== Final Status of All Tasks ====="
echo "===== Final Status of All Tasks =====" >> "$LOG_FILE"
echo "Time: $(date)"
echo "Time: $(date)" >> "$LOG_FILE"
echo ""
echo "" >> "$LOG_FILE"

log_task_details "$HIGH_TASK_ID" "High Priority Task (Final)"
log_task_details "$MEDIUM_TASK_ID" "Medium Priority Task (Final)"
log_task_details "$SCHEDULED_TASK_ID" "Scheduled Task (Final)"
log_task_details "$LOW_TASK_ID" "Low Priority Task (Final)"

# 12. Get final task counts
call_api "Get Final Task Counts" "/tasks/counts" "GET"

# Check active database connections
echo "Checking database connections..."
echo "Checking database connections..." >> "$LOG_FILE"
# Add error handling for the docker command
if POSTGRES_CONTAINER=$(docker ps | grep postgres | grep task_queue_system | awk '{print $NF}' 2>/dev/null); then
  if [ -n "$POSTGRES_CONTAINER" ]; then
    DB_CONNECTIONS=$(docker exec $POSTGRES_CONTAINER \
      psql -U postgres -c "SELECT count(*) FROM pg_stat_activity WHERE datname = 'taskqueue';" 2>/dev/null)
    if [ $? -eq 0 ]; then
      echo "$DB_CONNECTIONS"
      echo "$DB_CONNECTIONS" >> "$LOG_FILE"
    else
      echo "Error executing database query. Check if PostgreSQL is running properly."
      echo "Error executing database query. Check if PostgreSQL is running properly." >> "$LOG_FILE"
    fi
  else
    echo "PostgreSQL container not found. Check if the container is running."
    echo "PostgreSQL container not found. Check if the container is running." >> "$LOG_FILE"
  fi
else
  echo "Error executing docker command. Check if Docker is running."
  echo "Error executing docker command. Check if Docker is running." >> "$LOG_FILE"
fi

echo "===================================================="
echo "             API Testing Completed                  "
echo "===================================================="
echo "A complete log has been saved to: $LOG_FILE"