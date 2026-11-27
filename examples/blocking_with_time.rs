#[cfg(feature = "time")]
use time::{Duration, OffsetDateTime};

#[cfg(feature = "time")]
fn time_example() {
    let client = rsntp::SntpClient::new();
    let time_info = client.synchronize("pool.ntp.org").unwrap();

    let clock_offset: Duration = time_info.clock_offset().try_into().unwrap();
    println!("Clock offset: {clock_offset}");

    let round_trip_delay: Duration = time_info.round_trip_delay().try_into().unwrap();
    println!("Round trip delay: {round_trip_delay}");

    let datetime_utc: OffsetDateTime = time_info.datetime().try_into().unwrap();
    println!("UTC time: {datetime_utc}");

    println!("Reference identifier: {}", time_info.reference_identifier());
    println!("Stratum: {}", time_info.stratum());
}

fn main() {
    #[cfg(feature = "time")]
    time_example();
}
