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

    error ZeroAmount();
    error MOVETransferFailed();
    error ZeroAddress();
    error InvalidLenghts();
    error InvalidBridgeTransferId();
    error CompletedBridgeTransferId();

    /**
     * @dev Creates a new bridge
     * @param recipient The address on the other chain to which to transfer funds
     * @param amount The amount of MOVE to send
     * @return bridgeTransferId A unique id representing this BridgeTransfer
     *
     */
    function initiateBridge(bytes32 recipient, uint256 amount)
        external
        returns (bytes32 bridgeTransferId);

    /**
     * @dev Completes the bridging Counterparty
     * @param bridgeTransferId Unique identifier for the BridgeTransfer
     * @param originator The address on the other chain that originated the transfer of funds
     * @param recipient The address on this chain to which to transfer funds
     * @param amount The amount to transfer
     * @param nonce The seed nonce to generate the bridgeTransferId
     *
     */
    function completeBridge(
        bytes32 bridgeTransferId,
        bytes32 originator,
        address recipient,
        uint256 amount,
        uint256 nonce
        ) external;
}
