//! # rsntp
//!
//! An [RFC 4330](https://tools.ietf.org/html/rfc4330) compliant Simple Network Time Protocol (SNTP) client
//! library for Rust.
//!
//! `rsntp` provides both a synchronous (blocking) and an (optional) asynchronous API which allows
//! synchronization with SNTPv4 servers. Time and date handling is based on the `chrono` crate.
//!
//!
//! ## Usage
//!
//! Add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! rsntp = "0.2"
//! ```
//!
//! Obtain the current local time with the blocking API:
//!
//! ```no_run
//! use rsntp::SntpClient;
//! use chrono::{DateTime, Local};
//!
//! let client = SntpClient::new("pool.ntp.org").unwrap();
//! let result = client.synchronize().unwrap();
//!
//! let local_time: DateTime<Local> = DateTime::from(result.datetime());
//!
//! println!("Current time is: {}", local_time);
//! ```
//!
//! And the same with the asynchronous API:
//!
//! ```no_run
//! use rsntp::AsyncSntpClient;
//! use chrono::{DateTime, Local};
//!
//! async fn local_time() -> DateTime<Local> {
//!   let client = AsyncSntpClient::new("pool.ntp.org");
//!   let result = client.synchronize().await.unwrap();
//!
//!   DateTime::from(result.datetime())
//! }
//! ```
//!
//! ## Disabling asynchronous API
//!
//! The asynchronous API is compiled in by default but you can optionally disable it. This removes
//! dependency to `tokio` which reduces crate dependencies significantly.
//!
//! ```toml
//! [dependencies]
//! rsntp = { version = "0.2", default-features = false }
//! ```
mod core_logic;
mod error;
mod packet;

pub use core_logic::SynchronizationResult;
pub use error::{KissCode, ProtocolError, SynchroniztationError};
pub use packet::{LeapIndicator, ReferenceIdentifier};

use core_logic::{Reply, Request};
use packet::Packet;
use std::io::ErrorKind;
use std::net::{SocketAddr, ToSocketAddrs};
use std::time::Duration;

#[cfg(feature = "async")]
use tokio::time::timeout;

const SNTP_PORT: u16 = 123;

/// Blocking SNTP client instance
///
/// This is the main entry point of the blocking API. It is an association between the local host and the
/// remote server and can be reused, i.e. multiple synchronization can be executed with a single instance.
#[derive(Clone, Debug, Hash)]
pub struct SntpClient {
  server_address: SocketAddr,
}

impl SntpClient {
  /// Creates a new instance.
  ///
  /// The parameter is the server DNS name or IP addresss. It uses the default SNTP UDP port (123)
  ///
  /// # Example
  ///
  /// ```no_run
  /// use rsntp::SntpClient;
  ///
  /// let client = SntpClient::new("pool.ntp.org").unwrap();
  /// ```
  pub fn new(server_address: &str) -> Result<SntpClient, std::io::Error> {
    SntpClient::with_socket_addr((server_address, SNTP_PORT))
  }

  /// Creates a new instance for the given socket addres.
  /// # Example
  ///
  /// ```no_run
  /// use rsntp::SntpClient;
  ///
  /// let socket_addr = ("pool.ntp.org", 123);
  /// let client = SntpClient::with_socket_addr(socket_addr).unwrap();
  /// ```
  pub fn with_socket_addr<T: ToSocketAddrs>(socket_addr: T) -> Result<SntpClient, std::io::Error> {
    let server_address = socket_addr.to_socket_addrs()?.next().ok_or_else(|| {
      std::io::Error::new(
        ErrorKind::AddrNotAvailable,
        "Failed to resolve server address",
      )
    })?;

    Ok(SntpClient { server_address })
  }

  /// Synchronize with the server
  ///
  /// It sends a request to the server, waits for the reply and processes that reply. This is a blocking
  /// call and can block for quite long time (seconds). Default timeout is 3 seconds, if no reply is received in
  /// that timeframe then an error is returned. It tries to send the request just once, no retry in case of
  /// error.
  ///
  /// # Example
  ///
  /// ```no_run
  /// use rsntp::SntpClient;
  ///
  /// let socket_addr = ("pool.ntp.org", 123);
  /// let client = SntpClient::with_socket_addr(socket_addr).unwrap();
  /// let result = client.synchronize();
  /// ```
  pub fn synchronize(&self) -> Result<SynchronizationResult, SynchroniztationError> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0")?;

    socket.set_read_timeout(Some(Duration::from_secs(3)))?;
    socket.connect(self.server_address)?;

    let request = Request::new();
    let mut receive_buffer = [0; Packet::ENCODED_LEN];

    socket.send(&request.as_bytes())?;
    socket.recv(&mut receive_buffer)?;

    let reply = Reply::new(request, Packet::from_bytes(&receive_buffer)?);

    reply.process()
  }
}

/// Asynchronous API client instance
///
/// Only available when async feature is enabled (which is the default)
///
/// This is the main entry point of the asynchronous API. It is an association between the local host and the
/// remote server and can be reused, i.e. multiple synchronization can be executed with a single instance.
#[cfg(feature = "async")]
pub struct AsyncSntpClient {
  server_address: String,
}

#[cfg(feature = "async")]
impl AsyncSntpClient {
  /// Creates a new instance.
  ///
  /// Only available when async feature is enabled (which is the default)
  ///
  /// The parameter is the server DNS name or IP addresss. It uses the default SNTP UDP port (123)
  ///
  /// # Example
  ///
  /// ```no_run
  /// use rsntp::AsyncSntpClient;
  ///
  /// let client = AsyncSntpClient::new("pool.ntp.org");
  /// ```
  pub fn new(server_address: &str) -> AsyncSntpClient {
    AsyncSntpClient {
      server_address: server_address.into(),
    }
  }

  /// Synchronize with the server
  ///
  /// Only available when async feature is enabled (which is the default)
  ///
  /// It sends a request to the server and processes the reply. Default timeout is 3 seconds, if no reply is
  /// received in that timeframe then an error is returned. It tries to send the request just once, no retry
  /// in case of an error.
  ///
  /// # Example
  ///
  /// ```no_run
  /// use rsntp::{AsyncSntpClient, SynchronizationResult, SynchroniztationError};
  /// use chrono::{DateTime, Local};
  ///
  /// async fn local_time() -> Result<SynchronizationResult, SynchroniztationError> {
  ///   let client = AsyncSntpClient::new("pool.ntp.org");
  ///   
  ///   client.synchronize().await
  /// }
  /// ```
  pub async fn synchronize(&self) -> Result<SynchronizationResult, SynchroniztationError> {
    let mut receive_buffer = [0; Packet::ENCODED_LEN];
    let socket_address = (self.server_address.as_str(), SNTP_PORT);

    let mut socket = tokio::net::UdpSocket::bind("0.0.0.0:0").await?;
    socket.connect(socket_address).await?;
    let request = Request::new();

    socket.send(&request.as_bytes()).await?;

    let receive_result = timeout(Duration::from_secs(3), socket.recv(&mut receive_buffer)).await;

    if receive_result.is_err() {
      return Err(
        std::io::Error::new(
          std::io::ErrorKind::TimedOut,
          "Timeout while waiting for server reply",
        )
        .into(),
      );
    }

    let reply = Reply::new(request, Packet::from_bytes(&receive_buffer)?);

    reply.process()
  }
}
