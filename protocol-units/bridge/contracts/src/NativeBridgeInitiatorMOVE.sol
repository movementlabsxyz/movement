// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import {INativeBridgeInitiatorMOVE} from "./INativeBridgeInitiatorMOVE.sol";
import {MockMOVEToken} from "./MockMOVEToken.sol";
import {OwnableUpgradeable} from "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import {ERC20Upgradeable} from "@openzeppelin/contracts-upgradeable/token/ERC20/ERC20Upgradeable.sol";

contract NativeBridgeInitiatorMOVE is INativeBridgeInitiatorMOVE, OwnableUpgradeable {

    enum MessageState {
        INITIALIZED,
        COMPLETED,
        REFUNDED
    }
    // Mapping of bridge transfer ids to BridgeTransfer structs
    mapping(bytes32 => MessageState) public bridgeTransfers;

    address public counterpartyAddress;
    ERC20Upgradeable public moveToken;
    uint256 private _nonce;

    // Configurable time lock duration
    uint256 public initiatorTimeLockDuration;

    // Initialize the contract with MOVE token address, owner, custom time lock duration, and initial pool balance
    function initialize(address _moveToken, address owner, uint256 _timeLockDuration, uint256 _initialPoolBalance)
        public
        initializer
    {
        if (_moveToken == address(0)) {
            revert ZeroAddress();
        }
        moveToken = ERC20Upgradeable(_moveToken);
        __Ownable_init(owner);

        // Set the custom time lock duration
        initiatorTimeLockDuration = _timeLockDuration;
    }

    function setCounterpartyAddress(address _counterpartyAddress) external onlyOwner {
        if (_counterpartyAddress == address(0)) revert ZeroAddress();
        counterpartyAddress = _counterpartyAddress;
    }

    function initiateBridgeTransfer(bytes32 recipient, uint256 amount, bytes32 hashLock)
        external
        returns (bytes32 bridgeTransferId)
    {
        address originator = msg.sender;

        // Ensure there is a valid amount
        if (amount == 0) {
            revert ZeroAmount();
        }

        // Transfer the MOVE tokens from the user to the contract
        if (!moveToken.transferFrom(originator, address(this), amount)) {
            revert MOVETransferFailed();
        }

        // Generate a unique nonce to prevent replay attacks, and generate a transfer ID
        bridgeTransferId =
            keccak256(abi.encodePacked(originator, recipient, amount, hashLock, block.timestamp, ++_nonce));

        bridgeTransfers[bridgeTransferId] = MessageState.INITIALIZED;

        emit BridgeTransferInitiated(bridgeTransferId, originator, recipient, amount, hashLock, block.timestamp, _nonce);
        return bridgeTransferId;
    }

    function completeBridgeTransfer(
        bytes32 bridgeTransferId,
        address originator,
        bytes32 recipient,
        uint256 amount,
        bytes32 hashLock,
        uint256 initialTimestamp,
        uint256 nonce,
        bytes32 preImage
    ) external onlyOwner {
        require(bridgeTransfers[bridgeTransferId] == MessageState.INITIALIZED, BridgeTransferHasBeenCompleted());
        require(
            bridgeTransferId
                == keccak256(abi.encodePacked(originator, recipient, amount, hashLock, initialTimestamp, nonce)),
            InvalidBridgeTransferId()
        );
        if (keccak256(abi.encodePacked(preImage)) != hashLock) revert InvalidSecret();
        if (block.timestamp > initialTimestamp + initiatorTimeLockDuration) revert TimelockExpired();
        bridgeTransfers[bridgeTransferId] = MessageState.COMPLETED;

        emit BridgeTransferCompleted(
            bridgeTransferId, originator, recipient, amount, hashLock, initialTimestamp, nonce, preImage
        );
    }

    function refundBridgeTransfer(
        bytes32 bridgeTransferId,
        address originator,
        bytes32 recipient,
        uint256 amount,
        bytes32 hashLock,
        uint256 initialTimestamp,
        uint256 nonce
    ) external onlyOwner {
        require(
            bridgeTransferId
                == keccak256(abi.encodePacked(originator, recipient, amount, hashLock, initialTimestamp, nonce)),
            InvalidBridgeTransferId()
        );

        require(bridgeTransfers[bridgeTransferId] == MessageState.INITIALIZED, BridgeTransferHasBeenCompleted());

        if (block.timestamp < initialTimestamp + initiatorTimeLockDuration) revert TimeLockNotExpired();
        bridgeTransfers[bridgeTransferId] = MessageState.REFUNDED;
        if (!moveToken.transfer(originator, amount)) revert MOVETransferFailed();

        emit BridgeTransferRefunded(bridgeTransferId);
    }

    function withdrawMOVE(address recipient, uint256 amount) external {
        if (msg.sender != counterpartyAddress) revert Unauthorized();
        if (!moveToken.transfer(recipient, amount)) revert MOVETransferFailed();
    }
}
