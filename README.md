[![](https://img.shields.io/crates/v/rsntp)](https://crates.io/crates/rsntp)
[![Rust](https://github.com/dobaksz/rsntp/actions/workflows/rust.yml/badge.svg?branch=master)](https://github.com/dobaksz/rsntp/actions/workflows/rust.yml)

# rsntp

An [RFC 5905](https://www.rfc-editor.org/rfc/rfc5905.txt) compliant Simple Network Time Protocol (SNTP) client
library for Rust.

`rsntp` provides an API to synchronize time with SNTPv4 time servers with the following features:
* Provides both a synchronous (blocking) and an (optional) asynchronous, `tokio` based API
* Optional support for the `time` crate
* IPv6 support

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
rsntp = "4.1.2"
```

Obtain the current time with the blocking API:

```rust
use rsntp::SntpClient;
use std::time::SystemTime;

let client = SntpClient::new();
let result = client.synchronize("pool.ntp.org").unwrap();

let now: SystemTime = result.datetime().into_system_time().unwrap();

println!("Current time is: {:?}", now);
```

You can also use the asynchronous API to do the same:

```rust
use rsntp::AsyncSntpClient;
use std::time::SystemTime;

async fn current_time() -> SystemTime {
  let client = AsyncSntpClient::new();
  let result = client.synchronize("pool.ntp.org").await.unwrap();

  result.datetime().into_system_time().unwrap()
}
```

## API changes in version 5.0

Support for the `chrono` crate was removed, because it is unmaintained. The `chrono` feature,
`SntpDateTime::into_chrono_datetime()`, `SntpDuration::into_chrono_duration()` and the
corresponding `TryInto` implementations are gone. If you still need `chrono` types, convert
through `std::time::SystemTime`, which `chrono` supports out of the box:

```text
let datetime: chrono::DateTime<chrono::Utc> = result.datetime().into_system_time()?.into();
let clock_offset = chrono::Duration::milliseconds((result.clock_offset().as_secs_f64() * 1000.0) as i64);
```

## Support for the `time` crate

`SynchronizationResult` returns time and date data in crate-independent wrapper types
(`SntpDateTime` and `SntpDuration`), which can be converted to `std::time` types. Conversion to
the types of the `time` crate can be enabled with the `time` feature:

```toml
[dependencies]
rsntp = { version = "4.1.2", features = ["time"] }
```

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

## Disabling asynchronous API

The asynchronous API is enabled by default, but you can disable it. Disabling it
has the advantage that it removes the dependency to `tokio`, which reduces
the amount of dependencies significantly.

```toml
[dependencies]
rsntp = { version = "4.1.2", default-features = false }
```

## System suspend

`SynchronizationResult::datetime()` calculates the current time by adding the time elapsed since
synchronization to the synchronized time. The elapsed time is measured with the steady clock
(`std::time::Instant`), so changes to the system clock do not affect it. However, on some
platforms (e.g. Linux and macOS) the steady clock does not advance while the system is suspended.
If the system was suspended after synchronization, `datetime()` falls behind by the duration of
the suspension, so synchronize again after the system resumes.

The same applies if the system is suspended during synchronization: if the server reply arrives
before the suspension but is only read after resuming, the synchronization succeeds with a small
round trip delay, but `datetime()` is already behind by the duration of the suspension.

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
