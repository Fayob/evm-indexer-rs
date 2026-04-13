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

    // Register USDC for testing.
    // We'll move this to the API layer shortly.
    let usdc_abi = serde_json::json!([
        {
            "type": "event",
            "name": "Transfer",
            "inputs": [
                { "name": "from", "type": "address", "indexed": true },
                { "name": "to", "type": "address", "indexed": true },
                { "name": "value", "type": "uint256", "indexed": false }
            ]
        },
        {
            "type": "event",
            "name": "Approval",
            "inputs": [
                { "name": "owner", "type": "address", "indexed": true },
                { "name": "spender", "type": "address", "indexed": true },
                { "name": "value", "type": "uint256", "indexed": false }
            ]
        }
    ]);

    let usdc = storage::models::Contract {
        // USDC on Ethereum mainnet
        address: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_lowercase(),
        name: "USDC".to_string(),
        abi: usdc_abi,
    };

    storage::db::save_contract(&pool, &usdc).await?;
    println!("USDC contract registered");

    let fetcher = BlockFetcher::new(client, pool, config);

    fetcher.run().await?;

    Ok(())
}
