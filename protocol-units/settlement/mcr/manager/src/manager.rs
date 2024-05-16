use mcr_settlement_client::{McrSettlementClient, /*McrSettlementClientOperations*/};

pub struct Manager {
    pub client: McrSettlementClient,
}

impl Manager {
    pub fn new(client: McrSettlementClient) -> Self {
        Self { client }
    }
}