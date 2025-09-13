## Backend for Airdrop Eligibility Service

### Usage Examples:

**Upload CSV:**
```bash
curl -X POST "http://localhost:3000/api/v1/upload-csv" \
  -F "round_id=1" \
  -F "csv_file=@eligibility_round_1.csv"
```

**Submit Trie to Blockchain:**
```bash
curl -X POST "http://localhost:3000/api/v1/submit-trie/1"
```

**Verify Eligibility:**
```bash
curl -X POST "http://localhost:3000/api/v1/verify-eligibility" \
  -H "Content-Type: application/json" \
  -d '{
    "round_id": 1,
    "address": "0x742C4d97C86bCF0176776C16e073b8c6f9Db4021",
    "amount": "1000000000000000000"
  }'
```
