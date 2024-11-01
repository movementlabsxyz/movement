// service/src/metrics.rs

use opentelemetry::metrics::{Counter, Meter};

pub struct Metrics {
        pub client_contract_calls: Counter<u64>,
        pub events_received: Counter<u64>,
        pub actions_taken: Counter<u64>,
}

impl Metrics {
        pub fn new(meter: &Meter) -> Self {
                Self {
                        client_contract_calls: meter
                                .u64_counter("client_contract_calls")
                                .with_description("Counts the number of client contract calls")
                                .init(),
                        events_received: meter
                                .u64_counter("events_received")
                                .with_description("Counts the number of events received")
                                .init(),
                        actions_taken: meter
                                .u64_counter("actions_taken")
                                .with_description("Counts the number of actions taken from events")
                                .init(),
                }
        }
}
