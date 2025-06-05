use aptos_logger::info;

#[tokio::main]
async fn main() {
    // Initialize logger (for debugging)
    aptos_logger::Logger::builder()
        .level(aptos_logger::Level::Debug)
        .build();

    // Initialize telemetry
    movement_tracing::ensure_telemetry_initialized();

    let endpoint = movement_tracing::get_metrics_endpoint();
    info!("Starting telemetry server at {}", endpoint);

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
} 