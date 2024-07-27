pragma solidity ^0.8.22;

interface IAtomicBridgeCounterparty {
    // Event emitted when a new atomic bridge transfer is locked
    event BridgeTransferAssetsLocked(
        bytes32 indexed _bridgeTransferId,
        address indexed _recipient,
        uint256 amount,
        bytes32 _hashLock,
        uint256 _timeLock
    );

    // Event emitted when a BridgeTransfer is completed
    event BridgeTransferCompleted(bytes32 indexed _bridgeTransferId, bytes32 pre_image);

    // Event emitted when a BridgeTransfer is cancelled
    event BridgeTransferCancelled(bytes32 indexed _bridgeTransferId);

    error ZeroAmount();
    error WETHTransferFailed();
    error BridgeTransferInvalid();
    error InvalidSecret();
    error BridgeTransferHasBeenCompleted();
    error BridgeTransferStateNotInitialized();
    error TimeLockNotExpired();
    error ZeroAddress();
    error Unauthorized();

    /**
     * @dev Locks the assets for a new atomic bridge transfer
     * @param _bridgeTransferId A unique id representing this BridgeTransfer
     * @param _hashLock The hash of the secret (HASH) that will unlock the funds
     * @param _timeLock The timestamp until which this BridgeTransfer is valid and can be executed
     * @param _recipient The address to which to transfer the funds
     * @param _amount The amount of WETH to lock
     * @return bool indicating successful lock
     *
     */
    function lockBridgeTransferAssets(
        bytes32 _bridgeTransferId,
        bytes32 _hashLock,
        uint256 _timeLock,
        address _recipient,
        uint256 _amount
    ) external returns (bool);

    /**
     * @dev Completes the bridge transfer and withdraws WETH to the recipient
     * @param _bridgeTransferId Unique identifier for the BridgeTransfer
     * @param preImage The secret that unlocks the funds
     *
     */
    function completeBridgeTransfer(bytes32 _bridgeTransferId, bytes32 preImage) external;

    /**
     * @dev Cancels the bridge transfer and refunds the initiator if the timelock has expired
     * @param _bridgeTransferId Unique identifier for the BridgeTransfer
     *
     */
    function abortBridgeTransfer(bytes32 _bridgeTransferId) external;
}

