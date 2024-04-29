use monza_full_node::{
    MonzaFullNode,
    partial::MonzaPartialFullNode,
};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {

    let executor = MonzaPartialFullNode::try_from_env().await?;

    executor.run().await?;

    Ok(())

}
