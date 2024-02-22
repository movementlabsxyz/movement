#[cfg(test)]
mod tests;

#[must_use]
pub fn get_network_runner_grpc_endpoint() -> (String, bool) {
    match std::env::var("NETWORK_RUNNER_GRPC_ENDPOINT") {
        Ok(s) => (s, true),
        _ => (String::new(), false),
    }
}

#[must_use]
pub fn get_network_runner_enable_shutdown() -> bool {
    matches!(std::env::var("NETWORK_RUNNER_ENABLE_SHUTDOWN"), Ok(_))
}

#[must_use]
pub fn get_avalanchego_path() -> (String, bool) {
    match std::env::var("AVALANCHEGO_PATH") {
        Ok(s) => (s, true),
        _ => (String::new(), false),
    }
}

#[must_use]
pub fn get_vm_plugin_path() -> (String, bool) {
    match std::env::var("VM_PLUGIN_PATH") {
        Ok(s) => (s, true),
        _ => (String::new(), false),
    }
}
