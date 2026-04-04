use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::{error::Result, rpc::types::{Block, Log}};

pub async fn create_pool(database_url: &str) -> Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;

    Ok(pool)
}

pub async fn run_migration(pool: &PgPool) -> Result<()> {
    sqlx::query(include_str!("../../migrations/001_initial_schema.sql"))
        .execute(pool)
        .await?;

    sqlx::query(include_str!("../../migrations/002_contracts.sql"))
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn get_last_indexed_block(pool: &PgPool) -> Result<Option<u64>> {
    let row = sqlx::query_scalar!(
        "SELECT last_indexed_block FROM indexer_state WHERE id = 1"
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|n: i64| n as u64))
}

pub async fn save_block(pool: &PgPool, block: &Block, logs: &[Log]) -> Result<()> {
    let mut tx = pool.begin().await?;

    // Write the block
    sqlx::query!(
        "INSERT INTO blocks (number, hash, parent_hash, timestamp)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (number) DO NOTHING",
        block.number as i64,
        block.hash,
        block.parent_hash,
        block.timestamp as i64,
    )
    .execute(&mut *tx)
    .await?;

    // Write each transaction.
    for txn in &block.transactions {
        sqlx::query!(
            "INSERT INTO transactions
                (hash, block_number, from_address, to_address, value, input, transaction_index)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (hash) DO NOTHING",
            txn.hash,
            block.number as i64,
            txn.from,
            txn.to,
            txn.value,
            txn.input,
            txn.transaction_index as i64,
        )
        .execute(&mut *tx)
        .await?;
    }

    // Write each log.
    for log in logs {
        sqlx::query!(
            "INSERT INTO logs
                (block_number, block_hash, transaction_hash, transaction_index,
                 log_index, address, topics, data)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT (transaction_hash, log_index) DO NOTHING",
            log.block_number as i64,
            log.block_hash,
            log.transaction_hash,
            log.transaction_index as i64,
            log.log_index as i64,
            log.address,
            &log.topics,
            log.data,
        )
        .execute(&mut *tx)
        .await?;
    }

    // Update the checkpoint — same transaction as the data writes.
    // This is the atomicity guarantee: checkpoint and data are
    // always consistent with each other.
    sqlx::query!(
        "INSERT INTO indexer_state (id, last_indexed_block)
         VALUES (1, $1)
         ON CONFLICT (id) DO UPDATE SET last_indexed_block = $1",
        block.number as i64,
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(())
}
