// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import {INativeBridgeInitiatorMOVE} from "./INativeBridgeInitiatorMOVE.sol";
import {MockMOVEToken} from "./MockMOVEToken.sol";
import {OwnableUpgradeable} from "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import {ERC20Upgradeable} from "@openzeppelin/contracts-upgradeable/token/ERC20/ERC20Upgradeable.sol";
import {console} from "forge-std/Console.sol";

contract NativeBridgeInitiatorMOVE is INativeBridgeInitiatorMOVE, OwnableUpgradeable {

    enum MessageState {
        NOT_INITIALIZED,
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
    function initialize(address _moveToken, address _owner, uint256 _timeLockDuration)
        public
        initializer
    {
        require(_moveToken != address(0) && _owner != address(0), ZeroAddress());
        moveToken = ERC20Upgradeable(_moveToken);
        __Ownable_init(_owner);

        // Set the custom time lock duration
        initiatorTimeLockDuration = _timeLockDuration;
    }

    function setCounterpartyAddress(address _counterpartyAddress) external onlyOwner {
        require(_counterpartyAddress != address(0), ZeroAddress());
        counterpartyAddress = _counterpartyAddress;
    }

    function initiateBridgeTransfer(bytes32 recipient, uint256 amount, bytes32 hashLock)
        external
        returns (bytes32 bridgeTransferId)
    {
        address originator = msg.sender;

        // Ensure there is a valid amount
        require(amount != 0, ZeroAmount());

        // Transfer the MOVE tokens from the user to the contract
        require(moveToken.transferFrom(originator, address(this), amount),MOVETransferFailed());

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
    ) external {
        _verifyHash(bridgeTransferId, originator, recipient, amount, hashLock, initialTimestamp, nonce);
        require(bridgeTransfers[bridgeTransferId] == MessageState.INITIALIZED, BridgeTransferNotInitialized());
        require(keccak256(abi.encodePacked(preImage)) == hashLock,InvalidSecret());
        require(block.timestamp < initialTimestamp + initiatorTimeLockDuration, TimelockExpired());
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
        _verifyHash(bridgeTransferId, originator, recipient, amount, hashLock, initialTimestamp, nonce);
        require(bridgeTransfers[bridgeTransferId] == MessageState.INITIALIZED, BridgeTransferNotInitialized());
        require(block.timestamp > initialTimestamp + initiatorTimeLockDuration, TimeLockNotExpired());
        bridgeTransfers[bridgeTransferId] = MessageState.REFUNDED;
        require(moveToken.transfer(originator, amount), MOVETransferFailed());

        emit BridgeTransferRefunded(bridgeTransferId);
    }

    function withdrawMOVE(address recipient, uint256 amount) external {
        require(msg.sender == counterpartyAddress, Unauthorized());
        require(moveToken.transfer(recipient, amount), MOVETransferFailed());
    }

    function _verifyHash(bytes32 bridgeTransferId,
        address originator,
        bytes32 recipient,
        uint256 amount,
        bytes32 hashLock,
        uint256 initialTimestamp,
        uint256 nonce) internal {
            console.logBytes32(keccak256(abi.encodePacked(originator, recipient, amount, hashLock, initialTimestamp, nonce)));
            require(bridgeTransferId == keccak256(abi.encodePacked(originator, recipient, amount, hashLock, initialTimestamp, nonce)), InvalidBridgeTransferId());
    }
}
