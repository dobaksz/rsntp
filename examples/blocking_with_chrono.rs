#[cfg(feature = "chrono")]
use chrono::{DateTime, Duration, Local, Utc};

#[cfg(feature = "chrono")]
fn chrono_example() {
    let client = rsntp::SntpClient::new();
    let time_info = client.synchronize("pool.ntp.org").unwrap();

    let clock_offset: Duration = time_info.clock_offset().try_into().unwrap();
    println!("Clock offset: {} ms", clock_offset);

    let round_trip_delay: Duration = time_info.clock_offset().try_into().unwrap();
    println!("Round trip delay: {} ms", round_trip_delay);

    let datetime_utc: DateTime<Utc> = time_info.datetime().try_into().unwrap();
    let local_time: DateTime<Local> = DateTime::from(datetime_utc);
    println!("Local time: {}", local_time);

    println!(
        "Reference identifier: {}",
        time_info.reference_identifier().to_string()
    );
    println!("Stratum: {}", time_info.stratum());
}

fn main() {
    #[cfg(feature = "chrono")]
    chrono_example();
}
