# Load Tester Contract Interface

## Contract Overview
Benchmarking and stress testing contract for EVM networks.

## Errors

### Access Control
- `AccessControlBadConfirmation()` - Bad access control confirmation
- `AccessControlUnauthorizedAccount(address account, bytes32 neededRole)` - Unauthorized account
- `Unauthorized()` - General unauthorized access

### Input Validation
- `BadInput()` - Invalid input parameters

### Cryptography
- `ECDSAInvalidSignature()` - Invalid ECDSA signature
- `ECDSAInvalidSignatureLength(uint256 length)` - Invalid signature length
- `ECDSAInvalidSignatureS(bytes32 s)` - Invalid signature S value

### State Management
- `EnforcedPause()` - Operation attempted while paused
- `ExpectedPause()` - Expected contract to be paused
- `ReentrancyGuardReentrantCall()` - Reentrancy detected

### Testing
- `ForcedRevert(string why)` - Intentional revert for testing

## Events

### Operations
- `Op(uint8 kind, address indexed caller, uint256 beforeGas, uint256 afterGas, bytes32 indexed tag, bytes payload)`
  - Generic operation event with gas tracking

### External Interactions
- `ExternalCall(address target, bool success, uint256 gasUsed, bytes returndata)`
  - External contract call result
- `DelegateCall(address target, bool success, uint256 gasUsed)`
  - Delegate call result

### Storage Operations
- `StorageTouched(uint256 writes, uint256 reads)`
  - Storage read/write tracking

### Funds Management
- `FundsMoved(address indexed from, address indexed to, uint256 amount)`
  - Native token transfers

### Access Control
- `RoleAdminChanged(bytes32 indexed role, bytes32 indexed previousAdminRole, bytes32 indexed newAdminRole)`
- `RoleGranted(bytes32 indexed role, address indexed account, address indexed sender)`
- `RoleRevoked(bytes32 indexed role, address indexed account, address indexed sender)`

### Pausable
- `Paused(address account)`
- `Unpaused(address account)`

## Functions by Category

### 1. Gas Consumption Tests

#### `consumeGas(uint256 iters)`
**Purpose**: Consume gas through computation
**Parameters**: Number of iterations
**State**: Nonpayable
**Use Case**: Test gas consumption patterns

#### `hashLoop(uint256 iters) returns (bytes32)`
**Purpose**: CPU-intensive hashing loop
**Parameters**: Number of iterations
**Returns**: Resulting hash
**State**: View
**Use Case**: Test computational overhead

#### `pureHash(uint256 n) returns (bytes32)`
**Purpose**: Pure hash computation
**Parameters**: Input number
**Returns**: Hash result
**State**: Pure
**Use Case**: Test pure function gas costs

### 2. Storage Operations

#### `touchStorage(uint256 writes, uint256 reads, bytes32 tag)`
**Purpose**: Perform storage reads/writes
**Parameters**:
  - writes: Number of storage writes
  - reads: Number of storage reads
  - tag: Identifier tag
**Use Case**: Test storage gas costs (SLOAD/SSTORE)

#### Storage Slots (View Functions)
- `hotSlot1/2/3() returns (uint256)` - Frequently accessed slots
- `coldSlot1/2/3() returns (uint256)` - Rarely accessed slots
**Use Case**: Test hot vs cold storage access costs

#### `growArrays(uint256 words)`
**Purpose**: Expand dynamic arrays
**Parameters**: Number of words to add
**Use Case**: Test dynamic storage expansion costs

### 3. Calldata and Memory Tests

#### `bigCalldataEcho(bytes inData) returns (bytes)`
**Purpose**: Echo large calldata
**Parameters**: Input data
**Returns**: Same data
**Use Case**: Test calldata handling and costs

#### Storage Views
- `dynamicArray(uint256) returns (uint256)` - Dynamic array access
- `fixedArray(uint256) returns (uint256)` - Fixed array access
- `bytesArray(uint256) returns (bytes)` - Bytes array access
- `dynamicBytes() returns (bytes)` - Dynamic bytes storage
- `dynamicString() returns (string)` - Dynamic string storage

### 4. External Interactions

#### `callDummy(bytes data, uint256 gasLimit)`
**Purpose**: External contract call
**Parameters**:
  - data: Call data
  - gasLimit: Gas limit for call
**Use Case**: Test external call overhead

#### `delegateWork(bytes data, uint256 gasLimit)`
**Purpose**: Delegate call to another contract
**Parameters**:
  - data: Call data
  - gasLimit: Gas limit
**Use Case**: Test delegate call patterns

#### `staticQuery(address target, bytes data) returns (bytes)`
**Purpose**: Static call to external contract
**Parameters**:
  - target: Contract address
  - data: Call data
**Returns**: Return data
**State**: View
**Use Case**: Test view function calls

### 5. ERC Token Operations

#### `batchMintERC20(address[] to, uint256[] amt)`
**Purpose**: Batch mint ERC-20 tokens
**Parameters**:
  - to: Recipient addresses
  - amt: Amounts per recipient
**Use Case**: Test ERC-20 batch operations

#### `batchMintERC721(address[] to)`
**Purpose**: Batch mint ERC-721 NFTs
**Parameters**: Recipient addresses
**Use Case**: Test ERC-721 batch minting

#### `batchMintERC1155(address[] to, uint256 id, uint256[] amt, bytes data)`
**Purpose**: Batch mint ERC-1155 tokens
**Parameters**:
  - to: Recipients
  - id: Token ID
  - amt: Amounts
  - data: Additional data
**Use Case**: Test ERC-1155 batch operations

### 6. Contract Creation

#### `createChild(bytes32 salt, bytes initCode)`
**Purpose**: Deploy new contract using CREATE2
**Parameters**:
  - salt: Deployment salt
  - initCode: Contract bytecode
**Use Case**: Test contract deployment costs

### 7. Cryptography Tests

#### `verifyProof(bytes32 leaf, bytes32[] proof) returns (bool)`
**Purpose**: Verify Merkle proof
**Parameters**:
  - leaf: Leaf node
  - proof: Merkle proof array
**Returns**: Verification result
**State**: View
**Use Case**: Test Merkle tree verification

#### `verifySig(address expected, bytes32 hash, bytes sig) returns (bool)`
**Purpose**: Verify ECDSA signature
**Parameters**:
  - expected: Expected signer address
  - hash: Message hash
  - sig: Signature
**Returns**: Verification result
**State**: View
**Use Case**: Test signature verification

### 8. Reentrancy Tests

#### `reentrantEntry(uint256 depth)`
**Purpose**: Test reentrancy behavior
**Parameters**: Recursion depth
**Use Case**: Test reentrancy guards

#### `toggleReentrancy()`
**Purpose**: Enable/disable reentrancy
**Use Case**: Configure reentrancy testing

### 9. Configuration Functions

#### `setGasLoopIters(uint256 _iters)`
**Purpose**: Set gas loop iterations

#### `setSstoreWrites(uint256 _writes)`
**Purpose**: Set number of storage writes

#### `setRevertMode(uint8 _mode)`
**Purpose**: Configure revert behavior

#### `setRandomRevertPct(uint256 _pct)`
**Purpose**: Set random revert percentage

#### `setEventPayloadBytes(uint256 _bytes)`
**Purpose**: Set event payload size

#### `setMerkleRoot(bytes32 _root)`
**Purpose**: Set Merkle root for verification

#### `setDelegateTarget(address _target)`
**Purpose**: Set delegate call target

#### `setDummyCallee(address _callee)`
**Purpose**: Set dummy call target

#### `setTokenAddresses(address _erc20, address _erc721, address _erc1155)`
**Purpose**: Configure token contract addresses

### 10. View/Query Functions

#### `getKnobs() returns (...)`
**Purpose**: Get all configuration parameters
**Returns**: All tunable parameters
**Use Case**: Query current configuration

#### `getData(bytes32 key) returns (Data)`
**Purpose**: Get structured data by key
**Returns**: Data struct
**Use Case**: Test complex return types

### 11. Value Management

#### `sinkValue(address payable to)`
**Purpose**: Send native tokens
**Parameters**: Recipient address
**State**: Payable
**Use Case**: Test native token transfers

#### `sweep(address payable to, uint256 amt)`
**Purpose**: Sweep contract balance
**Parameters**:
  - to: Recipient
  - amt: Amount to transfer
**Use Case**: Recover funds

### 12. Access Control

#### `grantRole(bytes32 role, address account)`
**Purpose**: Grant role to account

#### `revokeRole(bytes32 role, address account)`
**Purpose**: Revoke role from account

#### `renounceRole(bytes32 role, address callerConfirmation)`
**Purpose**: Renounce own role

#### `hasRole(bytes32 role, address account) returns (bool)`
**Purpose**: Check role membership

#### `getRoleAdmin(bytes32 role) returns (bytes32)`
**Purpose**: Get role admin

#### `transferOperator(address newOperator)`
**Purpose**: Transfer operator role

### 13. State Variables (View Functions)

- `DEFAULT_ADMIN_ROLE() returns (bytes32)` - Admin role identifier
- `balances(address) returns (uint256)` - Balance mapping
- `gasLoopIters() returns (uint256)` - Gas loop configuration
- `sstoreWrites() returns (uint256)` - Storage write configuration
- `revertMode() returns (uint8)` - Revert behavior mode
- `randomRevertPct() returns (uint256)` - Random revert percentage
- `eventPayloadBytes() returns (uint256)` - Event payload size
- `reentrancyEnabled() returns (bool)` - Reentrancy flag
- `delegateTarget() returns (address)` - Delegate target address
- `dummyCallee() returns (address)` - Dummy call target
- `erc20/721/1155() returns (address)` - Token addresses
- `merkleRoot() returns (bytes32)` - Merkle root
- `paused() returns (bool)` - Pause state
- `dataMap(bytes32) returns (...)` - Data storage mapping
- `nested(address, uint256) returns (bytes32)` - Nested mapping

## Test Scenarios

### 1. Basic Load Test
- Call `consumeGas()` repeatedly
- Measure TPS and gas consumption

### 2. Storage Stress Test
- Call `touchStorage()` with varying reads/writes
- Test hot vs cold storage costs

### 3. Batch Operations Test
- Use `batchMintERC20/721/1155()`
- Measure batch efficiency

### 4. Calldata Test
- Call `bigCalldataEcho()` with increasing sizes
- Measure calldata costs

### 5. External Call Test
- Use `callDummy()` and `delegateWork()`
- Measure call overhead

### 6. Crypto Test
- Call `verifyProof()` and `verifySig()`
- Test verification costs

### 7. Mixed Workload
- Combine multiple operations
- Simulate realistic usage patterns
