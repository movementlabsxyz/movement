// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

interface INativeBridgeCounterpartyMOVE {
    
    function initialize(address _nativeBridgeInitiator, address owner, uint256 _timeLockDuration) external;
    function setNativeBridgeInitiator(address _nativeBridgeInitiator) external;
    function setTimeLockDuration(uint256 _timeLockDuration) external;

    function lockBridgeTransfer(
        bytes32 bridgeTransferId,
        bytes32 originator,
        address recipient,
        uint256 amount,
        bytes32 hashLock,
        uint256 initialTimestamp,
        uint256 nonce
    ) external;

    function completeBridgeTransfer(
        bytes32 bridgeTransferId,
        bytes32 originator,
        address recipient,
        uint256 amount,
        bytes32 hashLock,
        uint256 initialTimestamp,
        uint256 nonce,
        bytes32 preImage
    ) external;

    function abortBridgeTransfer(
        bytes32 bridgeTransferId,
        bytes32 originator,
        address recipient,
        uint256 amount,
        bytes32 hashLock,
        uint256 initialTimestamp,
        uint256 nonce
    ) external;

    event BridgeTransferLocked(
        bytes32 indexed bridgeTransferId,
        bytes32 indexed originator,
        address indexed recipient,
        uint256 amount,
        bytes32 hashLock,
        uint256 initialTimestamp,
        uint256 nonce
    );

    event BridgeTransferCompleted(
        bytes32 indexed bridgeTransferId,
        bytes32 indexed originator,
        address indexed recipient,
        uint256 amount,
        bytes32 hashLock,
        uint256 initialTimestamp,
        uint256 nonce,
        bytes32 preImage
    );

    event BridgeTransferAborted(bytes32 indexed bridgeTransferId);

    error ZeroAddress();
    error InvalidBridgeTransferId();
    error InvalidSecret();
    error BridgeTransferStateNotPending();
    error TimeLockExpired();
    error TimeLockNotExpired();
}
