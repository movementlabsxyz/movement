// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import {AccessControlUpgradeable} from "@openzeppelin/contracts-upgradeable/access/AccessControlUpgradeable.sol";
import {IAtomicBridgeCounterpartyMOVE} from "./IAtomicBridgeCounterpartyMOVE.sol";
import {AtomicBridgeInitiatorMOVE} from "./AtomicBridgeInitiatorMOVE.sol";
import {console} from "forge-std/console.sol";

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
    bytes32 constant ADMIN_ROLE = keccak256("ADMIN_ROLE");
    bytes32 constant RELAYER_ROLE = keccak256("RELAYER_ROLE");
    bytes32 constant REFUNDER_ROLE = keccak256("REFUNDER_ROLE");

    // Configurable time lock duration
    uint256 public counterpartyTimeLockDuration;

    // Prevents initialization of implementation contract exploits
    constructor() {
        _disableInitializers();
    }

    function initialize(
        address _atomicBridgeInitiator,
        address _owner,
        address _admin,
        address _relayer,
        address _refunder,
        uint256 _timeLockDuration
    ) public initializer {
        if (
            _atomicBridgeInitiator == address(0) && _owner == address(0) && _admin == address(0)
                && _relayer == address(0) && _refunder == address(0)
        ) revert ZeroAddress();
        if (_timeLockDuration == 0) revert ZeroValue();
        _grantRole(DEFAULT_ADMIN_ROLE, _owner);
        _grantRole(ADMIN_ROLE, _admin);
        _grantRole(RELAYER_ROLE, _relayer);
        _grantRole(REFUNDER_ROLE, _refunder);

        atomicBridgeInitiatorMOVE = AtomicBridgeInitiatorMOVE(_atomicBridgeInitiator);
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
        console.log("ICI LOCK timeLock:%d", timeLock);

        bridgeTransfers[bridgeTransferId] = BridgeTransferDetails({
            recipient: recipient,
            originator: originator,
            amount: amount,
            hashLock: hashLock,
            timeLock: timeLock,
            state: MessageState.PENDING
        });
        console.log("ICI LOCK done");

        emit BridgeTransferLocked(bridgeTransferId, recipient, amount, hashLock, counterpartyTimeLockDuration);
    }

    function completeBridgeTransfer(bytes32 bridgeTransferId, bytes32 preImage) external {
        BridgeTransferDetails storage details = bridgeTransfers[bridgeTransferId];
        // if (details.state != MessageState.PENDING) revert BridgeTransferStateNotPending();
        bytes32 computedHash = keccak256(abi.encodePacked(preImage));
        // if (computedHash != details.hashLock) revert InvalidSecret();
        // if (block.timestamp > details.timeLock) revert TimeLockExpired();

        details.state = MessageState.COMPLETED;

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
