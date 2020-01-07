# rsntp

An [RFC 4330](https://tools.ietf.org/html/rfc4330) compliant Simple Network Time Protocol (SNTP) client
library for Rust.

`rsntp` provides both a synchronous (blocking) and an (optional) asynchronous API which allows
synchronization with SNTPv4 servers. Time and date handling is based on the `chrono` crate.


## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
rsntp = "0.2"
```

Obtain the current local time with the blocking API:

```rust
use rsntp::SntpClient;
use chrono::{DateTime, Local};

let client = SntpClient::new("pool.ntp.org").unwrap();
let result = client.synchronize().unwrap();

let local_time: DateTime<Local> = DateTime::from(result.datetime());
println!("Current time is: {}", local_time);
```

And the same with the asynchronous API:

```rust
use rsntp::AsyncSntpClient;
use chrono::{DateTime, Local};

async fn local_time() -> DateTime<Local> {
  let client = AsyncSntpClient::new("pool.ntp.org");
  let result = client.synchronize().await.unwrap();

  DateTime::from(result.datetime())
}
```

## Disabling asynchronous API

The asynchronous API is compiled in by default but you can optionally disable it. This removes
dependency to `tokio` which reduces crate dependencies significantly.

```toml
[dependencies]
rsntp = { version = "0.2", default-features = false }
```