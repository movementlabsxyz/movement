// SPDX-License-Identifier: MIT
pragma solidity ^0.8.27;

import {OwnableUpgradeable} from "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import {IAtomicBridgeCounterpartyMOVE} from "./IAtomicBridgeCounterpartyMOVE.sol";
import {AtomicBridgeInitiatorMOVE} from "./AtomicBridgeInitiatorMOVE.sol";

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
    mapping(bytes32 => BridgeTransferDetails) public bridgeTransfers;

    // Configurable time lock duration
    uint256 public counterpartyTimeLockDuration;

    function initialize(address _atomicBridgeInitiator, address owner, uint256 _timeLockDuration) public initializer {
        require(_atomicBridgeInitiator != address(0), ZeroAddress());
        atomicBridgeInitiatorMOVE = AtomicBridgeInitiatorMOVE(_atomicBridgeInitiator);
        __Ownable_init(owner);

        // Set the configurable time lock duration
        counterpartyTimeLockDuration = _timeLockDuration;
    }

    function setAtomicBridgeInitiator(address _atomicBridgeInitiator) external onlyOwner {
        require(_atomicBridgeInitiator != address(0), ZeroAddress());
        atomicBridgeInitiatorMOVE = AtomicBridgeInitiatorMOVE(_atomicBridgeInitiator);
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
        require(amount > 0, ZeroAmount());
        require(atomicBridgeInitiatorMOVE.poolBalance() >= amount, InsufficientMOVEBalance());

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

        emit BridgeTransferLocked(bridgeTransferId, recipient, amount, hashLock, counterpartyTimeLockDuration);
        return true;
    }

    function completeBridgeTransfer(bytes32 bridgeTransferId, bytes32 preImage) external {
        BridgeTransferDetails storage details = bridgeTransfers[bridgeTransferId];
        require(details.state == MessageState.PENDING, BridgeTransferStateNotPending());
        bytes32 computedHash = keccak256(abi.encodePacked(preImage));
        require(computedHash == details.hashLock, InvalidSecret());
        require(block.timestamp <= details.timeLock, TimeLockExpired());

        details.state = MessageState.COMPLETED;

        atomicBridgeInitiatorMOVE.withdrawMOVE(details.recipient, details.amount);

        emit BridgeTransferCompleted(bridgeTransferId, preImage);
    }

    function abortBridgeTransfer(bytes32 bridgeTransferId) external onlyOwner {
        BridgeTransferDetails storage details = bridgeTransfers[bridgeTransferId];
        require(details.state == MessageState.PENDING, BridgeTransferStateNotPending());
        require(block.timestamp > details.timeLock, TimeLockNotExpired());

        details.state = MessageState.REFUNDED;

        emit BridgeTransferAborted(bridgeTransferId);
    }
}
