## Usage Example:

```rust
let mut service = AirdropService::new(config).await?;

// Process multiple rounds
service.process_csv_and_update_trie("data/round_1.csv", 1).await?;
service.process_csv_and_update_trie("data/round_2.csv", 2).await?;

// Submit to blockchain
service.submit_trie_update(1).await?;
service.submit_trie_update(2).await?;

// Verify eligibility
let is_eligible = service.verify_eligibility(1, user_address, amount).await?;
```
