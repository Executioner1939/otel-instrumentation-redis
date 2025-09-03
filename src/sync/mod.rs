//! This module provides an instrumented wrapper around the `redis::Connection` to
//! enable enhanced tracing and monitoring capabilities for Redis operations.
//! The `InstrumentedConnection` enables capturing command spans and attributes,

use crate::common::{apply_span_attributes, create_command_span, record_command_result};
use redis::{Cmd, Connection, ConnectionLike, RedisResult, Value};
use tracing::{instrument, Span};

/// A struct that represents a connection with added instrumentation capabilities.
///
/// The `InstrumentedConnection` wraps an inner `Connection` and can provide additional
/// functionality such as logging, tracking metrics, or monitoring the usage of the connection.
///
/// # Fields
/// - `inner`: The underlying `Connection` object that this struct wraps and extends.
///
/// # Examples
/// ```ignore
/// use otel_instrumentation_redis::sync::InstrumentedConnection;
/// use redis::Connection;
///
/// let client = redis::Client::open("redis://127.0.0.1/").unwrap();
/// let connection = client.get_connection().unwrap();
/// let instrumented_connection = InstrumentedConnection::new(connection);
///
/// // Use `instrumented_connection` as needed
/// ```
pub struct InstrumentedConnection {
    inner: Connection,
}

impl InstrumentedConnection {
    /// Creates a new instance of the struct with the provided database connection.
    ///
    /// # Arguments
    ///
    /// * `connection` - A `Connection` object that represents the database connection.
    ///
    /// # Returns
    ///
    /// Returns a new instance of the struct, with the `inner` field initialized to the provided
    /// `connection`.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let connection = Connection::new();
    /// let instance = StructName::new(connection);
    /// ```
    pub fn new(connection: Connection) -> Self {
        Self { inner: connection }
    }

    /// Returns a reference to the inner `Connection` object.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let connection = manager.inner();
    /// // `connection` is now a reference to the inner `Connection`.
    /// ```
    ///
    /// # Returns
    /// A reference to the `Connection` stored within the struct.
    pub fn inner(&self) -> &Connection {
        &self.inner
    }

    /// Provides mutable access to the inner `Connection` object.
    ///
    /// This method allows modification of the underlying `Connection` instance
    /// by returning a mutable reference to it.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let mut wrapper = Wrapper::new();
    /// let connection = wrapper.inner_mut();
    /// connection.set_timeout(30); // Example of modifying the connection
    /// ```
    ///
    /// # Returns
    ///
    /// A mutable reference to the inner `Connection`.
    ///
    /// # Note
    ///
    /// Use this method with caution, as modifying the inner state could
    /// potentially impact other parts of the code relying on the `Connection` state.
    pub fn inner_mut(&mut self) -> &mut Connection {
        &mut self.inner
    }

    /// Sends a command to the Redis server and handles tracing for the command execution.
    ///
    /// # Parameters
    /// - `cmd`: A reference to the Redis command (`Cmd`) to be executed.
    ///
    /// # Returns
    /// - `RedisResult<Value>`: The result of executing the command, which can be either a successful
    ///   response (`Ok(Value)`) or an error (`Err(RedisError)`).
    ///
    /// # Behavior
    /// 1. A tracing span is created for the command using `create_command_span`, which generates
    ///    a span and attributes based on the command information.
    /// 2. The span is entered, and additional attributes are applied to provide richer tracing context
    ///    using `apply_span_attributes`.
    /// 3. The command is executed by internally delegating to `self.inner.req_command(cmd)`.
    /// 4. The result of the command execution is recorded in the tracing span using `record_command_result`.
    /// 5. The function returns the result of the inner command execution.
    ///
    /// This function is intended to incorporate distributed tracing for enhanced observability and
    /// debugging of Redis command interactions.
    ///
    /// # Tracing
    /// - The tracing span helps track the lifecycle of the Redis command, including its attributes,
    ///   timing, and result.
    /// - If tracing is enabled, this function captures detailed execution data for diagnostics.
    ///
    /// # Examples
    /// ```ignore
    /// use redis::Cmd;
    /// use otel_instrumentation_redis::sync::InstrumentedConnection;
    /// 
    /// let client = redis::Client::open("redis://127.0.0.1/").unwrap();
    /// let conn = client.get_connection().unwrap();
    /// let mut instrumented = InstrumentedConnection::new(conn);
    /// 
    /// let mut cmd = Cmd::new();
    /// cmd.arg("GET").arg("key");
    /// 
    /// match instrumented.req_command(&cmd) {
    ///     Ok(value) => println!("Command succeeded: {:?}", value),
    ///     Err(err) => eprintln!("Command failed: {:?}", err),
    /// }
    /// ```
    ///
    /// # Errors
    /// - Returns a `RedisError` if the command execution fails.
    pub fn req_command(&mut self, cmd: &Cmd) -> RedisResult<Value> {
        let (span, attributes) = create_command_span(cmd);
        let _enter = span.enter();

        // Apply additional attributes
        apply_span_attributes(&span, &attributes);

        // Execute the command
        let result = self.inner.req_command(cmd);

        // Record the result
        record_command_result(&span, &result);

        result
    }

    /// Executes a packed Redis command and records the result.
    ///
    /// This function sends a packed binary command to the Redis server and captures its response.
    /// It utilizes distributed tracing to instrument the operation for observability.
    ///
    /// ## Instrumentation
    /// - This function is instrumented with the `tracing` crate to provide additional context for the operation.
    /// - `db.system` is set to `"redis"`, and `db.operation` is set to `"packed_command"`.
    /// - The tracing span allows for logging and tracing the execution of this operation, including its result.
    ///
    /// ## Parameters
    /// - `self`: A mutable reference to the current Redis connection object.
    /// - `cmd`: A byte slice (`&[u8]`) representing the packed Redis command to be executed.
    ///
    /// ## Returns
    /// - `RedisResult<Value>`: The result of the Redis operation. A `RedisResult` is typically either:
    ///   - `Ok(Value)`: The successful response from the Redis server, where `Value` represents the returned data.
    ///   - `Err(...)`: An error indicating what went wrong during the operation.
    ///
    /// ## Behavior
    /// - The method internally calls `self.inner.req_packed_command(cmd)` to send the command to the Redis server.
    /// - A tracing span is used to track the operation's execution context (`Span::current()`).
    /// - After the command completes, the result (success or error) is recorded using the `record_command_result` utility.
    ///
    /// ## Notes
    /// - The `skip(self, cmd)` directive in the `#[instrument]` macro ensures that the `self` reference and the `cmd` parameter
    ///   are not included in tracing spans to avoid exposing sensitive or verbose data during logs or telemetry.
    ///
    /// ## Example
    /// ```rust,ignore
    /// use otel_instrumentation_redis::Connection; // Replace with the actual module and type
    ///
    /// let mut redis_connection = Connection::new();
    /// let command = b"*2\r\n$4\r\nPING\r\n";
    ///
    /// let result = redis_connection.req_packed_command(command);
    /// match result {
    ///     Ok(value) => println!("Response: {:?}", value),
    ///     Err(e) => eprintln!("Error occurred: {}", e),
    /// }
    /// ```
    #[instrument(
        skip(self, cmd),
        fields(
            db.system = "redis",
            db.operation = "packed_command"
        )
    )]
    pub fn req_packed_command(&mut self, cmd: &[u8]) -> RedisResult<Value> {
        let span = Span::current();

        // Execute the command
        let result = self.inner.req_packed_command(cmd);

        // Record the result
        record_command_result(&span, &result);

        result
    }

    /// Executes a batch of Redis commands in a single pipeline and returns their results.
    ///
    /// This method sends serialized Redis commands (`REQ` commands) to the server, and
    /// executes them in a single pipeline operation. The method is instrumented for
    /// telemetry, logging metadata including the database system, operation type, and
    /// the number of commands being executed.
    ///
    /// # Arguments
    ///
    /// * `cmd` - A byte slice containing the serialized Redis commands to execute.
    /// * `offset` - The offset within the provided command buffer where the execution should start.
    /// * `count` - The number of commands in the pipeline to execute.
    ///
    /// # Returns
    ///
    /// A [`RedisResult`] that wraps a `Vec<Value>`, where each `Value` represents
    /// the response for a corresponding pipeline command. Returns an error if the
    /// execution fails for any reason.
    ///
    /// # Telemetry
    ///
    /// * The operation is instrumented with tracing, using the `instrument` attribute.
    /// * Metadata captured includes:
    ///   - `db.system`: `"redis"`
    ///   - `db.operation`: `"pipeline"`
    ///   - `redis.pipeline.count`: The count of commands executed in the pipeline.
    /// * The span associated with the telemetry will have the current command execution
    ///   results recorded using [`record_command_result`].
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let mut conn = get_redis_connection(); // Get a Redis connection.
    /// let cmd = serialize_pipeline_commands(commands); // Generate serialized commands.
    /// let offset = 0;
    /// let count = 3;
    /// let result = conn.req_packed_commands(&cmd, offset, count);
    /// match result {
    ///     Ok(values) => println!("Pipeline execution results: {:?}", values),
    ///     Err(err) => eprintln!("Pipeline execution failed: {:?}", err),
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error variant of [`RedisResult`] if:
    /// - The connection to Redis fails.
    /// - The provided command buffer or execution parameters are invalid.
    /// - The server returns an error.
    /// ```
    #[instrument(
        skip(self, cmd),
        fields(
            db.system = "redis",
            db.operation = "pipeline",
            redis.pipeline.count = %count
        )
    )]
    pub fn req_packed_commands(
        &mut self,
        cmd: &[u8],
        offset: usize,
        count: usize,
    ) -> RedisResult<Vec<Value>> {
        let span = Span::current();

        // Execute the commands
        let result = self.inner.req_packed_commands(cmd, offset, count);

        // Record the result
        record_command_result(&span, &result);

        result
    }

    /// Convenience method: GET a key with instrumentation
    #[instrument(skip(self, key), fields(db.operation = "GET"))]
    pub fn get<K: redis::ToRedisArgs, RV: redis::FromRedisValue>(
        &mut self,
        key: K,
    ) -> RedisResult<RV> {
        let mut cmd = redis::Cmd::new();
        cmd.arg("GET").arg(key);
        let result = self.req_command(&cmd)?;
        redis::FromRedisValue::from_redis_value(&result)
    }

    /// Convenience method: SET a key with instrumentation
    #[instrument(skip(self, key, value), fields(db.operation = "SET"))]
    pub fn set<K: redis::ToRedisArgs, V: redis::ToRedisArgs>(
        &mut self,
        key: K,
        value: V,
    ) -> RedisResult<()> {
        let mut cmd = redis::Cmd::new();
        cmd.arg("SET").arg(key).arg(value);
        let result = self.req_command(&cmd)?;
        redis::FromRedisValue::from_redis_value(&result)
    }

    /// Convenience method: DEL keys with instrumentation
    #[instrument(skip(self, keys), fields(db.operation = "DEL"))]
    pub fn del<K: redis::ToRedisArgs>(&mut self, keys: K) -> RedisResult<i64> {
        let mut cmd = redis::Cmd::new();
        cmd.arg("DEL").arg(keys);
        let result = self.req_command(&cmd)?;
        redis::FromRedisValue::from_redis_value(&result)
    }

    /// Convenience method: EXISTS check with instrumentation
    #[instrument(skip(self, keys), fields(db.operation = "EXISTS"))]
    pub fn exists<K: redis::ToRedisArgs>(&mut self, keys: K) -> RedisResult<bool> {
        let mut cmd = redis::Cmd::new();
        cmd.arg("EXISTS").arg(keys);
        let result = self.req_command(&cmd)?;
        redis::FromRedisValue::from_redis_value(&result)
    }

    /// Convenience method: EXPIRE key with instrumentation
    #[instrument(skip(self, key), fields(db.operation = "EXPIRE"))]
    pub fn expire<K: redis::ToRedisArgs>(&mut self, key: K, seconds: usize) -> RedisResult<bool> {
        let mut cmd = redis::Cmd::new();
        cmd.arg("EXPIRE").arg(key).arg(seconds);
        let result = self.req_command(&cmd)?;
        redis::FromRedisValue::from_redis_value(&result)
    }

    /// Convenience method: HGET hash field with instrumentation
    #[instrument(skip(self, key, field), fields(db.operation = "HGET"))]
    pub fn hget<K: redis::ToRedisArgs, F: redis::ToRedisArgs, RV: redis::FromRedisValue>(
        &mut self,
        key: K,
        field: F,
    ) -> RedisResult<RV> {
        let mut cmd = redis::Cmd::new();
        cmd.arg("HGET").arg(key).arg(field);
        let result = self.req_command(&cmd)?;
        redis::FromRedisValue::from_redis_value(&result)
    }

    /// Convenience method: HSET hash field with instrumentation
    #[instrument(skip(self, key, field, value), fields(db.operation = "HSET"))]
    pub fn hset<K: redis::ToRedisArgs, F: redis::ToRedisArgs, V: redis::ToRedisArgs>(
        &mut self,
        key: K,
        field: F,
        value: V,
    ) -> RedisResult<bool> {
        let mut cmd = redis::Cmd::new();
        cmd.arg("HSET").arg(key).arg(field).arg(value);
        let result = self.req_command(&cmd)?;
        redis::FromRedisValue::from_redis_value(&result)
    }

    /// Convenience method: SADD to set with instrumentation
    #[instrument(skip(self, key, members), fields(db.operation = "SADD"))]
    pub fn sadd<K: redis::ToRedisArgs, M: redis::ToRedisArgs>(
        &mut self,
        key: K,
        members: M,
    ) -> RedisResult<i64> {
        let mut cmd = redis::Cmd::new();
        cmd.arg("SADD").arg(key).arg(members);
        let result = self.req_command(&cmd)?;
        redis::FromRedisValue::from_redis_value(&result)
    }

    /// Convenience method: SISMEMBER check with instrumentation
    #[instrument(skip(self, key, member), fields(db.operation = "SISMEMBER"))]
    pub fn sismember<K: redis::ToRedisArgs, M: redis::ToRedisArgs>(
        &mut self,
        key: K,
        member: M,
    ) -> RedisResult<bool> {
        let mut cmd = redis::Cmd::new();
        cmd.arg("SISMEMBER").arg(key).arg(member);
        let result = self.req_command(&cmd)?;
        redis::FromRedisValue::from_redis_value(&result)
    }
}

/// A type alias for `InstrumentedConnection`, specifically representing a Redis connection
/// that is instrumented for monitoring or performance tracking purposes.
///
/// This alias simplifies the codebase by providing a more contextually relevant name
/// when dealing with Redis connections.
///
/// # Example
/// ```ignore
/// use otel_instrumentation_redis::sync::InstrumentedRedisConnection;
///
/// fn use_redis_connection(connection: InstrumentedRedisConnection) {
///     // Perform operations with the instrumented Redis connection.
/// }
/// ```
///
/// Note: `InstrumentedConnection` should be a type defined elsewhere in the
/// codebase that integrates some form of instrumentation (such as logging, metrics, or tracing).
pub type InstrumentedRedisConnection = InstrumentedConnection;
