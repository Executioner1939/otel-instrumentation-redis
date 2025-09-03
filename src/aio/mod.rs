//! Asynchronous Redis connection instrumentation

use crate::common::{apply_span_attributes, create_command_span, record_command_result};
use redis::aio::{ConnectionLike, MultiplexedConnection};
use redis::{Cmd, RedisResult, Value};
use tracing::instrument;

/// An instrumented wrapper around an async Redis connection
pub struct InstrumentedAsyncConnection<C> {
    inner: C,
}

impl<C: ConnectionLike> InstrumentedAsyncConnection<C> {
    /// Create a new instrumented async connection
    pub fn new(connection: C) -> Self {
        Self { inner: connection }
    }

    /// Get the underlying connection
    pub fn inner(&self) -> &C {
        &self.inner
    }

    /// Get a mutable reference to the underlying connection
    pub fn inner_mut(&mut self) -> &mut C {
        &mut self.inner
    }

    /// Execute a Redis command with tracing
    pub async fn req_command(&mut self, cmd: &Cmd) -> RedisResult<Value> {
        let (span, attributes) = create_command_span(cmd);
        let _enter = span.enter();

        // Apply additional attributes
        apply_span_attributes(&span, &attributes);

        // Execute the command using the query trait
        let result = cmd.query_async(&mut self.inner).await;

        // Record the result
        record_command_result(&span, &result);

        result
    }

    /// Execute a pipeline of commands with tracing
    pub async fn execute_pipeline(
        &mut self,
        pipeline: &redis::Pipeline,
    ) -> RedisResult<Vec<Value>> {
        let span = tracing::info_span!(
            "redis_pipeline",
            db.system = "redis",
            db.operation = "pipeline"
        );
        let _enter = span.enter();

        // Execute the pipeline
        let result: RedisResult<Vec<Value>> = pipeline.query_async(&mut self.inner).await;

        // Record the result
        record_command_result(&span, &result);

        result
    }

    /// Convenience method: GET a key with instrumentation
    #[instrument(skip(self, key), fields(db.operation = "GET"))]
    pub async fn get<K: redis::ToRedisArgs, RV: redis::FromRedisValue>(
        &mut self,
        key: K,
    ) -> RedisResult<RV> {
        let mut cmd = redis::Cmd::new();
        cmd.arg("GET").arg(key);
        let result = self.req_command(&cmd).await?;
        redis::FromRedisValue::from_redis_value(&result)
    }

    /// Convenience method: SET a key with instrumentation
    #[instrument(skip(self, key, value), fields(db.operation = "SET"))]
    pub async fn set<K: redis::ToRedisArgs, V: redis::ToRedisArgs>(
        &mut self,
        key: K,
        value: V,
    ) -> RedisResult<()> {
        let mut cmd = redis::Cmd::new();
        cmd.arg("SET").arg(key).arg(value);
        let result = self.req_command(&cmd).await?;
        redis::FromRedisValue::from_redis_value(&result)
    }

    /// Convenience method: DEL keys with instrumentation
    #[instrument(skip(self, keys), fields(db.operation = "DEL"))]
    pub async fn del<K: redis::ToRedisArgs>(&mut self, keys: K) -> RedisResult<i64> {
        let mut cmd = redis::Cmd::new();
        cmd.arg("DEL").arg(keys);
        let result = self.req_command(&cmd).await?;
        redis::FromRedisValue::from_redis_value(&result)
    }

    /// Convenience method: EXISTS check with instrumentation
    #[instrument(skip(self, keys), fields(db.operation = "EXISTS"))]
    pub async fn exists<K: redis::ToRedisArgs>(&mut self, keys: K) -> RedisResult<bool> {
        let mut cmd = redis::Cmd::new();
        cmd.arg("EXISTS").arg(keys);
        let result = self.req_command(&cmd).await?;
        redis::FromRedisValue::from_redis_value(&result)
    }

    /// Convenience method: EXPIRE key with instrumentation
    #[instrument(skip(self, key), fields(db.operation = "EXPIRE"))]
    pub async fn expire<K: redis::ToRedisArgs>(
        &mut self,
        key: K,
        seconds: usize,
    ) -> RedisResult<bool> {
        let mut cmd = redis::Cmd::new();
        cmd.arg("EXPIRE").arg(key).arg(seconds);
        let result = self.req_command(&cmd).await?;
        redis::FromRedisValue::from_redis_value(&result)
    }

    /// Convenience method: HGET hash field with instrumentation
    #[instrument(skip(self, key, field), fields(db.operation = "HGET"))]
    pub async fn hget<K: redis::ToRedisArgs, F: redis::ToRedisArgs, RV: redis::FromRedisValue>(
        &mut self,
        key: K,
        field: F,
    ) -> RedisResult<RV> {
        let mut cmd = redis::Cmd::new();
        cmd.arg("HGET").arg(key).arg(field);
        let result = self.req_command(&cmd).await?;
        redis::FromRedisValue::from_redis_value(&result)
    }

    /// Convenience method: HSET hash field with instrumentation
    #[instrument(skip(self, key, field, value), fields(db.operation = "HSET"))]
    pub async fn hset<K: redis::ToRedisArgs, F: redis::ToRedisArgs, V: redis::ToRedisArgs>(
        &mut self,
        key: K,
        field: F,
        value: V,
    ) -> RedisResult<bool> {
        let mut cmd = redis::Cmd::new();
        cmd.arg("HSET").arg(key).arg(field).arg(value);
        let result = self.req_command(&cmd).await?;
        redis::FromRedisValue::from_redis_value(&result)
    }

    /// Convenience method: SADD to set with instrumentation
    #[instrument(skip(self, key, members), fields(db.operation = "SADD"))]
    pub async fn sadd<K: redis::ToRedisArgs, M: redis::ToRedisArgs>(
        &mut self,
        key: K,
        members: M,
    ) -> RedisResult<i64> {
        let mut cmd = redis::Cmd::new();
        cmd.arg("SADD").arg(key).arg(members);
        let result = self.req_command(&cmd).await?;
        redis::FromRedisValue::from_redis_value(&result)
    }

    /// Convenience method: SISMEMBER check with instrumentation
    #[instrument(skip(self, key, member), fields(db.operation = "SISMEMBER"))]
    pub async fn sismember<K: redis::ToRedisArgs, M: redis::ToRedisArgs>(
        &mut self,
        key: K,
        member: M,
    ) -> RedisResult<bool> {
        let mut cmd = redis::Cmd::new();
        cmd.arg("SISMEMBER").arg(key).arg(member);
        let result = self.req_command(&cmd).await?;
        redis::FromRedisValue::from_redis_value(&result)
    }
}

/// An instrumented wrapper around `redis::aio::MultiplexedConnection`
#[derive(Clone)]
pub struct InstrumentedMultiplexedConnection {
    inner: MultiplexedConnection,
}

impl InstrumentedMultiplexedConnection {
    /// Create a new instrumented multiplexed connection
    pub fn new(connection: MultiplexedConnection) -> Self {
        Self { inner: connection }
    }

    /// Get the underlying connection
    pub fn inner(&self) -> &MultiplexedConnection {
        &self.inner
    }

    /// Execute a Redis command with tracing
    pub async fn req_command(&mut self, cmd: &Cmd) -> RedisResult<Value> {
        let (span, attributes) = create_command_span(cmd);
        let _enter = span.enter();

        // Apply additional attributes
        apply_span_attributes(&span, &attributes);

        // Execute the command using the query trait
        let result = cmd.query_async(&mut self.inner).await;

        // Record the result
        record_command_result(&span, &result);

        result
    }

    /// Execute a pipeline of commands with tracing
    pub async fn execute_pipeline(
        &mut self,
        pipeline: &redis::Pipeline,
    ) -> RedisResult<Vec<Value>> {
        let span = tracing::info_span!(
            "redis_pipeline",
            db.system = "redis",
            db.operation = "pipeline"
        );
        let _enter = span.enter();

        // Execute the pipeline
        let result: RedisResult<Vec<Value>> = pipeline.query_async(&mut self.inner).await;

        // Record the result
        record_command_result(&span, &result);

        result
    }

    /// Convenience method: GET a key with instrumentation
    #[instrument(skip(self, key), fields(db.operation = "GET"))]
    pub async fn get<K: redis::ToRedisArgs, RV: redis::FromRedisValue>(
        &mut self,
        key: K,
    ) -> RedisResult<RV> {
        let mut cmd = redis::Cmd::new();
        cmd.arg("GET").arg(key);
        let result = self.req_command(&cmd).await?;
        redis::FromRedisValue::from_redis_value(&result)
    }

    /// Convenience method: SET a key with instrumentation
    #[instrument(skip(self, key, value), fields(db.operation = "SET"))]
    pub async fn set<K: redis::ToRedisArgs, V: redis::ToRedisArgs>(
        &mut self,
        key: K,
        value: V,
    ) -> RedisResult<()> {
        let mut cmd = redis::Cmd::new();
        cmd.arg("SET").arg(key).arg(value);
        let result = self.req_command(&cmd).await?;
        redis::FromRedisValue::from_redis_value(&result)
    }

    /// Convenience method: DEL keys with instrumentation
    #[instrument(skip(self, keys), fields(db.operation = "DEL"))]
    pub async fn del<K: redis::ToRedisArgs>(&mut self, keys: K) -> RedisResult<i64> {
        let mut cmd = redis::Cmd::new();
        cmd.arg("DEL").arg(keys);
        let result = self.req_command(&cmd).await?;
        redis::FromRedisValue::from_redis_value(&result)
    }

    /// Convenience method: EXISTS check with instrumentation
    #[instrument(skip(self, keys), fields(db.operation = "EXISTS"))]
    pub async fn exists<K: redis::ToRedisArgs>(&mut self, keys: K) -> RedisResult<bool> {
        let mut cmd = redis::Cmd::new();
        cmd.arg("EXISTS").arg(keys);
        let result = self.req_command(&cmd).await?;
        redis::FromRedisValue::from_redis_value(&result)
    }

    /// Convenience method: EXPIRE key with instrumentation
    #[instrument(skip(self, key), fields(db.operation = "EXPIRE"))]
    pub async fn expire<K: redis::ToRedisArgs>(
        &mut self,
        key: K,
        seconds: usize,
    ) -> RedisResult<bool> {
        let mut cmd = redis::Cmd::new();
        cmd.arg("EXPIRE").arg(key).arg(seconds);
        let result = self.req_command(&cmd).await?;
        redis::FromRedisValue::from_redis_value(&result)
    }

    /// Convenience method: HGET hash field with instrumentation
    #[instrument(skip(self, key, field), fields(db.operation = "HGET"))]
    pub async fn hget<K: redis::ToRedisArgs, F: redis::ToRedisArgs, RV: redis::FromRedisValue>(
        &mut self,
        key: K,
        field: F,
    ) -> RedisResult<RV> {
        let mut cmd = redis::Cmd::new();
        cmd.arg("HGET").arg(key).arg(field);
        let result = self.req_command(&cmd).await?;
        redis::FromRedisValue::from_redis_value(&result)
    }

    /// Convenience method: HSET hash field with instrumentation
    #[instrument(skip(self, key, field, value), fields(db.operation = "HSET"))]
    pub async fn hset<K: redis::ToRedisArgs, F: redis::ToRedisArgs, V: redis::ToRedisArgs>(
        &mut self,
        key: K,
        field: F,
        value: V,
    ) -> RedisResult<bool> {
        let mut cmd = redis::Cmd::new();
        cmd.arg("HSET").arg(key).arg(field).arg(value);
        let result = self.req_command(&cmd).await?;
        redis::FromRedisValue::from_redis_value(&result)
    }

    /// Convenience method: SADD to set with instrumentation
    #[instrument(skip(self, key, members), fields(db.operation = "SADD"))]
    pub async fn sadd<K: redis::ToRedisArgs, M: redis::ToRedisArgs>(
        &mut self,
        key: K,
        members: M,
    ) -> RedisResult<i64> {
        let mut cmd = redis::Cmd::new();
        cmd.arg("SADD").arg(key).arg(members);
        let result = self.req_command(&cmd).await?;
        redis::FromRedisValue::from_redis_value(&result)
    }

    /// Convenience method: SISMEMBER check with instrumentation
    #[instrument(skip(self, key, member), fields(db.operation = "SISMEMBER"))]
    pub async fn sismember<K: redis::ToRedisArgs, M: redis::ToRedisArgs>(
        &mut self,
        key: K,
        member: M,
    ) -> RedisResult<bool> {
        let mut cmd = redis::Cmd::new();
        cmd.arg("SISMEMBER").arg(key).arg(member);
        let result = self.req_command(&cmd).await?;
        redis::FromRedisValue::from_redis_value(&result)
    }
}
