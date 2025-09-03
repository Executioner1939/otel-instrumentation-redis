# OpenTelemetry Instrumentation for Redis

This crate provides OpenTelemetry tracing instrumentation for the `redis-rs` crate, enabling distributed tracing and observability for Redis operations.

## Features

- **Synchronous Support**: Instrument synchronous Redis connections and operations
- **Asynchronous Support**: Instrument async Redis connections including multiplexed connections
- **OpenTelemetry Integration**: Full OpenTelemetry trace context propagation
- **Semantic Conventions**: Follows OpenTelemetry semantic conventions for database operations

## Usage

### Basic Synchronous Example

```rust
use otel_instrumentation_redis::InstrumentedClient;
use redis::{Client, Commands};

let client = Client::open("redis://127.0.0.1/")?;
let instrumented = InstrumentedClient::new(client);
let mut conn = instrumented.get_connection()?;

// Redis operations are now automatically traced
let _: () = conn.set("key", "value")?;
let result: String = conn.get("key")?;
```

### Asynchronous Example

```rust
use otel_instrumentation_redis::InstrumentedClient;
use redis::{Client, AsyncCommands};

let client = Client::open("redis://127.0.0.1/")?;
let instrumented = InstrumentedClient::new(client);
let mut conn = instrumented.get_async_connection().await?;

// Async Redis operations are automatically traced
let _: () = conn.set("key", "value").await?;
let result: String = conn.get("key").await?;
```

## Configuration

### Custom Service Name

You can specify a custom service name for tracing:

```rust
let instrumented = InstrumentedClient::with_service_name(client, "my-redis-service".to_string());
```

### Features

- `sync` (default): Enable synchronous Redis client instrumentation
- `aio`: Enable asynchronous Redis client instrumentation

## Dependencies

- `redis`: Core Redis client functionality
- `tracing`: Structured logging and tracing framework
- `opentelemetry`: OpenTelemetry API and SDK
- `opentelemetry-semantic-conventions`: Standard semantic conventions

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.