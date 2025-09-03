//! Common utilities and types shared across sync and async implementations

use opentelemetry::KeyValue;
use opentelemetry_semantic_conventions as semconv;

/// Extracts command attributes from a Redis command.
///
/// This function takes a Redis command (`redis::Cmd`) and attempts to extract relevant attributes
/// such as the database system name and the command name. These attributes are returned as
/// a vector of `KeyValue` objects, which can be used for telemetry, logging, or other purposes.
///
/// # Arguments
///
/// * `cmd` - A reference to a `redis::Cmd` object that represents the Redis command
///           to extract attributes from.
///
/// # Returns
///
/// A `Vec<KeyValue>` containing the extracted attributes:
/// * `DB_SYSTEM_NAME` ("redis") - The database system being used (in this case, Redis).
/// * `DB_OPERATION_NAME` - The name of the command (e.g., "GET", "SET"), if it can
///                         be extracted from the provided `cmd`. If the command name
///                         cannot be determined, this attribute is omitted.
///
/// # Example
///
/// ```rust,ignore
/// use redis::Cmd;
/// use your_crate::extract_command_attributes;
///
/// let cmd = Cmd::new(); // Create a Redis command (example)
/// let attributes = extract_command_attributes(&cmd);
///
/// for attribute in attributes {
///     println!("{}: {}", attribute.key(), attribute.value());
/// }
/// ```
///
/// # Note
///
/// This function assumes there exists an external helper function `get_command_name`
/// that parses the Redis command and retrieves its name.
/// If `get_command_name` returns `None`, the `DB_OPERATION_NAME` attribute will not
/// be added to the result vector.
pub fn extract_command_attributes(cmd: &redis::Cmd) -> Vec<KeyValue> {
    let mut attributes = vec![
        KeyValue::new(semconv::attribute::DB_SYSTEM_NAME, "redis"),
    ];

    // Try to extract the command name
    if let Some(cmd_name) = get_command_name(cmd) {
        attributes.push(KeyValue::new(semconv::attribute::DB_OPERATION_NAME, cmd_name));
    }

    attributes
}

/// Extracts the name of a Redis command from a `redis::Cmd` object.
///
/// This function attempts to determine the name of the Redis command
/// based on the first argument of the provided `redis::Cmd` object. It:
/// - Retrieves the first argument of the command, which typically represents the command name.
/// - Converts the first argument from binary data to a UTF-8 string.
/// - Transforms the command name into uppercase to provide a consistent format.
///
/// ## Behavior
/// - If the first argument of the command is `redis::Arg::Cursor`, the function assumes it belongs
///   to the SCAN family of commands and directly returns `"SCAN"`.
/// - If the first argument is a simple byte slice, the function attempts to parse it as UTF-8:
///   - If parsing is successful, the uppercase version of the command name is returned.
///   - If parsing fails, a warning is logged (using the `tracing` crate), and the function returns `None`.
/// - If the command's argument list is empty, the function returns `None`.
///
/// ## Parameters
/// - `cmd`: A reference to a `redis::Cmd` object containing the Redis command and its arguments.
///
/// ## Returns
/// - `Some<String>`: The uppercase Redis command name if successful.
/// - `None`: If the command name cannot be determined or parsed as UTF-8.
///
/// ## Examples
/// ```rust,ignore
/// use redis::{Cmd, Arg};
///
/// // Example command with simple arguments
/// let mut command = Cmd::new();
/// command.arg("GET").arg("key");
/// assert_eq!(get_command_name(&command), Some("GET".to_string()));
///
/// // Cursor-based command
/// let mut command = Cmd::new();
/// command.arg(Arg::Cursor);
/// assert_eq!(get_command_name(&command), Some("SCAN".to_string()));
///
/// // Invalid UTF-8
/// let mut command = Cmd::new();
/// command.arg(vec![0, 159, 146, 150]); // Invalid UTF-8 byte sequence
/// assert_eq!(get_command_name(&command), None);
///
/// // Empty command
/// let command = Cmd::new();
/// assert_eq!(get_command_name(&command), None);
/// ```
///
/// ## Notes
/// - This function assumes that the first argument in the `redis::Cmd` object is always
///   the command name, which is common in Redis command usage.
///
/// ## Logs
/// - If a command name fails UTF-8 parsing, a warning is logged using the `tracing` crate.
fn get_command_name(cmd: &redis::Cmd) -> Option<String> {
    // Get the first argument which should be the command name
    let mut args_iter = cmd.args_iter();
    if let Some(first_arg) = args_iter.next() {
        // Convert arg to bytes slice
        let arg_bytes = match first_arg {
            redis::Arg::Simple(bytes) => bytes,
            redis::Arg::Cursor => return Some("SCAN".to_string()), // Cursor commands are SCAN family
        };
        
        // Convert bytes to string, handling UTF-8 conversion
        match std::str::from_utf8(arg_bytes) {
            Ok(cmd_name) => Some(cmd_name.to_uppercase()),
            Err(_) => {
                // If we can't parse as UTF-8, return None
                tracing::warn!("Failed to parse Redis command name as UTF-8");
                None
            }
        }
    } else {
        None
    }
}

/// Generates a span name for a Redis operation.
///
/// This function takes an operation name as input, converts it to lowercase, 
/// and formats it into a span name prefixed with "redis". The resulting span 
/// name is used for tracing or monitoring purposes to identify the specific 
/// Redis operation being performed.
///
/// # Arguments
///
/// * `operation` - A string slice that holds the Redis operation name. 
///                 For example, "GET", "SET", etc.
///
/// # Returns
///
/// A `String` containing the formatted span name in the format `"redis <operation>"`,
/// where `<operation>` is the lowercase version of the provided operation name.
///
/// # Examples
///
/// ```rust,ignore
/// let span_name = generate_span_name("GET");
/// assert_eq!(span_name, "redis get");
///
/// let span_name = generate_span_name("SET");
/// assert_eq!(span_name, "redis set");
/// ```
pub fn generate_span_name(operation: &str) -> String {
    format!("redis {}", operation.to_lowercase())
}

///
/// Creates a tracing span for a Redis command, along with its associated key-value attributes.
///
/// This function generates a `tracing::Span` with a specific name and attributes for a given Redis command.
/// It extracts the command name and other relevant metadata, sets them as attributes of the span,
/// and provides additional attributes as a vector of key-value pairs.
///
/// # Arguments
///
/// * `cmd` - A reference to a `redis::Cmd` object representing the Redis command.
///
/// # Returns
///
/// A tuple containing:
/// - `tracing::Span`: A span with metadata to track the execution of the Redis command.
/// - `Vec<KeyValue>`: Associated attributes extracted from the command, represented as key-value pairs.
///
/// # Examples
///
/// ```rust,ignore
/// use tracing::info_span;
/// use redis::Cmd;
/// use your_crate::create_command_span;
///
/// let command = redis::cmd("SET").arg("key").arg("value");
/// let (span, attributes) = create_command_span(&command);
///
/// // Use the span for tracing purposes
/// let _entered = span.enter();
/// // Perform Redis operation...
/// ```
///
/// # Notes
///
/// * The span name is generated based on the operation type (e.g., "SET", "GET"). If the command name
///   cannot be extracted, it defaults to "command".
/// * The returned attributes can be used for further enrichment or for logging purposes.
///
/// # See Also
///
/// * `get_command_name` - Utility function to extract the command name.
/// * `generate_span_name` - Function to generate a well-structured span name.
/// * `extract_command_attributes` - Helper to retrieve additional attributes from the command context.
///
pub fn create_command_span(cmd: &redis::Cmd) -> (tracing::Span, Vec<KeyValue>) {
    let attributes = extract_command_attributes(cmd);
    
    // Extract command name for span name
    let operation = get_command_name(cmd).unwrap_or_else(|| "command".to_string());
    let span_name = generate_span_name(&operation);
    
    // Create span with initial attributes
    let span = tracing::info_span!(
        "redis_command",
        otel.name = %span_name,
        db.system = "redis",
        db.operation = %operation
    );
    
    (span, attributes)
}

/// Applies a set of attributes as fields to a given `tracing::Span`.
///
/// This function iterates through a list of attributes (key-value pairs) and maps
/// them to the provided span by recording their values. Only attributes with
/// supported value types (`String`, `i64`, `f64`, and `bool`) are recorded.
/// Unsupported value types are ignored.
///
/// # Parameters
///
/// - `span`: A reference to the `tracing::Span` to which the attributes should be
///   applied.
/// - `attributes`: A slice of key-value pairs representing the attributes to add to
///   the span. Each key-value pair consists of a key (as a string) and a value
///   (`opentelemetry::Value`).
///
/// # Supported Value Types
///
/// The following `opentelemetry::Value` types are supported and will be recorded:
/// - `String`: Strings are recorded directly.
/// - `i64`: 64-bit integers are recorded as is.
/// - `f64`: 64-bit floating point numbers are recorded as is.
/// - `bool`: Boolean values are recorded as is.
///
/// # Behavior
///
/// - For each attribute in the input slice, its key and value are matched against
///   the supported types.
/// - If the value type is supported, it is recorded in the span using its key.
/// - If the value type is unsupported, it will be silently skipped.
///
/// # Example
///
/// ```rust
/// use tracing::Span;
/// use opentelemetry::KeyValue;
///
/// let span = Span::current();
/// let attributes = vec![
///     KeyValue::new("http.method", "GET"),
///     KeyValue::new("http.status_code", 200),
///     KeyValue::new("http.duration", 2.5),
///     KeyValue::new("http.success", true),
/// ];
///
/// apply_span_attributes(&span, &attributes);
/// ```
///
/// In the example above, the attributes are added to the current span as tracing
/// fields. Unsupported types, if any, would simply be ignored.
///
/// # Notes
///
/// - This function assumes that the span is active or exists in a valid scope.
///   Passing an invalid span may result in runtime issues in the `tracing` library.
/// - Custom or complex `opentelemetry::Value` types that don't match the basic
///   supported types are ignored.
///
/// # Errors
///
/// This function does not return errors. However, if the span itself encounters
/// an issue (e.g., if it is invalid), the corresponding attribute recording
/// operation might fail silently depending on the configuration of `tracing`.
pub fn apply_span_attributes(span: &tracing::Span, attributes: &[KeyValue]) {
    for attr in attributes {
        match &attr.value {
            opentelemetry::Value::String(s) => {
                span.record(attr.key.as_str(), s.as_str());
            },
            opentelemetry::Value::I64(i) => {
                span.record(attr.key.as_str(), *i);
            },
            opentelemetry::Value::F64(f) => {
                span.record(attr.key.as_str(), *f);
            },
            opentelemetry::Value::Bool(b) => {
                span.record(attr.key.as_str(), *b);
            },
            _ => {
                // Skip other value types that don't map well to tracing fields
            }
        }
    }
}

/// Records the result of a command execution to a tracing span.
///
/// This function takes a tracing span and a result object (of type `Result`)
/// to log the success or failure of a command execution. On a successful result,
/// it records a status code of "OK". On an error, it delegates the error handling
/// to the `record_error_on_span` function, which will log the error details on the span.
///
/// # Type Parameters
/// - `T`: The success type of the `Result`.
///
/// # Arguments
/// - `span`: A reference to the `tracing::Span` where the command result will be recorded.
/// - `result`: A reference to the `Result` containing either a successful value (`Ok`) or an
///   error (`Err`) of type `redis::RedisError`.
///
/// # Behavior
/// - If `result` is `Ok`, it records the status code "OK" on the given `span`.
/// - If `result` is `Err`, it calls the `record_error_on_span` function to handle
///   the error.
///
/// # Examples
///
/// ```rust,ignore
/// use tracing::span;
/// use tracing::Level;
/// use redis::RedisError;
///
/// let span = span!(Level::INFO, "example_span");
/// let _enter = span.enter();
///
/// let result: Result<(), RedisError> = Ok(());
/// record_command_result(&span, &result);
///
/// let error_result: Result<(), RedisError> = Err(RedisError::from((redis::ErrorKind::IoError, "connection error")));
/// record_command_result(&span, &error_result);
/// ```
///
/// # Dependencies
/// - This function relies on the `tracing` crate for creating and recording spans.
/// - It also depends on the `redis` crate for `RedisError`.
///
/// # Notes
/// Ensure that the `record_error_on_span` function is properly implemented to handle and log
/// error details to the span. This function assumes `record_error_on_span` is already defined elsewhere in the code.
pub fn record_command_result<T>(span: &tracing::Span, result: &Result<T, redis::RedisError>) {
    match result {
        Ok(_) => {
            span.record("otel.status_code", "OK");
        },
        Err(err) => {
            record_error_on_span(span, err);
        }
    }
}

/// Records an error into a given tracing span with detailed metadata for observability.
///
/// # Parameters
///
/// - `span`: A reference to a `tracing::Span` to which the error information will be recorded.
/// - `err`: A reference to a `redis::RedisError` representing the error encountered.
///
/// # Behavior
///
/// This function records the following fields in the specified span:
///
/// - `"error"`: A boolean indicating that an error occurred (set to `true`).
/// - `"error.message"`: The error message as a human-readable string.
/// - `"otel.status_code"`: Set to `"ERROR"` to indicate the error status in OpenTelemetry terms.
/// - `"otel.status_description"`: Contains the error message as a description.
///
/// Additionally, this function categorizes the error type based on the kind of the `redis::RedisError`
/// and records it under the `"error.type"` field. The categorization is as follows:
///
/// - `ResponseError`: Recorded as `"response_error"`.
/// - `AuthenticationFailed`: Recorded as `"authentication_failed"`.
/// - `TypeError`: Recorded as `"type_error"`.
/// - `ExecAbortError`: Recorded as `"exec_abort_error"`.
/// - `BusyLoadingError`: Recorded as `"busy_loading_error"`.
/// - `NoScriptError`: Recorded as `"no_script_error"`.
/// - `InvalidClientConfig`: Recorded as `"invalid_client_config"`.
/// - `Moved`: Recorded as `"moved"`.
/// - `Ask`: Recorded as `"ask"`.
/// - `TryAgain`: Recorded as `"try_again"`.
/// - `ClusterDown`: Recorded as `"cluster_down"`.
/// - `CrossSlot`: Recorded as `"cross_slot"`.
/// - `MasterDown`: Recorded as `"master_down"`.
/// - `IoError`: Recorded as `"io_error"`.
/// - `ClientError`: Recorded as `"client_error"`.
/// - `ExtensionError`: Recorded as `"extension_error"`.
/// - Any other error kind: Recorded as `"unknown"`.
///
/// # Usage
///
/// This function is designed to enhance observability for systems using `tracing` and OpenTelemetry by
/// providing detailed error classification and contextual information about errors encountered in a Redis
/// operation. It enables effective debugging and monitoring in distributed applications.
///
/// # Example
///
/// ```rust,ignore
/// use tracing::Span;
/// use redis::{RedisError, ErrorKind};
///
/// let span = Span::current();
/// let error = RedisError::from((ErrorKind::TypeError, "An invalid type was encountered."));
///
/// record_error_on_span(&span, &error);
/// ```
///
/// In this example, the span will be enriched with error metadata, categorizing the error type as `"type_error"`.
pub fn record_error_on_span(span: &tracing::Span, err: &redis::RedisError) {
    span.record("error", true);
    span.record("error.message", tracing::field::display(err));
    span.record("otel.status_code", "ERROR");
    span.record("otel.status_description", tracing::field::display(err));

    // Add error type categorization for better observability
    match err.kind() {
        redis::ErrorKind::ResponseError => {
            span.record("error.type", "response_error");
        },
        redis::ErrorKind::AuthenticationFailed => {
            span.record("error.type", "authentication_failed");
        },
        redis::ErrorKind::TypeError => {
            span.record("error.type", "type_error");
        },
        redis::ErrorKind::ExecAbortError => {
            span.record("error.type", "exec_abort_error");
        },
        redis::ErrorKind::BusyLoadingError => {
            span.record("error.type", "busy_loading_error");
        },
        redis::ErrorKind::NoScriptError => {
            span.record("error.type", "no_script_error");
        },
        redis::ErrorKind::InvalidClientConfig => {
            span.record("error.type", "invalid_client_config");
        },
        redis::ErrorKind::Moved => {
            span.record("error.type", "moved");
        },
        redis::ErrorKind::Ask => {
            span.record("error.type", "ask");
        },
        redis::ErrorKind::TryAgain => {
            span.record("error.type", "try_again");
        },
        redis::ErrorKind::ClusterDown => {
            span.record("error.type", "cluster_down");
        },
        redis::ErrorKind::CrossSlot => {
            span.record("error.type", "cross_slot");
        },
        redis::ErrorKind::MasterDown => {
            span.record("error.type", "master_down");
        },
        redis::ErrorKind::IoError => {
            span.record("error.type", "io_error");
        },
        redis::ErrorKind::ClientError => {
            span.record("error.type", "client_error");
        },
        redis::ErrorKind::ExtensionError => {
            span.record("error.type", "extension_error");
        },
        _ => {
            span.record("error.type", "unknown");
        }
    }
}

/// Records the result of a Redis command execution and attaches additional context for failed operations.
///
/// This function integrates with the `tracing` crate to provide structured logging and metrics.
/// Upon execution, it records the success or failure of a Redis operation in the provided span,
/// and for failed operations, it adds additional metadata such as the operation type and an optional key pattern
/// for more detailed context.
///
/// # Type Parameters
/// - `T`: The type of the successful result returned from the Redis command.
///
/// # Arguments
/// - `span`: A reference to a `tracing::Span` used to record logs and metrics for the Redis operation.
/// - `result`: A `Result` containing the result of the Redis operation. If the operation fails, it should
///   contain a `redis::RedisError`.
/// - `operation`: A string slice (`&str`) describing the type of Redis operation being performed (e.g., "GET", "SET").
/// - `key_info`: An optional string slice (`&str`) containing information about the key or pattern involved
///   in the Redis operation, if applicable.
///
/// # Behavior
/// - The function calls `record_command_result` to log the general success or failure of the operation in the span.
/// - If the operation fails (`result.is_err()`):
///   - Records the `operation` string in the span under the field `redis.operation_context`.
///   - If `key_info` is provided, it records the `key_info` string under the field `redis.key_pattern`.
///
/// # Examples
///
/// ```rust
/// use tracing::span;
/// use tracing::Level;
/// use redis::RedisError;
///
/// let span = span!(Level::INFO, "redis_command");
/// let result: Result<(), RedisError> = Err(RedisError::from((redis::ErrorKind::IoError, "Connection closed")));
///
/// record_command_result_with_context(&span, &result, "SET", Some("user:123"));
/// ```
///
/// In the example above:
/// - It records the failure of the `SET` command.
/// - Adds contextual information, including the operation type and key pattern (`"user:123"`), to the span.
///
/// # Dependencies
/// - This function depends on the `tracing` crate for recording spans and structured logging.
/// - It also depends on the `redis` crate for handling `RedisError` structures.
pub fn record_command_result_with_context<T>(
    span: &tracing::Span, 
    result: &Result<T, redis::RedisError>,
    operation: &str,
    key_info: Option<&str>
) {
    record_command_result(span, result);
    
    // Add additional context for failed operations
    if result.is_err() {
        span.record("redis.operation_context", operation);
        if let Some(key) = key_info {
            span.record("redis.key_pattern", key);
        }
    }
}