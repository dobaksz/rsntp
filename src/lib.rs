#![cfg_attr(
    all(feature = "async", feature = "chrono", feature = "time"),
    doc = r##"
# rsntp

An [RFC 5905](https://www.rfc-editor.org/rfc/rfc5905.txt) compliant Simple Network Time Protocol (SNTP) client
library for Rust.

`rsntp` provides an API to synchronize time with SNTPv4 time servers with the following features:
* Provides both a synchronous (blocking) and an (optional) asynchronous, `tokio` based API
* Optional support for time and date crates `chrono` and `time` (`chrono` is enabled by
  default)
* IPv6 support

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
rsntp = "4.0.0"
```

Obtain the current local time with the blocking API:

```no_run
use rsntp::SntpClient;
use chrono::{DateTime, Local};

let client = SntpClient::new();
let result = client.synchronize("pool.ntp.org").unwrap();

let local_time: DateTime<Local> =
  DateTime::from(result.datetime().into_chrono_datetime().unwrap());

println!("Current time is: {}", local_time);
```

You can also use the asynchronous API to do the same:

```no_run
use rsntp::AsyncSntpClient;
use chrono::{DateTime, Local, Utc};

async fn local_time() -> DateTime<Local> {
  let client = AsyncSntpClient::new();
  let result = client.synchronize("pool.ntp.org").await.unwrap();
   
   DateTime::from(result.datetime().into_chrono_datetime().unwrap())
}
```

## API changes in version 3.0

Version 3.0 made core code independent of time and date crates and added support for the `time` crate.
This led to some breaking API changes, `SynchronizationResult` methods will return with wrappers
struct instead of `chrono` ones. Those wrapper structs has `TryInto` implementation and helper
methods to convert them to `chrono` format.

To convert old code, replace
```no_run
# use rsntp::SntpClient;
# let result = SntpClient::new().synchronize("pool.ntp.org").unwrap();
let datetime = result.datetime();
```
with
```no_run
# use rsntp::SntpClient;
# use chrono::{DateTime, Utc};
# let result = SntpClient::new().synchronize("pool.ntp.org").unwrap();
let datetime = result.datetime().into_chrono_datetime().unwrap();
```
or with
```no_run
# use rsntp::SntpClient;
# use chrono::{DateTime, Utc};
# let result = SntpClient::new().synchronize("pool.ntp.org").unwrap();
let datetime: chrono::DateTime<Utc> = result.datetime().try_into().unwrap();
```

The same applies to `Duration`s returned by `SynchronizationResult`.

## Support for time and date crates

`rsntp` supports returning time and date data in different formats. Currently the format of
the two most popular time and date handling crates supported: `chrono` and `time`.
By default, `chrono` is enabled, but you can add `time` support with a feature:

```no_run
use rsntp::SntpClient;

let client = SntpClient::new();
let result = client.synchronize("pool.ntp.org").unwrap();

let utc_time = result
  .datetime()
  .into_offset_date_time()
  .unwrap();

println!("UTC time is: {}", utc_time);
```

Support for both crates can be enabled independently; you can even enable both
at the same time.

## Disabling asynchronous API

The asynchronous API is enabled by default, but you can disable it. Disabling it 
has the advantage that it removes the dependency to `tokio`, which reduces 
the amount of dependencies significantly.

```toml
[dependencies]
rsntp = { version = "4.0.0", default-features = false, features = ["chrono"]  }
```

## System clock assumptions

`rsntp` assumes that system clock is monotonic and stable. This is especially important
with the `SynchronizationResult::datetime()` method, as `SynchronizationResult` stores just
an offset to the system clock. If the system clock is changed between synchronization
and the call to this method, then offset will not be valid anymore and some undefined result
might be returned.

## IPv6 support

`rsntp` supports IPv6, but for compatibility reasons, it binds its UDP socket to an
IPv4 address (0.0.0.0) by default. That might prevent synchronization with IPv6 servers.

To use IPv6, you need to set an IPv6 bind address:

```no_run
use rsntp::{Config, SntpClient};
use std::net::Ipv6Addr;

let config = Config::default().bind_address((Ipv6Addr::UNSPECIFIED, 0).into());
let client = SntpClient::with_config(config);

let result = client.synchronize("2.pool.ntp.org").unwrap();

let unix_timestamp_utc = result.datetime().unix_timestamp();
```
"##
)]

mod core_logic;
mod error;
mod packet;
mod result;
mod to_server_addrs;

pub use error::{ConversionError, KissCode, ProtocolError, SynchronizationError};
pub use packet::{LeapIndicator, ReferenceIdentifier};
pub use result::{SntpDateTime, SntpDuration, SynchronizationResult};
pub use to_server_addrs::ToServerAddrs;

use core_logic::{Reply, Request};
use packet::Packet;
use std::default::Default;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

#[cfg(feature = "async")]
use tokio::time::timeout;

const SNTP_PORT: u16 = 123;

/// Client configuration
///
/// This is a struct that contains the configuration of a client. It uses a builder-like pattern
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
    /// Set UDP bind address
    ///
    /// Sets the local address which is used to send/receive UDP packets. By default, it is
    /// "0.0.0.0:0" which means that IPv4 address and port are chosen automatically.
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
    /// Sets the time the client waits for a reply after the request has been sent.
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

impl Default for Config {
    /// Creates an instance with default configuration
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rsntp::Config;
    ///
    /// let config = Config::default();
    /// ```
    fn default() -> Config {
        Config {
            bind_address: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0),
            timeout: Duration::from_secs(3),
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

    /// Creates a new instance with the specified configuration
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
    /// Sends a request to the server, waits for the reply, and processes it. This is a blocking call
    /// and can block for a long time. After sending the request, it waits for a timeout; if no
    /// reply is received, an error is returned.
    ///
    /// If the supplied server address resolves to multiple addresses, only the first one is used.
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
    ) -> Result<SynchronizationResult, SynchronizationError> {
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
    /// Sets the time the client waits for a reply after the request has been sent.
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
    /// Sets the local address which is used to send/receive UDP packets. By default, it is
    /// "0.0.0.0:0" which means that IPv4 address and port are chosen automatically.
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

    /// Creates a new instance with the specified configuration
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
    /// Sends a request to the server and processes the reply. If no reply is received within timeout,
    /// then an error is returned. If the supplied server address resolves to multiple addresses,
    /// only the first one is used.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rsntp::{AsyncSntpClient, SynchronizationResult, SynchronizationError};
    ///
    /// async fn local_time() -> Result<SynchronizationResult, SynchronizationError> {
    ///   let client = AsyncSntpClient::new();
    ///   
    ///   client.synchronize("pool.ntp.org").await
    /// }
    /// ```
    pub async fn synchronize<A: ToServerAddrs>(
        &self,
        server_address: A,
    ) -> Result<SynchronizationResult, SynchronizationError> {
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

    pub async fn synchronize_with_reference_time<A: ToServerAddrs>(
        &self,
        server_address: A,
        reference_time: std::time::SystemTime,
    ) -> Result<SynchronizationResult, SynchronizationError> {
        let mut receive_buffer = [0; Packet::ENCODED_LEN];

        let socket = tokio::net::UdpSocket::bind(self.config.bind_address).await?;
        socket
            .connect(server_address.to_server_addrs(SNTP_PORT))
            .await?;
        let request = Request::new_with_transmit_time(reference_time);

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
    /// Sets the time which the client waits for a reply after the request has been sent.
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
    /// Sets the local address which is used to send/receive UDP packets. By default, it is
    /// "0.0.0.0:0" which means that IPv4 address and port are chosen automatically.
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
