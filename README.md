# rsntp

An [RFC 4330](https://tools.ietf.org/html/rfc4330) compliant Simple Network Time Protocol (SNTP) client
library for Rust.

`rsntp` provides a simple synchronous (blocking) API which allows synchronization with SNTPv4 servers
and uses data types from the `chrono` crate for convenience.

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
rsntp = "0.1"
```

After that you can query the current local time with the following code:

```rust
use rsntp::SntpClient;
use chrono::{DateTime, Local};

let client = SntpClient::new("pool.ntp.org").unwrap();
let result = client.synchronize().unwrap();

let local_time: DateTime<Local> = DateTime::from(result.datetime());

println!("Current time is: {}", local_time)
```