use evm_indexer::{config::Config, error, rpc::client::RpcClient};

#[tokio::main]
async fn main() -> error::Result<()> {
    dotenvy::dotenv().ok();

    let config = Config::from_env()?;

    let client = RpcClient::new(config.rpc_url);

    let block_number = client.get_block_number().await?;
    println!("Latest block number: {block_number}");

    let block = client.get_block_by_number(block_number).await?;
    match block {
        Some(b) => println!(
            "Block #{}: hash={}, txs={}",
            b.number,
            b.hash,
            b.transactions.len()
        ),
        None => println!("Block not found"),
    }

    Ok(())
}
