.PHONY: help build test run docker-build docker-run clean lint

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

build: ## Build the project
	cargo build --release

test: ## Run tests
	cargo test

test-verbose: ## Run tests with output
	cargo test -- --nocapture

run: ## Run the application
	cargo run

docker-build: ## Build Docker image
	docker build -t pokedex-api:latest .

docker-run: ## Run Docker container
	docker run -p 5000:5000 --env-file .env pokedex-api:latest

docker-compose-up: ## Start with docker-compose
	docker compose up -d

docker-compose-down: ## Stop docker-compose
	docker compose down

clean: ## Clean build artifacts
	cargo clean
	docker compose down -v

lint: ## Run clippy
	cargo clippy -- -D warnings

fmt: ## Format code
	cargo fmt

check: ## Check code without building
	cargo check
