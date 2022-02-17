#[cfg(feature = "chrono")]
fn chrono_example() {
    let client = rsntp::SntpClient::new();
    let time_info = client.synchronize("pool.ntp.org").unwrap();

    println!(
        "Clock offset: {} ms",
        time_info
            .clock_offset()
            .as_chrono_duration()
            .num_milliseconds()
    );
    println!(
        "Round trip delay: {} ms",
        time_info
            .round_trip_delay()
            .as_chrono_duration()
            .num_milliseconds()
    );
    println!(
        "Server timestamp: {}",
        time_info.datetime().as_chrono_datetime_utc().to_string()
    );

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
