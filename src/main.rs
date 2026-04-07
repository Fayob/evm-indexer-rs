use evm_indexer::{config::Config, error, fetcher::block_fetcher::BlockFetcher, rpc::client::RpcClient, storage};

#[tokio::main]
async fn main() -> error::Result<()> {
    dotenvy::dotenv().ok();

    let config = Config::from_env()?;

    let client = RpcClient::new(config.rpc_url.clone());

    let block_number = client.get_block_number().await?;
    println!("Latest block number: {block_number}");

    let pool = storage::db::create_pool(&config.database_url).await?;
    storage::db::run_migration(&pool).await?;

    println!("Chain tip: {block_number}");

    let fetcher = BlockFetcher::new(client, pool, config);

    fetcher.run().await?;

    Ok(())
}
