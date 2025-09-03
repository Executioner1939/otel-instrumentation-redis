//! OpenTelemetry instrumentation for redis-rs
//!
//! This crate provides OpenTelemetry tracing instrumentation for the redis-rs crate,
//! enabling distributed tracing and observability for Redis operations.
//!
//! The instrumentation captures:
//! - Redis command names (GET, SET, HGET, etc.)
//! - Database system information (Redis)
//! - Database index for SELECT operations
//! - Error information when operations fail
//! - Timing information for performance monitoring
//!
//! Service names should be configured at the application level through
//! the OpenTelemetry SDK configuration, not within individual instrumentation libraries.
//!
//! # Features
//!
//! - `sync` (default): Synchronous Redis client instrumentation
//! - `aio`: Asynchronous Redis client instrumentation  
//!
//! # Examples
//!
//! ## Synchronous Usage
//!
//! ```rust,ignore
//! use otel_instrumentation_redis::InstrumentedClient;
//! use redis::Client;
//!
//! // Create instrumented client
//! let client = Client::open("redis://127.0.0.1/")?;
//! let instrumented = InstrumentedClient::new(client);
//!
//! // Get a connection
//! let mut conn = instrumented.get_connection()?;
//!
//! // Use convenience methods with automatic instrumentation
//! conn.set("key1", "value1")?;
//! let value: String = conn.get("key1")?;
//! let exists: bool = conn.exists("key1")?;
//!
//! // Or use raw commands for maximum flexibility  
//! let mut cmd = redis::Cmd::new();
//! cmd.arg("HSET").arg("hash1").arg("field1").arg("value1");
//! conn.req_command(&cmd)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Asynchronous Usage
//!
//! ```rust
//! # #[cfg(feature = "aio")]
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! use otel_instrumentation_redis::InstrumentedClient;
//! use redis::Client;
//!
//! let client = Client::open("redis://127.0.0.1/")?;
//! let instrumented = InstrumentedClient::new(client);
//!
//! // Get async connection
//! let mut conn = instrumented.get_multiplexed_async_connection().await?;
//!
//! // Use convenience methods
//! conn.set("async_key", "async_value").await?;
//! let value: String = conn.get("async_key").await?;
//!
//! // Hash operations
//! conn.hset("user:1", "name", "Alice").await?;
//! let name: String = conn.hget("user:1", "name").await?;
//!
//! // Set operations
//! conn.sadd("active_users", "alice").await?;
//! let is_member: bool = conn.sismember("active_users", "alice").await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Service Name Configuration
//!
//! Service names should be configured at the application level through
//! the OpenTelemetry SDK configuration:
//!
//! ```rust,no_run
//! // Example using opentelemetry-sdk (add as dependency)
//! // use opentelemetry_sdk::Resource;
//! // use opentelemetry_semantic_conventions::resource::SERVICE_NAME;
//!
//! // Configure service name at the application level
//! // let resource = Resource::new(vec![
//! //     (SERVICE_NAME, "my-cache-service".into()),
//! // ]);
//!
//! // Use this resource when initializing your tracer provider
//! ```
//!
//! # OpenTelemetry Attributes
//!
//! The following attributes are automatically added to spans:
//!
//! - `db.system`: Always set to "redis"
//! - `db.operation`: The Redis command name (GET, SET, HGET, etc.)
//! - `db.redis.database_index`: Database index for SELECT operations
//! - `error`: Set to true when operations fail
//! - `error.message`: Error message when operations fail
//! - `otel.status_code`: "OK" or "ERROR"
//! - `otel.status_description`: Error description for failures
//!
//! Service name attributes are set at the application level through the OpenTelemetry
//! SDK resource configuration, not by this instrumentation library.
//!
//! # Performance Considerations
//!
//! This instrumentation adds minimal overhead:
//! - Command name extraction is done via efficient byte parsing
//! - Spans are created lazily only when tracing is enabled
//! - No heap allocations for successful operations
//! - Error information is captured without affecting performance of successful operations

pub mod client;
pub mod common;

#[cfg(feature = "sync")]
pub mod sync;

#[cfg(feature = "aio")]
pub mod aio;

pub use client::InstrumentedClient;

/// Re-export commonly used types
pub mod prelude {
    pub use crate::client::InstrumentedClient;

    #[cfg(feature = "sync")]
    pub use crate::sync::*;

    #[cfg(feature = "aio")]
    pub use crate::aio::*;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::{
        create_command_span, extract_command_attributes, generate_span_name, record_error_on_span,
    };
    use redis::Cmd;

    #[test]
    fn test_extract_command_attributes_get() {
        let mut cmd = Cmd::new();
        cmd.arg("GET").arg("test_key");

        let attributes = extract_command_attributes(&cmd);

        // Debug: Print actual attribute keys
        for attr in &attributes {
            eprintln!("Attribute key: {}", attr.key.as_str());
        }

        // Should have at least db.system and db.operation
        assert!(attributes.len() >= 2);

        // Check that we have db.system - use the actual constant
        assert!(attributes.iter().any(|attr| attr.key.as_str()
            == opentelemetry_semantic_conventions::attribute::DB_SYSTEM_NAME));

        // Check that we have db.operation with GET - use the actual constant
        let operation_attr = attributes.iter().find(|attr| {
            attr.key.as_str() == opentelemetry_semantic_conventions::attribute::DB_OPERATION_NAME
        });
        assert!(operation_attr.is_some());
        if let Some(attr) = operation_attr {
            if let opentelemetry::Value::String(op) = &attr.value {
                assert_eq!(op.as_str(), "GET");
            }
        }
    }

    #[test]
    fn test_extract_command_attributes_set() {
        let mut cmd = Cmd::new();
        cmd.arg("SET").arg("test_key").arg("test_value");

        let attributes = extract_command_attributes(&cmd);

        let operation_attr = attributes.iter().find(|attr| {
            attr.key.as_str() == opentelemetry_semantic_conventions::attribute::DB_OPERATION_NAME
        });
        assert!(operation_attr.is_some());
        if let Some(attr) = operation_attr {
            if let opentelemetry::Value::String(op) = &attr.value {
                assert_eq!(op.as_str(), "SET");
            }
        }
    }

    #[test]
    fn test_extract_command_attributes_hget() {
        let mut cmd = Cmd::new();
        cmd.arg("HGET").arg("test_hash").arg("field");

        let attributes = extract_command_attributes(&cmd);

        let operation_attr = attributes.iter().find(|attr| {
            attr.key.as_str() == opentelemetry_semantic_conventions::attribute::DB_OPERATION_NAME
        });
        assert!(operation_attr.is_some());
        if let Some(attr) = operation_attr {
            if let opentelemetry::Value::String(op) = &attr.value {
                assert_eq!(op.as_str(), "HGET");
            }
        }
    }

    #[test]
    fn test_extract_command_attributes_sadd() {
        let mut cmd = Cmd::new();
        cmd.arg("SADD")
            .arg("test_set")
            .arg("member1")
            .arg("member2");

        let attributes = extract_command_attributes(&cmd);

        let operation_attr = attributes.iter().find(|attr| {
            attr.key.as_str() == opentelemetry_semantic_conventions::attribute::DB_OPERATION_NAME
        });
        assert!(operation_attr.is_some());
        if let Some(attr) = operation_attr {
            if let opentelemetry::Value::String(op) = &attr.value {
                assert_eq!(op.as_str(), "SADD");
            }
        }
    }

    #[test]
    fn test_extract_command_attributes_lowercase_input() {
        let mut cmd = Cmd::new();
        cmd.arg("get").arg("test_key"); // lowercase command

        let attributes = extract_command_attributes(&cmd);

        let operation_attr = attributes.iter().find(|attr| {
            attr.key.as_str() == opentelemetry_semantic_conventions::attribute::DB_OPERATION_NAME
        });
        assert!(operation_attr.is_some());
        if let Some(attr) = operation_attr {
            if let opentelemetry::Value::String(op) = &attr.value {
                assert_eq!(op.as_str(), "GET"); // Should be uppercase
            }
        }
    }

    #[test]
    fn test_generate_span_name() {
        assert_eq!(generate_span_name("GET"), "redis get");
        assert_eq!(generate_span_name("SET"), "redis set");
        assert_eq!(generate_span_name("HGET"), "redis hget");
        assert_eq!(generate_span_name("DEL"), "redis del");
    }

    #[test]
    fn test_create_command_span() {
        let mut cmd = Cmd::new();
        cmd.arg("GET").arg("test_key");

        let (_span, attributes) = create_command_span(&cmd);

        // Verify attributes are returned
        assert!(!attributes.is_empty());
        assert!(attributes.iter().any(|attr| attr.key.as_str()
            == opentelemetry_semantic_conventions::attribute::DB_SYSTEM_NAME));
        assert!(attributes.iter().any(|attr| attr.key.as_str()
            == opentelemetry_semantic_conventions::attribute::DB_OPERATION_NAME));
    }

    #[test]
    fn test_empty_command() {
        let cmd = Cmd::new();

        let attributes = extract_command_attributes(&cmd);

        // Should still have db.system, but no db.operation
        assert!(attributes.iter().any(|attr| attr.key.as_str()
            == opentelemetry_semantic_conventions::attribute::DB_SYSTEM_NAME));
        assert!(!attributes.iter().any(|attr| attr.key.as_str()
            == opentelemetry_semantic_conventions::attribute::DB_OPERATION_NAME));
    }

    #[test]
    fn test_error_recording() {
        // Test that we can create error recording functionality
        let span = tracing::info_span!("test_span");

        // Create a mock Redis error
        let error = redis::RedisError::from((redis::ErrorKind::ResponseError, "Test error"));

        // This should not panic and should record error attributes
        record_error_on_span(&span, &error);

        // The test passes if no panic occurs
    }

    #[test]
    fn test_instrumented_client_creation() {
        let client = redis::Client::open("redis://127.0.0.1/").unwrap();
        let _instrumented = InstrumentedClient::new(client);
    }

    #[cfg(feature = "sync")]
    #[test]
    fn test_sync_connection_wrapper() {
        // This is just testing that we can create the wrapper
        // Actual Redis connection testing would require a running Redis instance
        let client = redis::Client::open("redis://127.0.0.1/").unwrap();
        let instrumented_client = InstrumentedClient::new(client);

        // Just test that we can call the method (it will fail without Redis server)
        let result = instrumented_client.get_connection();
        // We expect this to fail without a Redis server, but the method should exist
        assert!(result.is_err());
    }

    #[cfg(feature = "aio")]
    #[tokio::test]
    async fn test_async_connection_wrapper() {
        // This is just testing that we can create the wrapper
        // Actual Redis connection testing would require a running Redis instance
        let client = redis::Client::open("redis://127.0.0.1/").unwrap();
        let instrumented_client = InstrumentedClient::new(client);

        // Just test that we can call the method (it will fail without Redis server)
        let result = instrumented_client.get_multiplexed_async_connection().await;

        // We expect this to fail without a Redis server, but the method should exist
        assert!(result.is_err());
    }
}
