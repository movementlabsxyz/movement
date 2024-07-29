// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import {OwnableUpgradeable} from "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import {IAtomicBridgeCounterparty} from "./IAtomicBridgeCounterparty.sol";
import {AtomicBridgeInitiator} from "./AtomicBridgeInitator.sol";

contract AtomicBridgeCounterparty is IAtomicBridgeCounterparty, OwnableUpgradeable {
    enum MessageState {
        PENDING,
        COMPLETED,
        REFUNDED
    }

    struct BridgeTransferDetails {
        bytes32 initiator; 
        address recipient;
        uint256 amount;
        bytes32 hashLock;
        uint256 timeLock;
        MessageState state; 
    }

    AtomicBridgeInitiator public atomicBridgeInitiator;
    mapping(bytes32 => BridgeTransferDetails) public bridgeTransfers; 

    function initialize(address _atomicBridgeInitiator, address owner) public initializer {
        if (_atomicBridgeInitiator == address(0)) revert ZeroAddress();
        atomicBridgeInitiator = AtomicBridgeInitiator(_atomicBridgeInitiator);
        __Ownable_init(owner);
    }

    function setAtomicBridgeInitiator(address _atomicBridgeInitiator) external onlyOwner {
        if (_atomicBridgeInitiator == address(0)) revert ZeroAddress();
        atomicBridgeInitiator = AtomicBridgeInitiator(_atomicBridgeInitiator);
    }

    function lockBridgeTransferAssets(
        bytes32 initiator,
        bytes32 bridgeTransferId,
        bytes32 hashLock,
        uint256 timeLock,
        address recipient,
        uint256 amount
    ) external onlyOwner returns (bool) {
        if (recipient == address(0)) revert BridgeTransferInvalid();
        if (amount == 0) revert ZeroAmount();

        bridgeTransfers[bridgeTransferId] = BridgeTransferDetails({
            recipient: recipient,
            initiator: initiator,
            amount: amount,
            hashLock: hashLock,
            timeLock: block.timestamp + timeLock,
            state: MessageState.PENDING 
        });

        emit BridgeTransferAssetsLocked(bridgeTransferId, recipient, amount, hashLock, timeLock);
        return true;
    }

    function completeBridgeTransfer(bytes32 bridgeTransferId, bytes32 preImage) external {
        BridgeTransferDetails storage details = bridgeTransfers[bridgeTransferId];
        if (details.state != MessageState.PENDING) revert BridgeTransferInvalid();
        bytes32 computedHash = keccak256(abi.encodePacked(preImage));
        if (computedHash != details.hashLock) revert InvalidSecret();
        if (block.timestamp > details.timeLock) revert TimeLockNotExpired();

        details.state = MessageState.COMPLETED;

        atomicBridgeInitiator.withdrawWETH(details.recipient, details.amount);

        emit BridgeTransferCompleted(bridgeTransferId, preImage);
    }

    function abortBridgeTransfer(bytes32 bridgeTransferId) external {
        BridgeTransferDetails storage details = bridgeTransfers[bridgeTransferId];
        if (details.state != MessageState.PENDING) revert BridgeTransferInvalid();
        if (block.timestamp <= details.timeLock) revert TimeLockNotExpired();

        details.state = MessageState.REFUNDED;

        emit BridgeTransferCancelled(bridgeTransferId);
    }
}

