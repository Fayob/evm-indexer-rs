use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::{decoder::log_decoder::DecodedEvent, error::Result, rpc::types::{Block, Log}, storage::models::{Contract, DecodedEventRow}};

pub async fn create_pool(database_url: &str) -> Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;

    Ok(pool)
}

pub async fn run_migration(pool: &PgPool) -> Result<()> {
    sqlx::raw_sql(include_str!("../../migrations/001_initial_schema.sql"))
        .execute(pool)
        .await?;

    sqlx::raw_sql(include_str!("../../migrations/002_contracts.sql"))
        .execute(pool)
        .await?;

    sqlx::raw_sql(include_str!("../../migrations/003_decoded_events.sql"))
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

/// Register a contract for indexing.
///
/// On conflict we do nothing — registering the same address
/// twice is idempotent. The user gets no error, the existing
/// record is preserved.
pub async fn save_contract(pool: &PgPool, contract: &Contract) -> Result<()> {
    sqlx::query(
        "INSERT INTO contracts (address, name, abi)
         VALUES ($1, $2, $3)
         ON CONFLICT (address) DO NOTHING"
    )
    .bind(&contract.address)
    .bind(&contract.name)
    .bind(&contract.abi)
    .execute(pool)
    .await?;

    Ok(())
}

/// Load all registered contracts from the database.
/// Called at fetcher startup and periodically during the index loop.
pub async fn load_contracts(pool: &PgPool) -> Result<Vec<Contract>> {
    let rows = sqlx::query_as::<_, Contract>(
        "SELECT address, name, abi FROM contracts"
    )
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

/// Fetch the stored hash for a given block number.
/// Used for reorg detection.
pub async fn get_block_hash(pool: &PgPool, block_number: u64) -> Result<Option<String>> {
    let row = sqlx::query_scalar(
        "SELECT hash FROM blocks WHERE number = $1"
    )
    .bind(block_number as i64)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

pub async fn save_decoded_events(
    pool: &PgPool,
    events: &[DecodedEvent],
) -> Result<()> {
    for event in events {
        sqlx::query(
            "INSERT INTO decoded_events
                (contract_address, contract_name, event_name,
                 block_number, transaction_hash, log_index, parameters)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (transaction_hash, log_index) DO NOTHING"
        )
        .bind(&event.contract_address)
        .bind(&event.contract_name)
        .bind(&event.event_name)
        .bind(event.block_number as i64)
        .bind(&event.transaction_hash)
        .bind(event.log_index as i64)
        .bind(&event.parameters)
        .execute(pool)
        .await?;
    }

    Ok(())
}

pub async fn get_decoded_events(
    pool: &PgPool,
    contract: Option<&str>,
    event_name: Option<&str>,
    limit: i64
) -> Result<Vec<DecodedEventRow>> {
    let rows = sqlx::query_as::<_,DecodedEventRow>(
        "SELECT contract_address, contract_name, event_name,
                block_number, transaction_hash, log_index, parameters
         FROM decoded_events
         WHERE ($1::text IS NULL OR contract_address = $1)
           AND ($2::text IS NULL OR event_name = $2)
         ORDER BY block_number DESC, log_index DESC
         LIMIT $3"
    )
    .bind(contract)
    .bind(event_name)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}