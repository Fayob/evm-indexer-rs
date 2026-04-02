use evm_indexer::{config::Config, error, rpc::client::RpcClient};

#[tokio::main]
async fn main() -> error::Result<()> {
    dotenvy::dotenv().ok();

    let config = Config::from_env()?;

    let client = RpcClient::new(config.rpc_url);

    let block_number = client
        .call("eth_blockNumber", serde_json::json!([]))
        .await?;
    println!("Latest block: {}", block_number);

    Ok(())
}
