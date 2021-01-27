//! # rsntp
//!
//! An [RFC 4330](https://tools.ietf.org/html/rfc4330) compliant Simple Network Time Protocol (SNTP) client
//! library for Rust.
//!
//! `rsntp` provides an API to synchronize time with SNTPv4 time servers with the following features:
//!
//! * Provides both a synchronous (blocking) and an (optional) asynchronous API based `tokio`
//! * Time and date handling based on the `chrono` crate
//! * IPv6 support
//!
//! ## Usage
//!
//! Add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! rsntp = "2.0.0"
//! ```
//!
//! Obtain the current local time with the blocking API:
//!
//! ```no_run
//! use rsntp::SntpClient;
//! use chrono::{DateTime, Local};
//!
//! let client = SntpClient::new();
//! let result = client.synchronize("pool.ntp.org").unwrap();
//!
//! let local_time: DateTime<Local> = DateTime::from(result.datetime());
//!
//! println!("Current time is: {}", local_time);
//! ```
//!
#![cfg_attr(
    feature = "async",
    doc = r##"

A function which uses the asynchronous API to obtain local time:

```no_run
use rsntp::AsyncSntpClient;
use chrono::{DateTime, Local};

async fn local_time() -> DateTime<Local> {
  let client = AsyncSntpClient::new();
  let result = client.synchronize("pool.ntp.org").await.unwrap();
  
  DateTime::from(result.datetime())
}
```
## Disabling asynchronous API

The asynchronous API is enabled by default but you can optionally disable it. This removes
dependency to `tokio` which reduces crate dependencies significantly.

```toml
[dependencies]
rsntp = { version = "2.0.0", default-features = false }
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
//! use chrono::{DateTime, Local};
//! use rsntp::SntpClient;
//! use std::net::Ipv6Addr;
//!
//! let mut client = SntpClient::new();
//! client.set_bind_address((Ipv6Addr::UNSPECIFIED, 0).into());
//!
//! let result = client.synchronize("2.pool.ntp.org").unwrap();
//!
//! let local_time: DateTime<Local> = DateTime::from(result.datetime());
//! ```
//!

mod core_logic;
mod error;
mod packet;
mod to_server_addrs;

pub use core_logic::SynchronizationResult;
pub use error::{KissCode, ProtocolError, SynchroniztationError};
pub use packet::{LeapIndicator, ReferenceIdentifier};
pub use to_server_addrs::ToServerAddrs;

use core_logic::{Reply, Request};
use packet::Packet;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

#[cfg(feature = "async")]
use tokio::time::timeout;

const SNTP_PORT: u16 = 123;

/// Blocking client instance
///
/// This is the main entry point of the blocking API.
#[derive(Clone, Debug, Hash)]
pub struct SntpClient {
    bind_address: SocketAddr,
    timeout: Duration,
}

impl SntpClient {
    /// Creates a new instance with default parameters
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
            bind_address: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0),
            timeout: Duration::from_secs(3),
        }
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
        let socket = std::net::UdpSocket::bind(self.bind_address)?;

        socket.set_read_timeout(Some(self.timeout))?;
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
        self.timeout = timeout;
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
        self.bind_address = address;
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
    bind_address: SocketAddr,
    timeout: Duration,
}

#[cfg(feature = "async")]
impl AsyncSntpClient {
    /// Creates a new instance with default parameters
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
            bind_address: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0),
            timeout: Duration::from_secs(3),
        }
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
    /// use chrono::{DateTime, Local};
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

        let socket = tokio::net::UdpSocket::bind(self.bind_address).await?;
        socket
            .connect(server_address.to_server_addrs(SNTP_PORT))
            .await?;
        let request = Request::new();

        socket.send(&request.as_bytes()).await?;

        let result_future = timeout(self.timeout, socket.recv_from(&mut receive_buffer));

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
        self.timeout = timeout;
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
        self.bind_address = address;
    }
}

#[cfg(feature = "async")]
impl Default for AsyncSntpClient {
    fn default() -> Self {
        AsyncSntpClient::new()
    }
}
