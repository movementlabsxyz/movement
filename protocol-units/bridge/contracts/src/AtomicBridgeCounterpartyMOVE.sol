// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import {AccessControlUpgradeable} from "@openzeppelin/contracts-upgradeable/access/AccessControlUpgradeable.sol";
import {IAtomicBridgeCounterpartyMOVE} from "./IAtomicBridgeCounterpartyMOVE.sol";
import {AtomicBridgeInitiatorMOVE} from "./AtomicBridgeInitiatorMOVE.sol";

contract AtomicBridgeCounterpartyMOVE is IAtomicBridgeCounterpartyMOVE, AccessControlUpgradeable {
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

    // Prevents initialization of implementation contract exploits
    constructor(){_disableInitializers();}

    function initialize(address _atomicBridgeInitiator, address _owner, address _relayer, uint256 _timeLockDuration) public initializer {
        if (_atomicBridgeInitiator == address(0) && _owner == address(0)) revert ZeroAddress();
        if (_timeLockDuration == 0) revert ZeroValue();
        atomicBridgeInitiatorMOVE = AtomicBridgeInitiatorMOVE(_atomicBridgeInitiator);
        grantRole(DEFAULT_ADMIN_ROLE, _owner);
        grantRole(RELAYER_ROLE, _relayer);

        // Set the configurable time lock duration
        counterpartyTimeLockDuration = _timeLockDuration;
    }

    function setAtomicBridgeInitiator(address _atomicBridgeInitiator) external onlyRole(ADMIN_ROLE) {
        if (_atomicBridgeInitiator == address(0)) revert ZeroAddress();
        atomicBridgeInitiatorMOVE = AtomicBridgeInitiatorMOVE(_atomicBridgeInitiator);
    }

    function setTimeLockDuration(uint256 _timeLockDuration) external onlyRole(ADMIN_ROLE) {
        counterpartyTimeLockDuration = _timeLockDuration;
    }

    function lockBridgeTransfer(
        bytes32 originator,
        bytes32 bridgeTransferId,
        bytes32 hashLock,
        address recipient,
        uint256 amount
    ) external onlyRole(RELAYER_ROLE) {
        if (amount == 0) revert ZeroAmount();
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

    function abortBridgeTransfer(bytes32 bridgeTransferId) external onlyRole(REFUNDER_ROLE) {
        BridgeTransferDetails storage details = bridgeTransfers[bridgeTransferId];
        if (details.state != MessageState.PENDING) revert BridgeTransferStateNotPending();
        if (block.timestamp <= details.timeLock) revert TimeLockNotExpired();

        details.state = MessageState.REFUNDED;

        emit BridgeTransferAborted(bridgeTransferId);
    }
}
