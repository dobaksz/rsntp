[![](https://img.shields.io/crates/v/rsntp)](https://crates.io/crates/rsntp)
[![Build Status](https://app.travis-ci.com/dobaksz/rsntp.svg?branch=master)](https://app.travis-ci.com/dobaksz/rsntp)

# rsntp

An [RFC 5905](https://www.rfc-editor.org/rfc/rfc5905.txt) compliant Simple Network Time Protocol (SNTP) client
library for Rust.

`rsntp` provides an API to synchronize time with SNTPv4 time servers with the following features:
* Provides both a synchronous (blocking) and an (optional) asynchronous API based on `tokio`
* Optional support for time and date crates `chrono` and `time` (`chrono` is enabled by
  default)
* IPv6 support

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
rsntp = "3.0.0"
```

Obtain the current local time with the blocking API:

```rust
use rsntp::SntpClient;
use chrono::{DateTime, Local};

let client = SntpClient::new();
let result = client.synchronize("pool.ntp.org").unwrap();

let local_time: DateTime<Local> =
  DateTime::from(result.datetime().into_chrono_datetime().unwrap());

println!("Current time is: {}", local_time);
```

A function which uses the asynchronous API to obtain local time:

```rust
use rsntp::AsyncSntpClient;
use chrono::{DateTime, Local, Utc};

async fn local_time() -> DateTime<Local> {
  let client = AsyncSntpClient::new();
  let result = client.synchronize("pool.ntp.org").await.unwrap();
   
   DateTime::from(result.datetime().into_chrono_datetime().unwrap())
}
```

## API changes in version 3.0

Version 3.0 made core code independent of `chrono` crate and added support for `time` crate.
This led to some breaking API changes, `SynchronizationResult` methods will return with wrappers
struct instead of `chrono` ones. Those wrapper structs has `TryInto` implementation and helper
methods to convert them to `chrono` format.

To convert old code, replace
```rust
let datetime = result.datetime();
```
with
```rust
let datetime = result.datetime().into_chrono_datetime().unwrap();
```
or with
```rust
let datetime: chrono::DateTime<Utc> = result.datetime().try_into().unwrap();
```

The same applies to `Duration`s returned by `SynchronizationResult`.

## Support for time and date crates

`rsntp` supports returning time data in multiple different formats. By default, `chrono`
support is enabled, see examples above to use it. You can also use `time` crate support, if
you enable `time` feature:

```rust
use rsntp::SntpClient;

let client = SntpClient::new();
let result = client.synchronize("pool.ntp.org").unwrap();

let utc_time = result
  .datetime()
  .into_offset_date_time()
  .unwrap();

println!("UTC time is: {}", utc_time);
```

Support for both crates can be disabled or both can be enabled at the same time.

## Disabling asynchronous API

The asynchronous API is enabled by default but you can optionally disable it. This removes
dependency to `tokio` which reduces crate dependencies significantly.

```toml
[dependencies]
rsntp = { version = "3.0.0", default-features = false, features = ["chrono"] }
```

## System clock assumptions

`rsntp` assumes that system clock is monotonic and stable. This is especially important
with the `SynchronizationResult::datetime()` method, as `SynchronizationResult` stores just
an offset to the system clock. If the system clock is changed between synchronization
and the call to this method, then offset will not be valid anymore and some undefined result
will be returned.

## IPv6 support

`rsntp` supports IPv6, but by default (for compatilibty reasons) it binds its UDP socket to an
IPv4 address (0.0.0.0) which might prevent synchronization with IPv6 servers.

To use IPv6, you need to set an IPv6 bind address:

```rust
use rsntp::{Config, SntpClient};
use std::net::Ipv6Addr;

let config = Config::default().bind_address((Ipv6Addr::UNSPECIFIED, 0).into());
let client = SntpClient::with_config(config);

let result = client.synchronize("2.pool.ntp.org").unwrap();

let unix_timestamp_utc = result.datetime().unix_timestamp();
```
