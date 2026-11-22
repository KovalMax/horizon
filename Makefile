.DEFAULT_GOAL := help

COMPOSE ?= docker compose
SERVICE ?=

.PHONY: help certs up down restart build ps logs migrate test-api test-web test-browser test-e2e-stack perf-baseline format-web check

help: ## Show available commands
	@awk 'BEGIN {FS = ":.*## "; print "Horizon development commands:"} /^[a-zA-Z_-]+:.*## / {printf "  %-12s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

certs: ## Generate or replace local HTTPS certificates (requires trusting the new CA)
	sh scripts/local-certs.sh
	@echo ''
	@echo 'To trust the local development CA on macOS, run:'
	@echo '  security add-trusted-cert -r trustRoot -k "$$HOME/Library/Keychains/login.keychain-db" .local/certs/local-ca.pem'
	@echo 'Or use Keychain Access; see README.md for trust and security details.'

up: ## Build and start local services in the background
	$(COMPOSE) up -d --build
	@$(MAKE) --no-print-directory urls

down: ## Stop local services, preserving PostgreSQL data
	$(COMPOSE) down

restart: ## Restart running local services, preserving data
	$(COMPOSE) restart
	@$(MAKE) --no-print-directory urls

.PHONY: urls
urls: ## Show configured local URLs (use make ps to check service status)
	@echo ''
	@echo 'Configured local URLs (run make ps to check service status):'
	@echo '  Web:           https://horizon.localhost'
	@echo '  API readiness: https://api.horizon.localhost/health/ready'
	@echo '  API liveness:  https://api.horizon.localhost/health/live'

build: ## Build local service images
	$(COMPOSE) build

ps: ## Show local service status
	$(COMPOSE) ps

logs: ## Follow logs; optionally set SERVICE=api, web, db, or traefik
	$(COMPOSE) logs -f $(SERVICE)

migrate: ## Apply database migrations (requires running API and database)
	$(COMPOSE) exec -T api cargo run --locked -- migrate

test-api: ## Run API tests (requires running API and database)
	$(COMPOSE) exec -T api cargo test --locked

test-web: ## Run frontend tests in a temporary container
	$(COMPOSE) run --build --rm --no-deps web npm test

test-browser: ## Run deterministic Playwright checks in a container
	sh scripts/check-browser.sh deterministic

test-e2e-stack: ## Run signed-in Playwright checks with isolated PostgreSQL and Mailpit
	sh scripts/check-browser.sh stack

perf-baseline: ## Measure API, PostgreSQL, and signed-in browser startup on seeded data
	sh scripts/perf-baseline.sh

format-web: ## Format frontend source and tests in a temporary container
	$(COMPOSE) run --rm --no-deps web npm run format:write

check: ## Run all CI checks with an isolated temporary database
	sh scripts/check.sh
