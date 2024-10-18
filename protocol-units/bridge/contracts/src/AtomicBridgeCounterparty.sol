// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import {OwnableUpgradeable} from "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import {IAtomicBridgeCounterparty} from "./IAtomicBridgeCounterparty.sol";
import {AtomicBridgeInitiator} from "./AtomicBridgeInitiator.sol";
import {console} from "forge-std/console.sol";

contract AtomicBridgeCounterparty is IAtomicBridgeCounterparty, OwnableUpgradeable {
    enum MessageState {
        PENDING,
        COMPLETED,
        REFUNDED
    }

    struct BridgeTransferDetails {
        bytes32 originator;
        address recipient;
        uint256 amount;
        bytes32 hashLock;
        uint256 timeLock;
        MessageState state;
    }

    // Reference to the AtomicBridgeInitiator contract
    AtomicBridgeInitiator public atomicBridgeInitiator;
    mapping(bytes32 => BridgeTransferDetails) public bridgeTransfers;

    // Configurable time lock duration
    uint256 public counterpartyTimeLockDuration;

    function initialize(address _atomicBridgeInitiator, address owner, uint256 _timeLockDuration) public initializer {
        if (_atomicBridgeInitiator == address(0)) revert ZeroAddress();
        atomicBridgeInitiator = AtomicBridgeInitiator(_atomicBridgeInitiator);
        __Ownable_init(owner);

        // Set the configurable time lock duration
        counterpartyTimeLockDuration = _timeLockDuration;
    }

    function setAtomicBridgeInitiator(address _atomicBridgeInitiator) external onlyOwner {
        if (_atomicBridgeInitiator == address(0)) revert ZeroAddress();
        atomicBridgeInitiator = AtomicBridgeInitiator(_atomicBridgeInitiator);
    }

    function setTimeLockDuration(uint256 _timeLockDuration) external onlyOwner {
        counterpartyTimeLockDuration = _timeLockDuration;
    }

    function lockBridgeTransfer(
        bytes32 originator,
        bytes32 bridgeTransferId,
        bytes32 hashLock,
        address recipient,
        uint256 amount
    ) external onlyOwner returns (bool) {
        if (amount == 0) revert ZeroAmount();

        if (atomicBridgeInitiator.poolBalance() < amount) revert InsufficientWethBalance();
        
        // potentially mint some gas here for the recipient here. The recipient could be an account with gas already.

        // The time lock is now based on the configurable duration
        uint256 timeLock = block.timestamp + counterpartyTimeLockDuration;

        bridgeTransfers[bridgeTransferId] = BridgeTransferDetails({
            recipient: recipient,
            originator: originator,
            amount: amount,
            hashLock: hashLock,
            timeLock: timeLock,
            state: MessageState.PENDING
        });

        emit BridgeTransferLocked(bridgeTransferId, originator, recipient, amount, hashLock, counterpartyTimeLockDuration);
        return true;
    }

    function completeBridgeTransfer(bytes32 bridgeTransferId, bytes32 preImage) external {
        BridgeTransferDetails storage details = bridgeTransfers[bridgeTransferId];
        if (details.state != MessageState.PENDING) revert BridgeTransferStateNotPending();
        bytes32 computedHash = keccak256(abi.encodePacked(preImage));
        if (computedHash != details.hashLock) revert InvalidSecret();
        if (block.timestamp > details.timeLock) revert TimeLockExpired();

        details.state = MessageState.COMPLETED;

        atomicBridgeInitiator.withdrawWETH(details.recipient, details.amount);

        emit BridgeTransferCompleted(bridgeTransferId, preImage);
    }

    function abortBridgeTransfer(bytes32 bridgeTransferId) external onlyOwner {
        BridgeTransferDetails storage details = bridgeTransfers[bridgeTransferId];
        if (details.state != MessageState.PENDING) revert BridgeTransferStateNotPending();
        if (block.timestamp <= details.timeLock) revert TimeLockNotExpired();

        details.state = MessageState.REFUNDED;

        emit BridgeTransferAborted(bridgeTransferId);
    }
}
