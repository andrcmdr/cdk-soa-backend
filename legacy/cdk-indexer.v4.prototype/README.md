# Polygon CDK Indexer (Rust + Alloy)

### What’s new in this update
- Upgraded to latest `alloy = 1.0.25` meta-crate with `features = ["full"]`.
- ABI decoding uses `json_abi::Event::selector()` to map events and returns `event_hash`.
- `handle_log` enriches each record with: `block_hash`, `block_number`, `block_timestamp`, `transaction_hash`, `transaction_index`, `log_index`, `tx_sender`, `chain_id`, `log_hash`.
- `log_hash` computed via `Log::hash()` over a **SHA3-256** hasher adapter.
- Comprehensive Postgres schema & indexes for production queries.
- Publishes enriched JSON to NATS per-contract subject: `<subject>.<contract>.events`.
- Subscribes to logs for **all contracts** defined in `config.yaml`.

### Run locally
```bash
# 1) Put ABIs under ./abi and update config.yaml addresses/paths
# 2) Start services
docker compose up --build
# 3) Tail logs
docker compose logs -f indexer
```
