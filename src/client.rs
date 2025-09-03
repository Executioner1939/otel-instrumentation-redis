//! A module providing an instrumented wrapper around a Redis client for enhanced observability.

use redis::{Client, RedisError};
use tracing::instrument;

/// A struct that wraps around a `Client` to provide additional instrumentation capabilities.
///
/// The `InstrumentedClient` is used as a lightweight wrapper to augment the functionality
/// of a standard `Client`. This struct can be utilized for collecting, tracking, or
/// logging metrics, among other potential instrumentation-related purposes.
///
/// # Derives
/// - `Debug`: Provides a standard implementation for formatting the struct
///   using the `{:?}` formatter for debugging purposes.
/// - `Clone`: Allows for the creation of a new `InstrumentedClient` that is
///   a copy of an existing one.
///
/// # Fields
/// - `inner`: The inner `Client` instance that is being wrapped by this struct.
///
/// # Example
/// ```rust,ignore
/// use your_crate::InstrumentedClient;
/// use your_crate::Client;
///
/// let client = Client::new();
/// let instrumented_client = InstrumentedClient { inner: client };
///
/// println!("{:?}", instrumented_client); // Debug print
/// ```
#[derive(Debug, Clone)]
pub struct InstrumentedClient {
    inner: Client,
}

impl InstrumentedClient {
    /// Creates a new instance of the struct with the provided `client`.
    ///
    /// # Parameters
    /// - `client`: A `Client` instance that is used to initialize the struct.
    ///
    /// # Returns
    /// A new instance of the struct containing the provided `Client`.
    ///
    /// # Attributes
    /// - `#[instrument(skip(client))]`: This attribute is used for tracing and logging purposes,
    ///   and it skips logging or tracing the `client` parameter to avoid capturing large or sensitive data.
    ///
    /// # Example
    /// ```rust,ignore
    /// let client = Client::new(); // Assuming `Client::new()` creates a Client instance.
    /// let instance = StructName::new(client);
    /// ```
    #[instrument(skip(client))]
    pub fn new(client: Client) -> Self {
        Self {
            inner: client,
        }
    }


    /// Returns a reference to the inner `Client` instance.
    ///
    /// # Returns
    ///
    /// A shared reference (`&Client`) to the inner `Client` contained within the struct.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let wrapper = Wrapper { inner: Client::new() };
    /// let client_ref = wrapper.inner();
    /// // `client_ref` is now a reference to the inner `Client`.
    /// ```
    pub fn inner(&self) -> &Client {
        &self.inner
    }


    /// Retrieves a synchronous instrumented Redis connection.
    ///
    /// This function is available only when the `sync` feature is enabled.
    /// It attempts to fetch a connection from the inner Redis client and wraps it
    /// into an `InstrumentedConnection` for monitoring or additional instrumentation.
    ///
    /// # Returns
    ///
    /// - `Ok(crate::sync::InstrumentedConnection)`: A successfully obtained and wrapped Redis connection.
    /// - `Err(RedisError)`: An error if the connection could not be retrieved.
    ///
    /// # Errors
    ///
    /// This function returns a `RedisError` in case the underlying call to `get_connection` fails.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[cfg(feature = "sync")]
    /// {
    ///     let client = YourClientType::new();
    ///     match client.get_connection() {
    ///         Ok(conn) => {
    ///             // Use the instrumented connection here
    ///         },
    ///         Err(err) => {
    ///             eprintln!("Failed to get connection: {}", err);
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// # Feature Flags
    ///
    /// This function requires the `sync` feature to be enabled.
    ///
    /// ```toml,ignore
    /// [dependencies]
    /// your_crate = { version = "1.0", features = ["sync"] }
    /// ```
    #[cfg(feature = "sync")]
    #[instrument(skip(self))]
    pub fn get_connection(&self) -> Result<crate::sync::InstrumentedConnection, RedisError> {
        let conn = self.inner.get_connection()?;
        Ok(crate::sync::InstrumentedConnection::new(conn))
    }

    /// Get a multiplexed asynchronous connection to the Redis server
    #[cfg(feature = "aio")]
    #[instrument(skip(self))]
    pub async fn get_multiplexed_async_connection(&self) -> Result<crate::aio::InstrumentedMultiplexedConnection, RedisError> {
        let conn = self.inner.get_multiplexed_async_connection().await?;
        Ok(crate::aio::InstrumentedMultiplexedConnection::new(conn))
    }

}