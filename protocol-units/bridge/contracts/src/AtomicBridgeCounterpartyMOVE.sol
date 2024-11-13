// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import {OwnableUpgradeable} from "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import {IAtomicBridgeCounterpartyMOVE} from "./IAtomicBridgeCounterpartyMOVE.sol";
import {AtomicBridgeInitiatorMOVE} from "./AtomicBridgeInitiatorMOVE.sol";
import {RateLimiter} from "./RateLimiter.sol";

contract AtomicBridgeCounterpartyMOVE is IAtomicBridgeCounterpartyMOVE, OwnableUpgradeable {
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

    AtomicBridgeInitiatorMOVE public atomicBridgeInitiatorMOVE;
    RateLimiter public rateLimiter;
    mapping(bytes32 => BridgeTransferDetails) public bridgeTransfers;

    // Configurable time lock duration
    uint256 public counterpartyTimeLockDuration;

    // Initialize with initiator, RateLimiter, owner, and time lock duration
    function initialize(
        address _atomicBridgeInitiator,
        address _rateLimiter,
        address owner,
        uint256 _timeLockDuration
    ) public initializer {
        if (_atomicBridgeInitiator == address(0)) revert ZeroAddress();
        if (_rateLimiter == address(0)) revert ZeroAddress();
        
        atomicBridgeInitiatorMOVE = AtomicBridgeInitiatorMOVE(_atomicBridgeInitiator);
        rateLimiter = RateLimiter(_rateLimiter);
        __Ownable_init(owner);

        counterpartyTimeLockDuration = _timeLockDuration;
    }

    function setAtomicBridgeInitiator(address _atomicBridgeInitiator) external onlyOwner {
        if (_atomicBridgeInitiator == address(0)) revert ZeroAddress();
        atomicBridgeInitiatorMOVE = AtomicBridgeInitiatorMOVE(_atomicBridgeInitiator);
    }

    function setRateLimiter(address _rateLimiter) external onlyOwner {
        if (_rateLimiter == address(0)) revert ZeroAddress();
        rateLimiter = RateLimiter(_rateLimiter);
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
        if (atomicBridgeInitiatorMOVE.poolBalance() < amount) revert InsufficientMOVEBalance();

        bool isWithinRateLimit = rateLimiter.initiateTransfer(amount, RateLimiter.TransferDirection.L2_TO_L1);
        if (!isWithinRateLimit) {
            revert("RATE_LIMIT_EXCEEDED");
        }

        // The time lock is based on the configurable duration
        uint256 timeLock = block.timestamp + counterpartyTimeLockDuration;

        bridgeTransfers[bridgeTransferId] = BridgeTransferDetails({
            recipient: recipient,
            originator: originator,
            amount: amount,
            hashLock: hashLock,
            timeLock: timeLock,
            state: MessageState.PENDING
        });

        emit BridgeTransferLocked(bridgeTransferId, recipient, amount, hashLock, counterpartyTimeLockDuration);
        return true;
    }

    function completeBridgeTransfer(bytes32 bridgeTransferId, bytes32 preImage) external {
        BridgeTransferDetails storage details = bridgeTransfers[bridgeTransferId];
        if (details.state != MessageState.PENDING) revert BridgeTransferStateNotPending();
        bytes32 computedHash = keccak256(abi.encodePacked(preImage));
        if (computedHash != details.hashLock) revert InvalidSecret();
        if (block.timestamp > details.timeLock) revert TimeLockExpired();

        details.state = MessageState.COMPLETED;

        atomicBridgeInitiatorMOVE.withdrawMOVE(details.recipient, details.amount);

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
