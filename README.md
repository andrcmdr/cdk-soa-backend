# CDK SOA backend components

### Polygon CDK (Chain Development Kit) blockchain backend (off-chain) components/services for SOA (service-oriented architecture) backend infrastructure and SOA service bus (MOM/MQ, message-oriented middleware, message queue) based on NATS Server.

## Components:
 - Events Monitor for monitoring and indexing contract events for CDK chain.
 - Events Monitor (more generic) for monitoring and indexing block data and providing request/response API and publisher/subscriber (producer/consumer) protocol for services over the NATS JetStream topics.
 - Transaction producer, library to include in other services. Includes wallet client functionality and KMS integration for storing private keys securely encrypted, for transaction signing.
 - Oracle service, for providing data from concrete API and DB.
 - Oracle service (more generic), on-chain contracts data provider component from off-chain data sources.
 - Merkle Trie generator (includes transaction producer), for generating Merkle tree, compute root hash, for storing off-chain data for on-chain contracts.
 - Airdrop Eligibility service: fully-fledged service with Merkle Trie generator and transaction producer in its core.

## CDK Indexer

### Output example:

For an event:
```solidity
event DataPosted(address sender, uint256 id, string message);
```

We'll get output:
```json
{
  "sender": "0xabc123abc123abc123abc123abc123abc123abc1",
  "id": "42",
  "message": "Data are posted!"
}
```
