// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import {AccessControlUpgradeable} from "@openzeppelin/contracts-upgradeable/access/AccessControlUpgradeable.sol";
import {IAtomicBridgeInitiatorMOVE} from "./IAtomicBridgeInitiatorMOVE.sol";
import {ERC20Upgradeable} from "@openzeppelin/contracts-upgradeable/token/ERC20/ERC20Upgradeable.sol";

contract AtomicBridgeInitiatorMOVE is IAtomicBridgeInitiatorMOVE, AccessControlUpgradeable {
    enum MessageState {
        INITIALIZED,
        COMPLETED,
        REFUNDED
    }

    struct BridgeTransfer {
        address originator;
        bytes32 recipient;
        uint256 amount;
        bytes32 hashLock;
        uint256 timeLock;
        MessageState state;
    }

    // Mapping of bridge transfer ids to BridgeTransfer structs
    mapping(bytes32 => BridgeTransfer) public bridgeTransfers;

    address public counterpartyAddress;
    ERC20Upgradeable public moveToken;
    uint256 private nonce;

    // Configurable time lock duration
    uint256 public initiatorTimeLockDuration;

    // Prevents initialization of implementation contract exploits
    constructor(){_disableInitializers();}

    // Initialize the contract with MOVE token address, owner, and custom time lock duration
    function initialize(
        address _moveToken,
        address _owner,
        uint256 _timeLockDuration
    ) public initializer {
        if (_moveToken == address(0) && owner == address(0)) {
            revert ZeroAddress();
        }
        if (_timeLockDuration == 0) {
            revert ZeroValue();
        }
        moveToken = ERC20Upgradeable(_moveToken);
        grantRole(DEFAULT_ADMIN_ROLE, _owner);

        // Set the custom time lock duration
        initiatorTimeLockDuration = _timeLockDuration;
    }

    function setCounterpartyAddress(address _counterpartyAddress) external onlyRole(ADMIN_ROLE) {
        if (_counterpartyAddress == address(0)) revert ZeroAddress();
        counterpartyAddress = _counterpartyAddress;
    }

    function setTimeLockDuration(uint256 _timeLockDuration) external onlyRole(ADMIN_ROLE) {
        initiatorTimeLockDuration = _timeLockDuration;
    }

    function initiateBridgeTransfer(uint256 moveAmount, bytes32 recipient, bytes32 hashLock)
        external
        returns (bytes32 bridgeTransferId)
    {
        address originator = msg.sender;

        // Ensure there is a valid amount
        if (moveAmount == 0) {
            revert ZeroAmount();
        }

        // Transfer the MOVE tokens from the user to the contract
        if (!moveToken.transferFrom(originator, address(this), moveAmount)) {
            revert MOVETransferFailed();
        }

        // Generate a unique nonce to prevent replay attacks, and generate a transfer ID
        bridgeTransferId = keccak256(abi.encodePacked(originator, recipient, hashLock, initiatorTimeLockDuration, block.timestamp, nonce++));

        bridgeTransfers[bridgeTransferId] = BridgeTransfer({
            originator: originator,
            recipient: recipient,
            amount: moveAmount,
            hashLock: hashLock,
            timeLock: block.timestamp + initiatorTimeLockDuration,
            state: MessageState.INITIALIZED
        });

        emit BridgeTransferInitiated(bridgeTransferId, originator, recipient, moveAmount, hashLock, initiatorTimeLockDuration);
        return bridgeTransferId;
    }

    function completeBridgeTransfer(bytes32 bridgeTransferId, bytes32 preImage) external {
        BridgeTransfer storage bridgeTransfer = bridgeTransfers[bridgeTransferId];
        if (bridgeTransfer.state != MessageState.INITIALIZED) revert BridgeTransferHasBeenCompleted();
        if (keccak256(abi.encodePacked(preImage)) != bridgeTransfer.hashLock) revert InvalidSecret();
        if (block.timestamp > bridgeTransfer.timeLock) revert TimelockExpired();
        bridgeTransfer.state = MessageState.COMPLETED;

        emit BridgeTransferCompleted(bridgeTransferId, preImage);
    }

    function refundBridgeTransfer(bytes32 bridgeTransferId) external onlyRole(REFUNDER_ROLE) {
        BridgeTransfer storage bridgeTransfer = bridgeTransfers[bridgeTransferId];
        if (bridgeTransfer.state != MessageState.INITIALIZED) revert BridgeTransferStateNotInitialized();
        if (block.timestamp < bridgeTransfer.timeLock) revert TimeLockNotExpired();
        bridgeTransfer.state = MessageState.REFUNDED;

        if (!moveToken.transfer(bridgeTransfer.originator, bridgeTransfer.amount)) revert MOVETransferFailed();

        emit BridgeTransferRefunded(bridgeTransferId);
    }

    function withdrawMOVE(address recipient, uint256 amount) external {
        if (msg.sender != counterpartyAddress) revert Unauthorized();
        if (!moveToken.transfer(recipient, amount)) revert MOVETransferFailed();
    }
}

