//! # rsntp
//!
//! An [RFC 4330](https://tools.ietf.org/html/rfc4330) compliant Simple Network Time Protocol (SNTP) client
//! library for Rust.
//!
//! `rsntp` provides a simple synchronous (blocking) API which allows synchronization with SNTPv4 servers
//! and uses data types from the `chrono` crate for convenience.
//!
//!
//! ## Usage
//!
//! Add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! rsntp = "0.1"
//! ```
//!
//! After that you can query the current local time with the following code:
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
//! println!("Current time is: {}", local_time)
//! ```
mod core_logic;
mod error;
mod packet;

pub use core_logic::{Reply, Request, SynchronizationResult};
pub use error::{KissCode, ProtocolError, SynchroniztationError};
pub use packet::{LeapIndicator, Packet, ReferenceIdentifier};

use std::io::ErrorKind;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::time::Duration;

/// SNTP client instance
///
/// This is the main entry point of the API. It is an association between the local host and the remote server
/// and can be reused, i.e. multiple synchronization can be executed with a single instance.
#[derive(Clone, Debug, Hash)]
pub struct SntpClient {
  server_address: SocketAddr,
}

impl SntpClient {
  const SNTP_PORT: u16 = 123;

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
    SntpClient::with_socket_addr((server_address, SntpClient::SNTP_PORT))
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
    let socket = UdpSocket::bind("0.0.0.0:0")?;

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
