//! # rsntp
//!
//! An [RFC 4330](https://tools.ietf.org/html/rfc4330) compliant Simple Network Time Protocol (SNTP) client
//! library for Rust.
//!
//! `rsntp` provides an API to synchronize time with SNTPv4 time servers with the following features:
//!
//! * Provides both a synchronous (blocking) and an (optional) asynchronous API based `tokio`
//! * Time and date handling based on the `chrono` crate (can be disabled)
//! * IPv6 support
//!
//! ## Usage
//!
//! Add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! rsntp = "2.1.0"
//! ```
//!
#![cfg_attr(
    feature = "async",
    doc = r##"

//! Obtain the current local time with the blocking API:
//!
//! ```no_run
//! use rsntp::SntpClient;
//! use chrono::{DateTime, Local};
//!
//! let client = SntpClient::new();
//! let result = client.synchronize("pool.ntp.org").unwrap();
//!
//! let local_time: DateTime<Local> = DateTime::from(result.datetime().as_chrono_datetime_utc());
//!
//! println!("Current time is: {}", local_time);
//! ```
//!
//! "##
)]
#![cfg_attr(
    all(feature = "async", feature = "chrono"),
    doc = r##"

A function which uses the asynchronous API to obtain local time:

```no_run
use rsntp::AsyncSntpClient;
use chrono::{DateTime, Local};

async fn local_time() -> DateTime<Local> {
  let client = AsyncSntpClient::new();
  let result = client.synchronize("pool.ntp.org").await.unwrap();
  
  DateTime::from(result.datetime().as_chrono_datetime_utc())
}
```
## Disabling asynchronous API

The asynchronous API is enabled by default but you can optionally disable it. This removes
dependency to `tokio` which reduces crate dependencies significantly.

```toml
[dependencies]
rsntp = { version = "2.1.0", default-features = false }
```
"##
)]
//! ## IPv6 support
//!
//! `rsntp` supports IPv6, but by default (for compatilibty reasons) it binds its UDP socket to an
//! IPv4 address (0.0.0.0) which might prevent synchronization with IPv6 servers.
//!
//! To use IPv6, you need to set an IPv6 bind address:
//!
//! ```no_run
//! use rsntp::{Config, SntpClient};
//! use std::net::Ipv6Addr;
//!
//! let config = Config::default().bind_address((Ipv6Addr::UNSPECIFIED, 0).into());
//! let client = SntpClient::with_config(config);
//!
//! let result = client.synchronize("2.pool.ntp.org").unwrap();
//!
//! let unix_timestamp_utc = result.datetime().unix_timestamp();
//! ```
//!

mod core_logic;
mod error;
mod packet;
mod result;
mod to_server_addrs;

pub use error::{KissCode, ProtocolError, SynchroniztationError};
pub use packet::{LeapIndicator, ReferenceIdentifier};
pub use result::{SntpDateTime, SntpDuration, SynchronizationResult};
pub use to_server_addrs::ToServerAddrs;

use core_logic::{Reply, Request};
use packet::Packet;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

#[cfg(feature = "async")]
use tokio::time::timeout;

const SNTP_PORT: u16 = 123;

/// Client configuration
///
/// This is a struct which contains the configuration of a client. It uses a builder-like pattern
/// to set parameters. Its main aim is to be able to create client instances with non-default
/// configuration without making them mutable.
///
/// # Example
///
/// ```no_run
/// use rsntp::{Config, SntpClient};
/// use std::time::Duration;
///
/// let config = Config::default().bind_address("192.168.0.1:0".parse().unwrap()).timeout(Duration::from_secs(10));
/// let client = SntpClient::with_config(config);
/// ```
#[derive(Clone, Debug, Hash)]
pub struct Config {
    bind_address: SocketAddr,
    timeout: Duration,
}

impl Config {
    /// Creates an instance with default configuration
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rsntp::Config;
    ///
    /// let config = Config::default();
    /// ```
    pub fn default() -> Config {
        Config {
            bind_address: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0),
            timeout: Duration::from_secs(3),
        }
    }

    /// Set UDP bind address
    ///
    /// Sets the local address which is used to send/receive UDP packets. By default it is
    /// "0.0.0.0:0" which means that an IPv4 address and a port is chosen automatically.
    ///
    /// To synchronize with IPv6 servers, you might need to set it to an IPv6 address.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rsntp::{Config, SntpClient};
    ///
    /// let config = Config::default().bind_address("192.168.0.1:0".parse().unwrap());
    /// let client = SntpClient::with_config(config);
    /// ```
    pub fn bind_address(self, address: SocketAddr) -> Config {
        Config {
            bind_address: address,
            timeout: self.timeout,
        }
    }

    /// Sets synchronization timeout
    ///
    /// Sets the amount of time which the client waits for reply after the request has been sent.
    /// Default is 3 seconds.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rsntp::{Config, SntpClient};
    /// use std::time::Duration;
    ///
    /// let config = Config::default().timeout(Duration::from_secs(10));
    /// let client = SntpClient::with_config(config);
    /// ```
    pub fn timeout(self, timeout: Duration) -> Config {
        Config {
            bind_address: self.bind_address,
            timeout,
        }
    }
}

/// Blocking client instance
///
/// This is the main entry point of the blocking API.
#[derive(Clone, Debug, Hash)]
pub struct SntpClient {
    config: Config,
}

impl SntpClient {
    /// Creates a new instance with default configuration
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rsntp::SntpClient;
    ///
    /// let client = SntpClient::new();
    /// ```
    pub fn new() -> SntpClient {
        SntpClient {
            config: Config::default(),
        }
    }

    /// Creates a new instance with a specific configuration
    /// # Example
    ///
    /// ```no_run
    /// use rsntp::{Config, SntpClient};
    ///
    /// let client = SntpClient::with_config(Config::default());
    /// ```
    pub fn with_config(config: Config) -> SntpClient {
        SntpClient { config }
    }

    /// Synchronize with the server
    ///
    /// Sends a request to the server, waits for the reply and processes it. This is a blocking call
    /// and can block for quite long time. After sending the request it waits for a timeout and if no
    /// reply is received then an error is returned.
    ///
    /// If the supplied server address resolves to multiple addresses then only the first one is used.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rsntp::SntpClient;
    ///
    /// let client = SntpClient::new();
    /// let result = client.synchronize("pool.ntp.org");
    /// ```
    pub fn synchronize<A: ToServerAddrs>(
        &self,
        server_address: A,
    ) -> Result<SynchronizationResult, SynchroniztationError> {
        let socket = std::net::UdpSocket::bind(self.config.bind_address)?;

        socket.set_read_timeout(Some(self.config.timeout))?;
        socket.connect(server_address.to_server_addrs(SNTP_PORT))?;

        let request = Request::new();
        let mut receive_buffer = [0; Packet::ENCODED_LEN];

        socket.send(&request.as_bytes())?;
        let (bytes_received, server_address) = socket.recv_from(&mut receive_buffer)?;

        let reply = Reply::new(
            request,
            Packet::from_bytes(&receive_buffer[..bytes_received], server_address)?,
        );

        reply.process()
    }

    /// Sets synchronization timeout
    ///
    /// This sets the amount of time which the client waits for reply after the request has been sent.
    /// Default is 3 seconds.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rsntp::SntpClient;
    /// use std::time::Duration;
    ///
    /// let mut client = SntpClient::new();
    /// client.set_timeout(Duration::from_secs(10));
    /// ```
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.config.timeout = timeout;
    }

    /// Set UDP bind address
    ///
    /// Sets the local address which is used to send/receive UDP packets. By default it is
    /// "0.0.0.0:0" which means that an IPv4 address and a port is chosen automatically.
    ///
    /// To synchronize with IPv6 servers, you might need to set it to an IPv6 address.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rsntp::SntpClient;
    ///
    /// let mut client = SntpClient::new();
    /// client.set_bind_address("192.168.0.1:0".parse().unwrap());
    /// ```
    pub fn set_bind_address(&mut self, address: SocketAddr) {
        self.config.bind_address = address;
    }

    /// Set the configuration
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rsntp::{Config, SntpClient};
    ///
    /// let client = SntpClient::new();
    /// let config = Config::default().bind_address("192.168.0.1:0".parse().unwrap());
    /// ```
    pub fn set_config(&mut self, config: Config) {
        self.config = config
    }
}

impl Default for SntpClient {
    fn default() -> Self {
        SntpClient::new()
    }
}

/// Asynchronous client instance
///
/// Only available when async feature is enabled (which is the default)
///
/// This is the main entry point of the asynchronous API.
#[cfg(feature = "async")]
pub struct AsyncSntpClient {
    config: Config,
}

#[cfg(feature = "async")]
impl AsyncSntpClient {
    /// Creates a new instance with default configuration
    ///
    /// Only available when async feature is enabled (which is the default)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rsntp::AsyncSntpClient;
    ///
    /// let client = AsyncSntpClient::new();
    /// ```
    pub fn new() -> AsyncSntpClient {
        AsyncSntpClient {
            config: Config::default(),
        }
    }

    /// Creates a new instance with a specific configuration
    ///
    /// Only available when async feature is enabled (which is the default)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rsntp::{Config, AsyncSntpClient};
    ///
    /// let client = AsyncSntpClient::with_config(Config::default());
    /// ```
    pub fn with_config(config: Config) -> AsyncSntpClient {
        AsyncSntpClient { config }
    }

    /// Synchronize with the server
    ///
    /// Only available when async feature is enabled (which is the default)
    ///
    /// Sends a request to the server and processes the reply. If no reply is received within timeout
    /// then an error is returned. If the supplied server address resolves to multiple addresses then
    /// only the first one is used.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rsntp::{AsyncSntpClient, SynchronizationResult, SynchroniztationError};
    ///
    /// async fn local_time() -> Result<SynchronizationResult, SynchroniztationError> {
    ///   let client = AsyncSntpClient::new();
    ///   
    ///   client.synchronize("pool.ntp.org").await
    /// }
    /// ```
    pub async fn synchronize<A: ToServerAddrs>(
        &self,
        server_address: A,
    ) -> Result<SynchronizationResult, SynchroniztationError> {
        let mut receive_buffer = [0; Packet::ENCODED_LEN];

        let socket = tokio::net::UdpSocket::bind(self.config.bind_address).await?;
        socket
            .connect(server_address.to_server_addrs(SNTP_PORT))
            .await?;
        let request = Request::new();

        socket.send(&request.as_bytes()).await?;

        let result_future = timeout(self.config.timeout, socket.recv_from(&mut receive_buffer));

        let (bytes_received, server_address) = result_future.await.map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "Timeout while waiting for server reply",
            )
        })??;

        let reply = Reply::new(
            request,
            Packet::from_bytes(&receive_buffer[..bytes_received], server_address)?,
        );

        reply.process()
    }

    /// Sets synchronization timeout
    ///
    /// Sets the amount of time which the client waits for reply after the request has been sent.
    /// Default is 3 seconds.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rsntp::AsyncSntpClient;
    /// use std::time::Duration;
    ///
    /// let mut client = AsyncSntpClient::new();
    /// client.set_timeout(Duration::from_secs(10));
    /// ```
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.config.timeout = timeout;
    }

    /// Set UDP bind address
    ///
    /// Sets the local address which is used to send/receive UDP packets. By default it is
    /// "0.0.0.0:0" which means that an IPv4 address and a port is chosen automatically.
    ///
    /// To synchronize with IPv6 servers, you might need to set it to an IPv6 address.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rsntp::AsyncSntpClient;
    ///
    /// let mut client = AsyncSntpClient::new();
    /// client.set_bind_address("192.168.0.1:0".parse().unwrap());
    /// ```
    pub fn set_bind_address(&mut self, address: SocketAddr) {
        self.config.bind_address = address;
    }

    /// Set the configuration
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rsntp::{Config, AsyncSntpClient};
    ///
    /// let client = AsyncSntpClient::new();
    /// let config = Config::default().bind_address("192.168.0.1:0".parse().unwrap());
    /// ```
    pub fn set_config(&mut self, config: Config) {
        self.config = config
    }
}

#[cfg(feature = "async")]
impl Default for AsyncSntpClient {
    fn default() -> Self {
        AsyncSntpClient::new()
    }
}
