fn main() {
    let client = rsntp::SntpClient::new();
    let time_info = client.synchronize("pool.ntp.org").unwrap();

    println!(
        "Clock offset: {} ms",
        time_info.clock_offset().as_secs_f64() * 1000.0
    );
    println!(
        "Round trip delay: {} ms",
        time_info.round_trip_delay().as_secs_f64() * 1000.0
    );
    println!(
        "Server UTC UNIX timestamp: {}",
        time_info.datetime().unix_timestamp().unwrap().as_secs()
    );

    println!(
        "Reference identifier: {}",
        time_info.reference_identifier().to_string()
    );
    println!("Stratum: {}", time_info.stratum());
}
