#[tokio::main()]
async fn main() -> Result<(), Box<dyn std::error::Error + Sync + Send>> {
    let timestamp_ms: u64 = 1739718398362;
    let ntp_server_address = "pool.ntp.org";
    let async_sntp_client = rsntp::AsyncSntpClient::new();

    let ntp_result = async_sntp_client
        .synchronize_with_reference_time(
            &ntp_server_address,
            std::time::UNIX_EPOCH + std::time::Duration::from_millis(timestamp_ms),
        )
        .await?;
    let diff_ms = (ntp_result.clock_offset().as_secs_f64() * 1000.0).round() as i64;

    println!("diff_ms is {diff_ms}");

    Ok(())
}
