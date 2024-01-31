[![](https://img.shields.io/crates/v/rsntp)](https://crates.io/crates/rsntp)
[![CircleCI](https://dl.circleci.com/status-badge/img/gh/dobaksz/rsntp/tree/master.svg?style=shield)](https://dl.circleci.com/status-badge/redirect/gh/dobaksz/rsntp/tree/master)

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

```rust
use rsntp::SntpClient;
use chrono::{DateTime, Local};

let client = SntpClient::new();
let result = client.synchronize("pool.ntp.org").unwrap();

let local_time: DateTime<Local> =
  DateTime::from(result.datetime().into_chrono_datetime().unwrap());

println!("Current time is: {}", local_time);
```

You can also use the asynchronous API to do the same:

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

Version 3.0 made core code independent of time and date crates and added support for the `time` crate.
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

`rsntp` supports returning time and date data in different formats. Currently the format of
the two most popular time and date handling crates supported: `chrono` and `time`.
By default, `chrono` is enabled, but you can add `time` support with a feature:

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

Support for both crates can be enabled independently; you can even enable both
at the same time.

## Disabling asynchronous API

The asynchronous API is enabled by default, but you can disable it. Disabling it 
has the advantage that it removes the dependency to `tokio`, which reduces 
the amount of dependencies significantly.

```toml
[dependencies]
rsntp = { version = "4.0.0", default-features = false, features = ["chrono"] }
```

## System clock assumptions

`rsntp` assumes that system clock is monotonic and stable. This is especially important
with the `SynchronizationResult::datetime()` method, as `SynchronizationResult` stores just
an offset to the system clock. If the system clock is changed between synchronization
and the call to this method, then offset will not be valid anymore and some undefined result
will be returned.

## IPv6 support

`rsntp` supports IPv6, but for compatibility reasons, it binds its UDP socket to an
IPv4 address (0.0.0.0) by default. That might prevent synchronization with IPv6 servers.

To use IPv6, you need to set an IPv6 bind address:

```rust
use rsntp::{Config, SntpClient};
use std::net::Ipv6Addr;

let config = Config::default().bind_address((Ipv6Addr::UNSPECIFIED, 0).into());
let client = SntpClient::with_config(config);

let result = client.synchronize("2.pool.ntp.org").unwrap();

let unix_timestamp_utc = result.datetime().unix_timestamp();
```
