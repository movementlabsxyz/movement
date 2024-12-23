pragma solidity ^0.8.22;

interface IAtomicBridgeCounterpartyMOVE {
    // Event emitted when a new atomic bridge transfer is locked
    event BridgeTransferLocked(
        bytes32 indexed bridgeTransferId, address indexed recipient, uint256 amount, bytes32 hashLock, uint256 timeLock
    );

    // Event emitted when a BridgeTransfer is completed
    event BridgeTransferCompleted(bytes32 indexed bridgeTransferId, bytes32 pre_image);

    // Event emitted when a BridgeTransfer is aborted
    event BridgeTransferAborted(bytes32 indexed bridgeTransferId);

    error ZeroAmount();
    error MOVETransferFailed();
    error BridgeTransferInvalid();
    error InvalidSecret();
    error BridgeTransferHasBeenCompleted();
    error BridgeTransferStateNotInitialized();
    error BridgeTransferStateNotPending();
    error InsufficientMOVEBalance();
    error TimeLockExpired();
    error TimeLockNotExpired();
    error ZeroAddress();
    error Unauthorized();

    /**
     * @dev Locks the assets for a new atomic bridge transfer
     * @param initiator The address of the initiator of the BridgeTransfer
     * @param bridgeTransferId A unique id representing this BridgeTransfer
     * @param hashLock The hash of the secret (HASH) that will unlock the funds
     * @param recipient The address to which to transfer the funds
     * @param amount The amount of MOVE to lock
     *
     */
    function lockBridgeTransfer(
        bytes32 initiator,
        bytes32 bridgeTransferId,
        bytes32 hashLock,
        address recipient,
        uint256 amount
    ) external;

    /**
     * @dev Completes the bridge transfer and withdraws MOVE to the recipient
     * @param bridgeTransferId Unique identifier for the BridgeTransfer
     * @param preImage The secret that unlocks the funds
     *
     */
    function completeBridgeTransfer(bytes32 bridgeTransferId, bytes32 preImage) external;

    /**
     * @dev Cancels the bridge transfer and refunds the initiator if the timelock has expired
     * @param bridgeTransferId Unique identifier for the BridgeTransfer
     *
     */
    function abortBridgeTransfer(bytes32 bridgeTransferId) external;
}
