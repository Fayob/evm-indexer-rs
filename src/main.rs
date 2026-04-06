use evm_indexer::{config::Config, error, rpc::client::RpcClient, storage};

#[tokio::main]
async fn main() -> error::Result<()> {
    dotenvy::dotenv().ok();

    let config = Config::from_env()?;

    let client = RpcClient::new(config.rpc_url);

    let block_number = client.get_block_number().await?;
    println!("Latest block number: {block_number}");

    let pool = storage::db::create_pool(&config.database_url).await?;
    storage::db::run_migration(&pool).await?;

    println!("Chain tip: {block_number}");

    let last = storage::db::get_last_indexed_block(&pool).await?;
    match last {
        Some(n) => println!("Resuming from block {n}"),
        None => println!("Fresh start"),
    }

    Ok(())
}
