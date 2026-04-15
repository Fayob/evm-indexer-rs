# EVM Indexer

A production-grade Ethereum event indexer written in Rust. Connects to any Ethereum-compatible JSON-RPC node, listens for new blocks, decodes transaction logs using ABI definitions, persists structured data in PostgreSQL, and serves it via a REST API.

---

## What It Does

Ethereum nodes are optimized for consensus and execution, not for queries. Asking a node for all Transfer events for a given address across thousands of blocks will time out, get rate-limited, or return incomplete data. Nodes are not databases. They are state machines.

This indexer is the purpose-built read layer on top of that state machine. It:

- Follows the chain block by block, resuming from a checkpoint after crashes
- Decodes raw log bytes into structured, named event parameters using ABI definitions
- Persists blocks, transactions, raw logs, and decoded events atomically in PostgreSQL
- Detects chain reorganizations via parent hash continuity checks
- Serves indexed data via a REST API with filtering by contract and event name
- Supports any number of contracts from any number of users, not hardcoded to a single contract

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        EVM Indexer                              │
│                                                                 │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────────┐   │
│  │  RPC Client  │───▶│    Fetcher   │───▶│   Log Decoder    │   │
│  │              │    │              │    │                  │   │
│  │ eth_getBlock │    │ Block loop   │    │ ABI → typed      │   │
│  │ eth_getLogs  │    │ Checkpoint   │    │ DecodedEvent     │   │
│  └──────────────┘    │ Reorg check  │    └────────┬─────────┘   │
│                      └──────────────┘             │             │
│                                                   ▼             │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                   Storage Layer (PostgreSQL)             │   │
│  │                                                          │   │
│  │  blocks │ transactions │ logs │ decoded_events │ state   │   │
│  └──────────────────────────────────────────────────────────┘   │
│                             │                                   │
│                             ▼                                   │
│                    ┌──────────────────┐                         │
│                    │    REST API      │                         │
│                    │  (Axum server)   │                         │
│                    └──────────────────┘                         │
└─────────────────────────────────────────────────────────────────┘
```

The fetcher and API server run as concurrent Tokio tasks in the same process. Each module has a single responsibility and a single direction of dependency, `api` depends on `storage`, `fetcher` depends on `rpc` and `storage`, nothing depends on `api`.

---

## Project Structure

```
evm-indexer/
├── Cargo.toml
├── .env.example
├── migrations/
│   ├── 001_initial_schema.sql       # blocks, transactions, logs, indexer_state
│   ├── 002_contracts.sql            # contract registry
│   └── 003_decoded_events.sql       # decoded event storage with indexes
└── src/
    ├── main.rs                      # wires everything, starts Tokio runtime
    ├── lib.rs                       # import all modules 
    ├── config.rs                    # typed config from environment variables
    ├── error.rs                     # unified error type with thiserror
    ├── rpc/
    │   ├── client.rs                # generic JSON-RPC 2.0 HTTP client
    │   └── types.rs                 # Block, Transaction, Log, LogFilter
    ├── fetcher/
    │   └── block_fetcher.rs         # main indexing loop, checkpoint, reorg detection
    ├── decoder/
    │   ├── abi.rs                   # ABI types, event selector computation (keccak256)
    │   └── log_decoder.rs           # EventRegistry, raw log → DecodedEvent
    ├── storage/
    │   ├── db.rs                    # connection pool, migrations, all query functions
    │   └── models.rs                # Rust structs that map to database rows
    └── api/
        ├── server.rs                # Axum router setup
        └── handlers.rs              # HTTP handlers — one per endpoint
```

---

## How ABI Decoding Works

A raw Ethereum log is opaque bytes until you apply an ABI definition:

```
topics[0]  →  keccak256("Transfer(address,address,uint256)")  →  event selector
topics[1]  →  from address (32-byte padded)
topics[2]  →  to address (32-byte padded)
data       →  ABI-encoded value (uint256)
```

At startup the indexer builds an `EventRegistry`, a hash map from event selector to ABI definition, built from all registered contracts. For each incoming log, it looks up `topics[0]` in the registry. On a match, it decodes indexed parameters from topics and non-indexed parameters from data, producing a named JSON object:

```json
{
  "from": "0xd91efec7e42f80156d1d9f660a69847188950747",
  "to": "0x3974549dc16bf72af6fc3668d5f6c092c9e91c2b",
  "value": "7534659460"
}
```

The registry reloads every 10 blocks so newly registered contracts are picked up without a restart.

---

## Reorg Handling

Ethereum's chain tip is probabilistic. Two validators can produce competing blocks at the same height, the network resolves this by following the heaviest chain. Blocks you indexed from the losing chain contain events that never happened on the canonical chain.

This indexer uses a **confirmation depth** strategy: only index blocks that are N confirmations behind the tip (default: 12). Reorgs almost never reach 12 blocks deep.

Additionally, before indexing each block, the fetcher verifies that the block's `parentHash` matches the stored hash of the previous block. A mismatch halts the indexer immediately with a `ReorgDetected` error rather than silently persisting invalid data.

---

## Prerequisites

- Rust (edition 2024)
- Docker
- An Ethereum JSON-RPC endpoint (Alchemy, Infura, or a local node)

---

## Setup

**1. Clone the repository**

```bash
git clone https://github.com/Fayob/evm-indexer-rs.git
cd evm-indexer-rs
```

**2. Start PostgreSQL**

```bash
docker run --name evm-indexer-db \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=evm_indexer \
  -p 5435:5432 \
  -d postgres:18
```

**3. Configure environment**

```bash
cp .env.example .env
```

Edit `.env`:


```
RPC_URL=https://eth-mainnet.g.alchemy.com/v2/YOUR_KEY
DATABASE_URL=postgres://user:password@localhost:5432/evm_indexer
START_BLOCK=21000000
CONFIRMATION_DEPTH=12
API_PORT=3000
```

**4. Build**

```bash
cargo build
```

**5. Run**

```bash
cargo run
```

The indexer applies migrations automatically on startup. You should see:

```
API server listening on 0.0.0.0:3000
Fresh start from block 21000000
Indexed block 21000000 | txs: 181 | logs: 17 | decoded: 15
Indexed block 21000001 | txs: 203 | logs: 4 | decoded: 4
```

**Note**
If you want to run migration manually and your db is running on docker. For postgres run

```
docker exec -i postgres-db psql -U postgres -d <your_db_name> < migrations/001_initial_schema.sql
```

---

## Registering a Contract

Send a `POST /contracts` request with the contract address, a human-readable name, and the ABI events you want to index:

```bash
curl -X POST http://localhost:3000/contracts \
  -H "Content-Type: application/json" \
  -d '{
    "address": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
    "name": "USDC",
    "abi": [
      {
        "type": "event",
        "name": "Transfer",
        "inputs": [
          { "name": "from", "type": "address", "indexed": true },
          { "name": "to", "type": "address", "indexed": true },
          { "name": "value", "type": "uint256", "indexed": false }
        ]
      }
    ]
  }'
```

You do not need the full contract ABI, only the event definitions you want decoded.

The fetcher picks up the new contract within 10 blocks without requiring a restart.

---

## API Reference

### `GET /contracts`

List all registered contracts.

```bash
curl http://localhost:3000/contracts
```

```json
[
  {
    "address": "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
    "name": "USDC"
  }
]
```

---

### `POST /contracts`

Register a contract for indexing.

**Body**

| Field     | Type   | Description                        |
|-----------|--------|------------------------------------|
| `address` | string | Contract address (any case)        |
| `name`    | string | Human-readable label               |
| `abi`     | array  | ABI event definitions to index     |

---

### `GET /events`

Query decoded events. All parameters are optional.

| Parameter  | Type    | Description                              |
|------------|---------|------------------------------------------|
| `contract` | string  | Filter by contract address               |
| `event`    | string  | Filter by event name e.g. `Transfer`     |
| `limit`    | integer | Max results, default 50, max 500         |

```bash
# All recent events
curl http://localhost:3000/events

# USDC Transfer events only
curl "http://localhost:3000/events?event=Transfer&limit=10"

# All events for a specific contract
curl "http://localhost:3000/events?contract=0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"
```

```json
[
  {
    "contract_address": "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
    "contract_name": "USDC",
    "event_name": "Transfer",
    "block_number": 21000000,
    "transaction_hash": "0x24e20c506fd16546178a03c955bca381376f97b9ff5aefb726abf84dea6c8913",
    "log_index": 1,
    "parameters": {
      "from": "0xd91efec7e42f80156d1d9f660a69847188950747",
      "to": "0x3974549dc16bf72af6fc3668d5f6c092c9e91c2b",
      "value": "7534659460"
    }
  }
]
```

---

## Configuration Reference

| Variable             | Required | Default | Description                                      |
|----------------------|----------|---------|--------------------------------------------------|
| `RPC_URL`            | yes      | —       | Ethereum JSON-RPC endpoint URL                   |
| `DATABASE_URL`       | yes      | —       | PostgreSQL connection string                     |
| `START_BLOCK`        | no       | `0`     | Block to start from on a fresh database          |
| `CONFIRMATION_DEPTH` | no       | `12`    | Blocks behind tip before indexing                |
| `API_PORT`           | no       | `3000`  | Port for the REST API                            |

---

## Database Schema

| Table             | Description                                              |
|-------------------|----------------------------------------------------------|
| `blocks`          | Indexed block headers with hash and parent hash          |
| `transactions`    | Transactions with from, to, value, input data            |
| `logs`            | Raw logs with topics array and data                      |
| `decoded_events`  | Decoded event parameters as JSONB                        |
| `contracts`       | Registered contracts with ABI                            |
| `indexer_state`   | Single-row checkpoint — last successfully indexed block  |

All block writes are atomic: block data, logs, decoded events, and the checkpoint update commit in a single PostgreSQL transaction. A crash mid-block leaves the database in a consistent state, the block is re-indexed on restart and upserts handle deduplication.

---

## Known Limitations and Production Upgrade Path

**uint256 precision** — values above `u128::MAX` are not supported. A production system uses a `U256` type from the `alloy-primitives` or `ethnum` crate.

**Dynamic ABI types** — `string`, `bytes`, and array types in the non-indexed `data` field are not yet decoded. Fixed-size types (`address`, `uint*`, `bool`, `bytes32`) are fully supported.

**No authentication** — the API is open. A production deployment adds JWT or API key authentication and per-user contract isolation via a `user_id` column on the `contracts` table.

**No WebSocket support** — the fetcher polls via HTTP. A WebSocket subscription to `eth_subscribe` reduces latency from ~12 seconds to milliseconds at the tip.

**Single process** — the fetcher and API run in one process. A production system separates them so the API can be scaled horizontally while the fetcher runs as a single writer.

---

## Tech Stack

| Crate             | Purpose                              |
|-------------------|--------------------------------------|
| `tokio`           | Async runtime                        |
| `reqwest`         | HTTP client for JSON-RPC calls       |
| `serde`           | Serialization and deserialization    |
| `sqlx`            | Async PostgreSQL driver              |
| `axum`            | HTTP server and routing              |
| `thiserror`       | Typed error definitions              |
| `tiny-keccak`     | Keccak256 for event selector hashing |
| `hex`             | Hex encoding and decoding            |
| `dotenvy`         | `.env` file loading in development   |

---

## License

MIT
