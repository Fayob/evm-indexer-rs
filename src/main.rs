use evm_indexer::{config::Config, error};

fn main() -> error::Result<()> {
   let config = Config::from_env()?;
    println!("RPC URL: {}", config.rpc_url);

    Ok(())
}
