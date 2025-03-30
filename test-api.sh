#!/bin/bash
# API Testing Script for Distributed Task Queue System

API_BASE="http://localhost:8080/api/v1"
JQ_AVAILABLE=false

# Check if jq is available
if command -v jq &> /dev/null; then
    JQ_AVAILABLE=true
    echo "jq is available, JSON will be formatted."
else
    echo "jq is not installed. JSON output will not be formatted."
    echo "Install jq for better output: 'brew install jq' or 'apt-get install jq'"
fi

# Print formatted JSON
print_json() {
    if [ "$JQ_AVAILABLE" = true ]; then
        echo "$1" | jq .
    else
        echo "$1"
    fi
}

# Function to make an API call
call_api() {
    local name="$1"
    local endpoint="$2"
    local method="$3"
    local payload="$4"
    
    echo "===== $name ====="
    echo "Endpoint: $endpoint"
    echo "Method: $method"
    
    if [ -n "$payload" ]; then
        echo "Payload: $payload"
        RESPONSE=$(curl -s -X "$method" "$API_BASE$endpoint" -H "Content-Type: application/json" -d "$payload")
    else
        RESPONSE=$(curl -s -X "$method" "$API_BASE$endpoint")
    fi
    
    echo "Response:"
    print_json "$RESPONSE"
    echo ""
    
    # Save to variable with sanitized name
    local var_name=$(echo "$name" | tr '[:upper:]' '[:lower:]' | tr ' ' '_')
    eval "${var_name}_response=\"$RESPONSE\""
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

# Extract the task ID
if [ "$JQ_AVAILABLE" = true ]; then
    HIGH_TASK_ID=$(echo "$create_high_priority_task_response" | jq -r '.task_id')
else
    HIGH_TASK_ID=$(echo "$create_high_priority_task_response" | grep -o '"task_id":"[^"]*"' | cut -d':' -f2 | tr -d '"')
fi
echo "Extracted high priority task ID: $HIGH_TASK_ID"
echo ""

# 3. Create a medium priority task
MEDIUM_PRIORITY_TASK='{
    "name": "Medium Priority Task",
    "payload": {"action": "process", "data": "test-medium"},
    "priority": "Medium",
    "max_attempts": 2,
    "tags": ["test", "medium-priority"]
}'
call_api "Create Medium Priority Task" "/tasks" "POST" "$MEDIUM_PRIORITY_TASK"

# Extract the task ID
if [ "$JQ_AVAILABLE" = true ]; then
    MEDIUM_TASK_ID=$(echo "$create_medium_priority_task_response" | jq -r '.task_id')
else
    MEDIUM_TASK_ID=$(echo "$create_medium_priority_task_response" | grep -o '"task_id":"[^"]*"' | cut -d':' -f2 | tr -d '"')
fi
echo "Extracted medium priority task ID: $MEDIUM_TASK_ID"
echo ""

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

# Extract the task ID
if [ "$JQ_AVAILABLE" = true ]; then
    SCHEDULED_TASK_ID=$(echo "$create_scheduled_task_response" | jq -r '.task_id')
else
    SCHEDULED_TASK_ID=$(echo "$create_scheduled_task_response" | grep -o '"task_id":"[^"]*"' | cut -d':' -f2 | tr -d '"')
fi
echo "Extracted scheduled task ID: $SCHEDULED_TASK_ID"
echo ""

# 5. Get high priority task details
call_api "Get High Priority Task" "/tasks/$HIGH_TASK_ID" "GET"

# 6. Get medium priority task details
call_api "Get Medium Priority Task" "/tasks/$MEDIUM_TASK_ID" "GET"

# 7. Get scheduled task details
call_api "Get Scheduled Task" "/tasks/$SCHEDULED_TASK_ID" "GET"

# 8. List all tasks
call_api "List All Tasks" "/tasks" "GET"

# 9. Get task counts
call_api "Get Task Counts" "/tasks/counts" "GET"

# 10. Cancel medium priority task
call_api "Cancel Medium Priority Task" "/tasks/$MEDIUM_TASK_ID/cancel" "POST"

# 11. Check medium priority task status after cancellation
call_api "Check Cancelled Task Status" "/tasks/$MEDIUM_TASK_ID" "GET"

# 12. Get updated task counts
call_api "Get Updated Task Counts" "/tasks/counts" "GET"

# Wait for tasks to process
echo "Waiting 5 seconds for tasks to process..."
sleep 5

# 13. Get high priority task details after processing
call_api "Get High Priority Task After Processing" "/tasks/$HIGH_TASK_ID" "GET"

# 14. Get final task counts
call_api "Get Final Task Counts" "/tasks/counts" "GET"

# Add to test-api.sh - check active connections
echo "Checking database connections..."
docker exec $(docker ps | grep postgres | grep task_queue_system | awk '{print $NF}') \
  psql -U postgres -c "SELECT count(*) FROM pg_stat_activity WHERE datname = 'taskqueue';"

echo "===================================================="
echo "             API Testing Completed                  "
echo "===================================================="