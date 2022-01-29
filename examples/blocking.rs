fn main() {
    let client = rsntp::SntpClient::new();
    let time_info = client.synchronize("pool.ntp.org").unwrap();

    #[cfg(feature = "chrono")]
    let clock_offset = time_info.clock_offset().num_milliseconds();
    #[cfg(feature = "time")]
    let clock_offset = time_info.clock_offset().whole_milliseconds();
    println!("Clock offset: {} ms", clock_offset);

    #[cfg(feature = "chrono")]
    let round_trip_delay = time_info.round_trip_delay().num_milliseconds();
    #[cfg(feature = "time")]
    let round_trip_delay = time_info.round_trip_delay().whole_milliseconds();
    println!("Round trip delay: {} ms", round_trip_delay);
    println!("Server timestamp: {}", time_info.datetime());

    println!("Reference identifier: {}", time_info.reference_identifier());
    println!("Stratum: {}", time_info.stratum());
}
