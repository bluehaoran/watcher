# CLAUDE.md - Uatu Watcher Development Guide

## Project Overview

High-performance Rust application for tracking price/version/number changes on web pages with extensible plugin system for tracking types and notification methods.

## Core Features

- **Web Scraping**: Headless Chrome scraping with JS support, retry logic, concurrent processing
- **Product Management**: Multi-URL product tracking, price comparison, best deal identification
- **Plugin System**: Extensible trackers (price, version, number) and notifiers (email, Discord)
- **Element Selection**: Interactive picker, fuzzy matching, reliable selector storage
- **Change Detection**: Configurable rules with absolute/relative thresholds
- **Web Interface**: Dashboard, setup wizard, visual element selector

## Tech Stack

- **Rust 1.75+**, Axum web framework, SQLite + SQLx, Tokio async runtime
- **Docker + Alpine**, headless Chrome, HTMX frontend
- See [Cargo.toml](./Cargo.toml) for full dependency list

## File Structure

- **Database**: See [migrations/](./migrations/) for SQLx schema
- **Plugin System**: Core traits in [src/plugins/traits/](./src/plugins/traits/)
- **Configuration**: [config/default.toml](./config/default.toml), [.env.example](./.env.example)
- **Docker**: [Dockerfile](./Dockerfile), [docker-compose.yml](./docker-compose.yml)

## Quick Start

1. **Setup Environment**: `cp .env.example .env` and configure
2. **Build**: `cargo build --release`
3. **Database**: `sqlx migrate run`
4. **Run**: `cargo run` or `docker-compose up`

## Development Commands

```bash
cargo watch -x run        # Auto-reload development
cargo test               # Run tests
cargo clippy             # Lint code
sqlx migrate run         # Apply database migrations
```

## Key Implementation Features

- **Zero-copy parsing** for performance
- **Compile-time SQL queries** with SQLx
- **Plugin system** via Rust traits
- **Concurrent scraping** with Tokio
- **Type-safe configuration** with validation

## Development Instructions

- Before development, run linting, tests and build to establish a baseline.
    - If errors are detected, fix these, stub these or mark test as incomplete. 
    - If warnings are detected, note these.
- During development, use Test Driven Development. 
    - Understand the functionality, 
    - get human input if necessary, and then 
    - write tests that check for the success conditions. These should fail initially.
    - write functionality until new tests pass.
- After development, run linting, tests, and build to check that no new warnings have been introduced.