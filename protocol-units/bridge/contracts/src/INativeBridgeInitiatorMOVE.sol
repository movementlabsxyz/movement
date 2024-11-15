// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

interface INativeBridgeInitiatorMOVE {
    

    function initiatorTimeLockDuration() external view returns (uint256);

    function initialize(address _moveToken, address owner, uint256 _timeLockDuration) external;
    function setCounterpartyAddress(address _counterpartyAddress) external;

    function initiateBridgeTransfer(bytes32 recipient, uint256 amount, bytes32 hashLock)
        external
        returns (bytes32 bridgeTransferId);

    function completeBridgeTransfer(
        bytes32 bridgeTransferId,
        address originator,
        bytes32 recipient,
        uint256 amount,
        bytes32 hashLock,
        uint256 initialTimestamp,
        uint256 nonce,
        bytes32 preImage
    ) external;

    function refundBridgeTransfer(
        bytes32 bridgeTransferId,
        address originator,
        bytes32 recipient,
        uint256 amount,
        bytes32 hashLock,
        uint256 initialTimestamp,
        uint256 nonce
    ) external;

    function withdrawMOVE(address recipient, uint256 amount) external;

    event BridgeTransferInitiated(
        bytes32 indexed bridgeTransferId,
        address indexed originator,
        bytes32 indexed recipient,
        uint256 amount,
        bytes32 hashLock,
        uint256 initialTimestamp,
        uint256 nonce
    );

    event BridgeTransferCompleted(
        bytes32 indexed bridgeTransferId,
        address indexed originator,
        bytes32 indexed recipient,
        uint256 amount,
        bytes32 hashLock,
        uint256 initialTimestamp,
        uint256 nonce,
        bytes32 preImage
    );

    event BridgeTransferRefunded(bytes32 indexed bridgeTransferId);

    error ZeroAddress();
    error ZeroAmount();
    error MOVETransferFailed();
    error BridgeTransferNotInitialized();
    error InvalidBridgeTransferId();
    error InvalidSecret();
    error TimelockExpired();
    error TimeLockNotExpired();
    error Unauthorized();
}
