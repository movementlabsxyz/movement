use anyhow::Result;
use eth_secure_sign::aws::AwsKms;
use eth_secure_sign::hashivault::Vault;
use eth_secure_sign::ActionStream;
use eth_secure_sign::Application;
use eth_secure_sign::Bytes;
use eth_secure_sign::Hsm;
use eth_secure_sign::Message;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    use tracing_subscriber::EnvFilter;

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let key_name = "secsign3";

    // Use AWS KMS
    let key_id = env::var("AWS_KEY_ID").expect("AWS_KEY_ID not set");
    let access_key = env::var("AWS_ACCESS_KEY").expect("AWS_ACCESS_KEY not set");
    let secret_key = env::var("AWS_SECRET_KEY").expect("AWS_SECRET_KEY not set");
    let aws = AwsKms::new(&key_id, &access_key, &secret_key);
    run_scenario(Box::new(aws)).await?;

    // Use Hashicorp Vault
    let vault_addr = env::var("VAULT_ADDR").expect("VAULT_ADDR not set");
    let token = env::var("VAULT_TOKEN").expect("VAULT_TOKEN not set");
    let namespace = env::var("VAULT_NAMESPACE").ok();
    let vault = Vault::new(&vault_addr, &token, key_name.to_string(), namespace).await?;
    run_scenario(Box::new(vault)).await?;

    Ok(())
}

struct Scenario {
    messages: Vec<Message>,
}
#[async_trait::async_trait]
impl ActionStream for Scenario {
    async fn next(&mut self) -> Option<Message> {
        (self.messages.len() != 0).then(|| {
            self.messages.remove(0)
        })
        
    }
}

async fn run_scenario(hsm: Box<dyn Hsm>) -> Result<()> {
        // Build scenario
    let tosign =Bytes(b"message 1".to_vec());
    let sign = hsm.sign(tosign.clone()).await?;
    let scenario = Scenario {
        messages: vec![ Message::Sign(tosign.clone()),  Message::Verify(tosign, sign)],
    };

    let mut app = Application {
        hsm,
        stream: Box::new(scenario),
    };

    app.run().await?;
    Ok(())

}


