[![](https://img.shields.io/crates/v/rsntp)](https://crates.io/crates/rsntp)
[![Build Status](https://travis-ci.com/dobaksz/rsntp.svg?branch=master)](https://travis-ci.com/dobaksz/rsntp)

# rsntp

An [RFC 4330](https://tools.ietf.org/html/rfc4330) compliant Simple Network Time Protocol (SNTP) client
library for Rust.

`rsntp` provides an API to synchronize time with SNTPv4 time servers with the following features:

* Provides both a synchronous (blocking) and an (optional) asynchronous API based `tokio`
* Time and date handling based either on the `chrono` or `time` crates
* IPv6 support


## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
rsntp = "2.1.0"
```

Obtain the current local time with the blocking API:

```rust
use rsntp::SntpClient;
use chrono::{DateTime, Local};

let client = SntpClient::new();
let result = client.synchronize("pool.ntp.org").unwrap();

let local_time: DateTime<Local> = DateTime::from(result.datetime());

println!("Current time is: {}", local_time);
```

A function which uses the asynchronous API to obtain local time:

```rust
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
rsntp = { version = "2.1.0", default-features = false }
```

## IPv6 support

`rsntp` supports IPv6, but by default (for compatilibty reasons) it binds its UDP socket to an
IPv4 address (0.0.0.0) which might prevent synchronization with IPv6 servers.

To use IPv6, you need to set an IPv6 bind address:

```rust
use chrono::{DateTime, Local};
use rsntp::{Config, SntpClient};
use std::net::Ipv6Addr;

let config = Config::default().bind_address((Ipv6Addr::UNSPECIFIED, 0).into());
let client = SntpClient::with_config(config);

let result = client.synchronize("2.pool.ntp.org").unwrap();

let local_time: DateTime<Local> = DateTime::from(result.datetime());
```
