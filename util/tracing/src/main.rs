use aptos_logger::info;

#[tokio::main]
async fn main() {
    // Initialize logger for better debugging
    aptos_logger::Logger::builder()
        .level(aptos_logger::Level::Debug)
        .build();

    // Initialize telemetry
    movement_tracing::ensure_telemetry_initialized();

    // Get and print the metrics endpoint
    let endpoint = movement_tracing::get_metrics_endpoint();
    info!("Starting telemetry server at {}", endpoint);
    
    // Keep the process running
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
} 