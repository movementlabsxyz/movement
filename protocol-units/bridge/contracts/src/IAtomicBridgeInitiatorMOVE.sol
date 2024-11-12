// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

interface IAtomicBridgeInitiatorMOVE {

    bytes32 constant DEFAULT_ADMIN_ROLE = 0x00;
    bytes32 constant ADMIN_ROLE = keccak256("ADMIN_ROLE");
    bytes32 constant RELAYER_ROLE = keccak256("RELAYER_ROLE");
    bytes32 constant REFUNDER_ROLE = keccak256("REFUNDER_ROLE");
    
    // Event emitted when a new atomic bridge transfer is created
    event BridgeTransferInitiated(
        bytes32 indexed _bridgeTransferId,
        address indexed _originator,
        bytes32 indexed _recipient,
        uint256 amount,
        bytes32 _hashLock,
        uint256 _timeLock
    );
    // Event emitted when a BridgeTransfer is completed (withdrawn)
    event BridgeTransferCompleted(bytes32 indexed _bridgeTransferId, bytes32 pre_image);
    // Event emitted when a BridgeTransfer is refunded
    event BridgeTransferRefunded(bytes32 indexed _bridgeTransferId);

    error ZeroAmount();
    error MOVETransferFailed();
    error BridgeTransferInvalid();
    error InvalidSecret();
    error BridgeTransferHasBeenCompleted();
    error BridgeTransferStateNotInitialized();
    error InsufficientMOVEBalance();
    error TimeLockNotExpired();
    error TimelockExpired();
    error ZeroAddress();
    error Unauthorized();
    error ZeroValue();


    /**
     * @dev Creates a new atomic bridge transfer using native ETH
     * @param _wethAmount The amount of WETH to send
     * @param _recipient The address on the other chain to which to transfer the funds
     * @param _hashLock The hash of the secret (HASH) that will unlock the funds
     * @return _bridgeTransferId A unique id representing this BridgeTransfer
     *
     */
    function initiateBridgeTransfer(uint256 _wethAmount, bytes32 _recipient, bytes32 _hashLock)
        external
        returns (bytes32 _bridgeTransferId);

    /**
     * @dev Completes the bridging Counterparty
     * @param _bridgeTransferId Unique identifier for the BridgeTransfer
     * @param preImage The secret that unlocks the funds
     *
     */
    function completeBridgeTransfer(bytes32 _bridgeTransferId, bytes32 preImage) external;

    /**
     * @dev Refunds the funds back to the initiator if the timelock has expired
     * @param _bridgeTransferId Unique identifier for the BridgeTransfer
     *
     */
    function refundBridgeTransfer(bytes32 _bridgeTransferId) external;
}
