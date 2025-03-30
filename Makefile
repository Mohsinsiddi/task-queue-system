# Distributed Task Queue System Makefile

# Configuration
BINARY_NAME = task_queue_system
PID_FILE = $(BINARY_NAME).pid
LOG_FILE = $(BINARY_NAME).log

# Default target
.PHONY: all
all: build

# Build the application
.PHONY: build
build:
	@echo "Building $(BINARY_NAME)..."
	@cargo build --release

# Start PostgreSQL database
.PHONY: db-start
db-start:
	@echo "Starting PostgreSQL database..."
	@docker-compose down 2>/dev/null || true
	@docker-compose up -d
	@echo "Waiting for PostgreSQL to be ready..."
	@for i in $$(seq 1 30); do \
		CONTAINER=$$(docker ps | grep postgres | grep task_queue_system | awk '{print $$NF}'); \
		if [ -n "$$CONTAINER" ] && docker exec $$CONTAINER pg_isready -U postgres > /dev/null 2>&1; then \
			echo "PostgreSQL is ready!"; \
			break; \
		fi; \
		echo "Waiting for PostgreSQL to start... ($$i/30)"; \
		sleep 2; \
		if [ $$i -eq 30 ]; then \
			echo "Error: PostgreSQL failed to start within the time limit."; \
			exit 1; \
		fi; \
	done
	@CONTAINER=$$(docker ps | grep postgres | grep task_queue_system | awk '{print $$NF}'); \
	echo "Creating database if it doesn't exist..."; \
	docker exec $$CONTAINER psql -U postgres -c "SELECT 1 FROM pg_database WHERE datname = 'taskqueue'" | grep -q 1 || docker exec $$CONTAINER psql -U postgres -c "CREATE DATABASE taskqueue;"
	@echo "Database setup complete."

# Stop PostgreSQL database
.PHONY: db-stop
db-stop:
	@echo "Stopping PostgreSQL database..."
	@docker-compose down
	@echo "Database stopped."

# Start the application
.PHONY: start
start: build
	@echo "Starting $(BINARY_NAME)..."
	@./target/release/$(BINARY_NAME) > $(LOG_FILE) 2>&1 & echo $$! > $(PID_FILE)
	@echo "Application started with PID: $$(cat $(PID_FILE))"
	@echo "Waiting for application to be ready..."
	@for i in $$(seq 1 15); do \
		if curl -s http://localhost:8080/api/v1/health > /dev/null 2>&1; then \
			echo "Application is ready!"; \
			break; \
		fi; \
		echo "Waiting for application to start... ($$i/15)"; \
		sleep 2; \
		if [ $$i -eq 15 ]; then \
			echo "Error: Application failed to start within the time limit."; \
			make stop; \
			exit 1; \
		fi; \
	done

# Stop the application gracefully
.PHONY: stop
stop:
	@if [ -f $(PID_FILE) ]; then \
		PID=$$(cat $(PID_FILE)); \
		if ps -p $$PID > /dev/null; then \
			echo "Stopping application with PID: $$PID..."; \
			kill $$PID; \
			for i in $$(seq 1 10); do \
				if ! ps -p $$PID > /dev/null; then \
					echo "Application stopped successfully."; \
					break; \
				fi; \
				echo "Waiting for application to stop... ($$i/10)"; \
				sleep 1; \
				if [ $$i -eq 10 ]; then \
					echo "Application is taking too long to stop. Force killing..."; \
					kill -9 $$PID 2>/dev/null || true; \
				fi; \
			done; \
		else \
			echo "Application is not running (PID: $$PID not found)."; \
		fi; \
		rm -f $(PID_FILE); \
	else \
		echo "PID file not found. Application may not be running."; \
		pkill -f $(BINARY_NAME) 2>/dev/null || true; \
	fi
	@echo "Application stopped."

# View application logs
.PHONY: logs
logs:
	@if [ -f $(LOG_FILE) ]; then \
		tail -f $(LOG_FILE); \
	else \
		echo "Log file not found. Application may not have been started."; \
	fi

# Check application status
.PHONY: status
status:
	@if [ -f $(PID_FILE) ]; then \
		PID=$$(cat $(PID_FILE)); \
		if ps -p $$PID > /dev/null; then \
			echo "Application is running with PID: $$PID"; \
			ps -p $$PID -o pid,ppid,cmd,%cpu,%mem,etime; \
		else \
			echo "Application is not running (PID: $$PID not found)."; \
		fi; \
	else \
		echo "PID file not found. Application may not be running."; \
	fi
	@echo "Database status:"
	@if docker ps | grep -q postgres | grep -q task_queue_system; then \
		echo "PostgreSQL database is running."; \
		docker ps | grep postgres | grep task_queue_system; \
	else \
		echo "PostgreSQL database is not running."; \
	fi

# Run a simple test
.PHONY: test
test:
	@echo "Testing API endpoints..."
	@echo "1. Health check:"
	@curl -s http://localhost:8080/api/v1/health | jq . || echo "Failed to get health status"
	@echo "2. Creating a test task:"
	@TASK_ID=$$(curl -s -X POST "http://localhost:8080/api/v1/tasks" -H "Content-Type: application/json" -d '{"name":"Test Task","payload":{"action":"test"},"priority":"High","tags":["test"]}' | jq -r .task_id); \
	echo "Created task with ID: $$TASK_ID"; \
	echo "3. Getting task details:"; \
	curl -s "http://localhost:8080/api/v1/tasks/$$TASK_ID" | jq .; \
	echo "4. Getting task counts:"; \
	curl -s "http://localhost:8080/api/v1/tasks/counts" | jq .

# Run end-to-end
.PHONY: run
run: db-start start
	@echo "System is running. Press Ctrl+C to stop."
	@trap "make stop db-stop" EXIT INT TERM; \
	tail -f $(LOG_FILE)

# Clean up
.PHONY: clean
clean: stop db-stop
	@echo "Cleaning up..."
	@rm -f $(PID_FILE) $(LOG_FILE)
	@echo "Clean up complete."

# Show help
.PHONY: help
help:
	@echo "Distributed Task Queue System Makefile"
	@echo ""
	@echo "Available commands:"
	@echo "  make build     - Build the application"
	@echo "  make db-start  - Start PostgreSQL database with Docker Compose"
	@echo "  make db-stop   - Stop PostgreSQL database"
	@echo "  make start     - Start the application"
	@echo "  make stop      - Stop the application"
	@echo "  make run       - Run the complete system (database + application)"
	@echo "  make status    - Check application and database status"
	@echo "  make logs      - View application logs"
	@echo "  make test      - Run a simple API test"
	@echo "  make clean     - Stop everything and clean up files"
	@echo "  make help      - Show this help"