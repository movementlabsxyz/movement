// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

interface INativeBridge {
    // Event emitted when a new native bridge transfer is created
    event BridgeTransferInitiated(
        bytes32 indexed bridgeTransferId,
        address indexed originator,
        bytes32 indexed recipient,
        uint256 amount,
        uint256 nonce
    );
    // Event emitted when a BridgeTransfer is completed (withdrawn)
    event BridgeTransferCompleted(
        bytes32 indexed bridgeTransferId,
        bytes32 indexed originator,
        address indexed recipient,
        uint256 amount,
        uint256 nonce
    );

    event InsuranceFundUpdated(address insuranceFund);
    event PauseToggled(bool paused);

    error ZeroAmount();
    error MOVETransferFailed();
    error ZeroAddress();
    error InvalidLenghts();
    error InvalidBridgeTransferId();
    error CompletedBridgeTransferId();
    error InvalidNonce();
    error OutboundRateLimitExceeded();
    error InboundRateLimitExceeded();

    /**
     * @dev Creates a new bridge
     * @param recipient The address on the other chain to which to transfer funds
     * @param amount The amount of MOVE to send
     * @return bridgeTransferId A unique id representing this BridgeTransfer
     *
     */
    function initiateBridgeTransfer(bytes32 recipient, uint256 amount) external returns (bytes32 bridgeTransferId);

    /**
     * @dev Completes the bridging of funds
     * @param bridgeTransferId Unique identifier for the BridgeTransfer
     * @param originator The address on the other chain that originated the transfer of funds
     * @param recipient The address on this chain to which to transfer funds
     * @param amount The amount to transfer
     * @param nonce The seed nonce to generate the bridgeTransferId
     *
     */
    function completeBridgeTransfer(
        bytes32 bridgeTransferId,
        bytes32 originator,
        address recipient,
        uint256 amount,
        uint256 nonce
    ) external;

    /**
     * @dev Completes multiple bridge transfers
     * @param bridgeTransferIds Unique identifiers for the BridgeTransfers
     * @param initiators The addresses on the other chain that originated the transfer of funds
     * @param recipients The addresses on this chain to which to transfer funds
     * @param amounts The amounts to transfer
     * @param nonces The seed nonces to generate the bridgeTransferIds
     */
    function batchCompleteBridgeTransfer(
        bytes32[] memory bridgeTransferIds,
        bytes32[] memory initiators,
        address[] memory recipients,
        uint256[] memory amounts,
        uint256[] memory nonces
    ) external;
}
