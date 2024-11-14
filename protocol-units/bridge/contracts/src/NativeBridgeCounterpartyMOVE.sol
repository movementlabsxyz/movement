// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import {OwnableUpgradeable} from "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import {INativeBridgeCounterpartyMOVE} from "./INativeBridgeCounterpartyMOVE.sol";
import {NativeBridgeInitiatorMOVE} from "./NativeBridgeInitiatorMOVE.sol";

contract NativeBridgeCounterpartyMOVE is INativeBridgeCounterpartyMOVE, OwnableUpgradeable {

    enum MessageState {
        PENDING,
        COMPLETED,
        REFUNDED
    }
    NativeBridgeInitiatorMOVE public nativeBridgeInitiatorMOVE;
    mapping(bytes32 => MessageState) public bridgeTransfers;

    // Configurable time lock duration
    uint256 public counterpartyTimeLockDuration;

    function initialize(address _nativeBridgeInitiator, address owner, uint256 _timeLockDuration) public initializer {
        if (_nativeBridgeInitiator == address(0)) revert ZeroAddress();
        nativeBridgeInitiatorMOVE = NativeBridgeInitiatorMOVE(_nativeBridgeInitiator);
        __Ownable_init(owner);

        // Set the configurable time lock duration
        counterpartyTimeLockDuration = _timeLockDuration;
    }

    function setNativeBridgeInitiator(address _nativeBridgeInitiator) external onlyOwner {
        if (_nativeBridgeInitiator == address(0)) revert ZeroAddress();
        nativeBridgeInitiatorMOVE = NativeBridgeInitiatorMOVE(_nativeBridgeInitiator);
    }

    function setTimeLockDuration(uint256 _timeLockDuration) external onlyOwner {
        counterpartyTimeLockDuration = _timeLockDuration;
    }

    function lockBridgeTransfer(
        bytes32 bridgeTransferId,
        bytes32 originator,
        address recipient,
        uint256 amount,
        bytes32 hashLock,
        uint256 initialTimestamp,
        uint256 nonce
    ) external onlyOwner {

        // The time lock is now based on the configurable duration
        uint256 timeLock = block.timestamp + counterpartyTimeLockDuration;

        require(bridgeTransferId == keccak256(abi.encodePacked(originator, recipient, amount, hashLock, initialTimestamp, nonce)), InvalidBridgeTransferId());

        bridgeTransfers[bridgeTransferId] = MessageState.PENDING;

        emit BridgeTransferLocked(bridgeTransferId, originator, recipient, amount, hashLock, block.timestamp, nonce);
    }

    function completeBridgeTransfer(
        bytes32 bridgeTransferId,
        bytes32 originator,
        address recipient,
        uint256 amount,
        bytes32 hashLock,
        uint256 initialTimestamp,
        uint256 nonce, bytes32 preImage) external {
        
        require(bridgeTransferId == keccak256(abi.encodePacked(originator, recipient, amount, hashLock, initialTimestamp, nonce)), InvalidBridgeTransferId());
        require(keccak256(abi.encodePacked(preImage)) == hashLock, InvalidSecret());

        require(bridgeTransfers[bridgeTransferId] == MessageState.PENDING, BridgeTransferStateNotPending());
        
        if (block.timestamp > initialTimestamp + counterpartyTimeLockDuration) revert TimeLockExpired();

        bridgeTransfers[bridgeTransferId] = MessageState.COMPLETED;

        nativeBridgeInitiatorMOVE.withdrawMOVE(recipient, amount);

        emit BridgeTransferCompleted(bridgeTransferId, originator, recipient, amount, hashLock, initialTimestamp, nonce, preImage);
    }

    function abortBridgeTransfer(bytes32 bridgeTransferId,
    bytes32 originator,
        address recipient,
        uint256 amount,
        bytes32 hashLock,
        uint256 initialTimestamp,
        uint256 nonce) external onlyOwner {
        require(bridgeTransferId == keccak256(abi.encodePacked(originator, recipient, amount, hashLock, initialTimestamp, nonce)), InvalidBridgeTransferId());
        require(bridgeTransfers[bridgeTransferId] == MessageState.PENDING, BridgeTransferStateNotPending());
        if (block.timestamp <= initialTimestamp + counterpartyTimeLockDuration) revert TimeLockNotExpired();

        bridgeTransfers[bridgeTransferId] = MessageState.REFUNDED;

        emit BridgeTransferAborted(bridgeTransferId);
    }
}
