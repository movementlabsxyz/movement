// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import {OwnableUpgradeable} from "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import {INativeBridgeCounterpartyMOVE} from "./INativeBridgeCounterpartyMOVE.sol";
import {NativeBridgeInitiatorMOVE} from "./NativeBridgeInitiatorMOVE.sol";
import {console} from "forge-std/Console.sol";

contract NativeBridgeCounterpartyMOVE is INativeBridgeCounterpartyMOVE, OwnableUpgradeable {

    enum MessageState {
        NOT_INITIALIZED,
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
        _verifyHash(bridgeTransferId, originator, recipient, amount, hashLock, initialTimestamp, nonce);
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
        uint256 nonce,
        bytes32 preImage) external {
        _verifyHash(bridgeTransferId, originator, recipient, amount, hashLock, initialTimestamp, nonce);
        require(keccak256(abi.encodePacked(preImage)) == hashLock, InvalidSecret());
        require(bridgeTransfers[bridgeTransferId] == MessageState.PENDING, BridgeTransferStateNotPending());
        require(block.timestamp < initialTimestamp + counterpartyTimeLockDuration, TimeLockExpired());

        bridgeTransfers[bridgeTransferId] = MessageState.COMPLETED;

        nativeBridgeInitiatorMOVE.withdrawMOVE(recipient, amount);

        emit BridgeTransferCompleted(bridgeTransferId, originator, recipient, amount, hashLock, initialTimestamp, nonce, preImage);
    }

    function abortBridgeTransfer(
        bytes32 bridgeTransferId,
        bytes32 originator,
        address recipient,
        uint256 amount,
        bytes32 hashLock,
        uint256 initialTimestamp,
        uint256 nonce) external onlyOwner {
        _verifyHash(bridgeTransferId, originator, recipient, amount, hashLock, initialTimestamp, nonce);
        require(bridgeTransfers[bridgeTransferId] == MessageState.PENDING, BridgeTransferStateNotPending());
        require(block.timestamp > initialTimestamp + counterpartyTimeLockDuration, TimeLockNotExpired());

        bridgeTransfers[bridgeTransferId] = MessageState.REFUNDED;

        emit BridgeTransferAborted(bridgeTransferId);
    }

    function _verifyHash(bytes32 bridgeTransferId,
        bytes32 originator,
        address recipient,
        uint256 amount,
        bytes32 hashLock,
        uint256 initialTimestamp,
        uint256 nonce) internal {
            console.logBytes32(keccak256(abi.encodePacked(originator, recipient, amount, hashLock, initialTimestamp, nonce)));
            require(bridgeTransferId == keccak256(abi.encodePacked(originator, recipient, amount, hashLock, initialTimestamp, nonce)), InvalidBridgeTransferId());
        }
}
