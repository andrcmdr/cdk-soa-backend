// Generated with abi2sol
// Usage: sol! { ... }

interface AirdropContract {
    // Types
    struct CustomStruct {
        uint256 roundId;
        bytes32 rootHash;
        uint256 totalEligible;
        uint256 totalAmount;
        uint256 startTime;
        uint256 endTime;
        bool isActive;
        string metadataUri;
    }

    // Functions
    function getContractVersion() external view returns (string);
    function getRoundCount() external view returns (uint256);
    function getRoundMetadata(uint256 roundId) external view returns (RoundMetadata);
    function getTrieRoot(uint256 roundId) external view returns (bytes32);
    function isRootHashExists(bytes32 rootHash) external view returns (bool);
    function isRoundActive(uint256 roundId) external view returns (bool);
    function verifyEligibility(uint256 roundId, address user, uint256 amount, bytes[] proof) external view returns (bool);
    function updateTrieRoot(uint256 roundId, bytes32 rootHash, bytes trieData) external;

    // Events
    event EligibilityVerified(uint256 indexed roundId, address indexed user, uint256 amount);
    event RoundCreated(uint256 indexed roundId, uint256 startTime, uint256 endTime, string metadataUri);
    event RoundStatusChanged(uint256 indexed roundId, bool isActive);
    event TrieRootUpdated(uint256 indexed roundId, bytes32 indexed rootHash, uint256 totalEligible, uint256 totalAmount);

}

