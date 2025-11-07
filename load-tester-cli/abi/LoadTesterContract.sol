// Generated with abi2sol
// Usage: sol! { ... }

interface LoadTesterContract {
    // Constructor
    constructor();

    // Types
    struct CustomStruct {
        uint128 a;
        uint64 b;
        uint32 c;
        bool flag;
        bytes blob;
        address owner;
    }

    // Functions
    function pureHash(uint256 n) external pure returns (bytes32);
    function DEFAULT_ADMIN_ROLE() external view returns (bytes32);
    function balances(address) external view returns (uint256);
    function bytesArray(uint256) external view returns (bytes);
    function coldSlot1() external view returns (uint256);
    function coldSlot2() external view returns (uint256);
    function coldSlot3() external view returns (uint256);
    function dataMap(bytes32) external view returns (uint128 a, uint64 b, uint32 c, bool flag, bytes blob, address owner);
    function delegateTarget() external view returns (address);
    function dummyCallee() external view returns (address);
    function dynamicArray(uint256) external view returns (uint256);
    function dynamicBytes() external view returns (bytes);
    function dynamicString() external view returns (string);
    function erc1155() external view returns (address);
    function erc20() external view returns (address);
    function erc721() external view returns (address);
    function eventPayloadBytes() external view returns (uint256);
    function fixedArray(uint256) external view returns (uint256);
    function gasLoopIters() external view returns (uint256);
    function getData(bytes32 key) external view returns (Data);
    function getKnobs() external view returns (uint256 _gasLoopIters, uint256 _sstoreWrites, uint8 _revertMode, uint256 _randomRevertPct, uint256 _eventPayloadBytes, bool _reentrancyEnabled, address _delegateTarget, address _dummyCallee, address _erc20, address _erc721, address _erc1155, bytes32 _merkleRoot);
    function getRoleAdmin(bytes32 role) external view returns (bytes32);
    function hasRole(bytes32 role, address account) external view returns (bool);
    function hashLoop(uint256 iters) external view returns (bytes32);
    function hotSlot1() external view returns (uint256);
    function hotSlot2() external view returns (uint256);
    function hotSlot3() external view returns (uint256);
    function merkleRoot() external view returns (bytes32);
    function nested(address, uint256) external view returns (bytes32);
    function paused() external view returns (bool);
    function randomRevertPct() external view returns (uint256);
    function reentrancyEnabled() external view returns (bool);
    function revertMode() external view returns (uint8);
    function sstoreWrites() external view returns (uint256);
    function staticQuery(address target, bytes data) external view returns (bytes);
    function supportsInterface(bytes4 interfaceId) external view returns (bool);
    function verifyProof(bytes32 leaf, bytes32[] proof) external view returns (bool);
    function verifySig(address expected, bytes32 hash, bytes sig) external view returns (bool);
    function sinkValue(address to) external payable;
    function batchMintERC1155(address[] to, uint256 id, uint256[] amt, bytes data) external;
    function batchMintERC20(address[] to, uint256[] amt) external;
    function batchMintERC721(address[] to) external;
    function bigCalldataEcho(bytes inData) external returns (bytes);
    function callDummy(bytes data, uint256 gasLimit) external;
    function consumeGas(uint256 iters) external;
    function createChild(bytes32 salt, bytes initCode) external;
    function delegateWork(bytes data, uint256 gasLimit) external;
    function grantRole(bytes32 role, address account) external;
    function growArrays(uint256 words) external;
    function reentrantEntry(uint256 depth) external;
    function renounceRole(bytes32 role, address callerConfirmation) external;
    function revokeRole(bytes32 role, address account) external;
    function setDelegateTarget(address _target) external;
    function setDummyCallee(address _callee) external;
    function setEventPayloadBytes(uint256 _bytes) external;
    function setGasLoopIters(uint256 _iters) external;
    function setMerkleRoot(bytes32 _root) external;
    function setRandomRevertPct(uint256 _pct) external;
    function setRevertMode(uint8 _mode) external;
    function setSstoreWrites(uint256 _writes) external;
    function setTokenAddresses(address _erc20, address _erc721, address _erc1155) external;
    function sweep(address to, uint256 amt) external;
    function toggleReentrancy() external;
    function touchStorage(uint256 writes, uint256 reads, bytes32 tag) external;
    function transferOperator(address newOperator) external;
    fallback() external payable;
    receive() external payable;

    // Events
    event DelegateCall(address target, bool success, uint256 gasUsed);
    event ExternalCall(address target, bool success, uint256 gasUsed, bytes returndata);
    event FundsMoved(address indexed from, address indexed to, uint256 amount);
    event Op(uint8 kind, address indexed caller, uint256 beforeGas, uint256 afterGas, bytes32 indexed tag, bytes payload);
    event Paused(address account);
    event RoleAdminChanged(bytes32 indexed role, bytes32 indexed previousAdminRole, bytes32 indexed newAdminRole);
    event RoleGranted(bytes32 indexed role, address indexed account, address indexed sender);
    event RoleRevoked(bytes32 indexed role, address indexed account, address indexed sender);
    event StorageTouched(uint256 writes, uint256 reads);
    event Unpaused(address account);

    // Errors
    error AccessControlBadConfirmation();
    error AccessControlUnauthorizedAccount(address account, bytes32 neededRole);
    error BadInput();
    error ECDSAInvalidSignature();
    error ECDSAInvalidSignatureLength(uint256 length);
    error ECDSAInvalidSignatureS(bytes32 s);
    error EnforcedPause();
    error ExpectedPause();
    error ForcedRevert(string why);
    error ReentrancyGuardReentrantCall();
    error Unauthorized();
}

