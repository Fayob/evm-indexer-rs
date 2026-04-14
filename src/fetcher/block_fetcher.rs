use std::time::Duration;

use sqlx::PgPool;

use crate::{config::Config, decoder::log_decoder::EventRegistry, error::{IndexerError, Result}, rpc::{client::RpcClient, types::{Block, LogFilter}}, storage::db};

/// How often the fetcher pauses when it has caught up to the tip.
const POLL_INTERVAL_SECS: u64 = 12;

/// How many blocks between contract list reloads.
/// Every 10 blocks we re-read the contracts table so newly
/// registered contracts are picked up without a restart.
const CONTRACT_RELOAD_INTERVAL: u64 = 10;

pub struct BlockFetcher {
    client: RpcClient,
    pool: PgPool,
    config: Config
}

impl BlockFetcher {
    pub fn new(client: RpcClient, pool: PgPool, config: Config) -> Self {
        Self { client, pool, config }
    }

    /// Start the indexing loop. Runs forever until the process is killed
    /// or an unrecoverable error occurs.
    ///
    /// Recoverable errors (RPC timeouts, temporary DB issues) are logged
    /// and retried. Unrecoverable errors (reorg detected, DB corruption)
    /// are returned immediately — the caller decides whether to restart.
    pub async fn run(&self) -> Result<()> {
        let mut contracts = db::load_contracts(&self.pool).await?;
        let registry = EventRegistry::from_contracts(&contracts)?;


        let start = match db::get_last_indexed_block(&self.pool).await? {
            Some(n) => {
                println!("Resuming from block {}", n + 1);
                n + 1
            }
            None => {
                println!("Fresh start from block {}", self.config.start_block);
                self.config.start_block
            }
        };

        let mut current = start;
        let mut blocks_since_reload: u64 = 0;

        loop {
            // Periodically reload contracts to pick up new registrations.
            if blocks_since_reload > CONTRACT_RELOAD_INTERVAL {
                contracts = db::load_contracts(&self.pool).await?;
                blocks_since_reload = 0;
            }

            // Get the current safe ceiling — tip minus confirmation depth.
            let tip = self.client.get_block_number().await?;
            let ceiling = tip.saturating_sub(self.config.confirmation_depth);

            if current > ceiling {
                println!(
                    "Caught up at block {}. Waiting {}s...",
                    current, POLL_INTERVAL_SECS
                );
                tokio::time::sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
                continue;
            }

            // Fetch the block.
            let block = match self.client.get_block_by_number(current).await? {
                Some(b) => b,
                None => {
                    // Block doesn't exist yet — wait and retry.
                    tokio::time::sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
                    continue;
                }
            };

            // Reorg detection — verify parent hash continuity.
            if current > start {
                self.check_reorg(&block, current).await?;
            }

            let logs = if contracts.is_empty() {
                vec![]
            } else {
                let addresses = contracts
                    .iter()
                    .map(|c| c.address.clone())
                    .collect::<Vec<_>>();

                let filter = LogFilter {
                    from_block: format!("0x{current:x}"),
                    to_block: format!("0x{current:x}"),
                    address: addresses,
                };
                self.client.get_logs(&filter).await?
            };

            let decoded: Vec<_> = logs
                .iter()
                .filter_map(|log| registry.decode_log(log).transpose())
                .collect::<Result<_>>()?;

            db::save_block(&self.pool, &block, &logs).await?;
            db::save_decoded_events(&self.pool, &decoded).await?;

            println!(
                "Indexed block {} | txs: {} | logs: {} | decoded: {}",
                block.number,
                block.transactions.len(),
                logs.len(),
                decoded.len()
            );

            current += 1;
            blocks_since_reload += 1;
        }
    }

    /// Verify that the fetched block's parent hash matches
    /// what we have stored for the previous block.
    ///
    /// A mismatch means a reorg occurred — blocks we indexed
    /// may no longer be on the canonical chain.
    async fn check_reorg(
        &self,
        block: &Block,
        current: u64,
    ) -> Result<()> {
        let stored_hash = db::get_block_hash(&self.pool, current - 1).await?;

        if let Some(expected) = stored_hash {
            if block.parent_hash != expected {
                return Err(IndexerError::ReorgDetected {
                    block_number: current,
                    expected,
                    actual: block.parent_hash.clone(),
                });
            }
        }

        Ok(())
    }
}
