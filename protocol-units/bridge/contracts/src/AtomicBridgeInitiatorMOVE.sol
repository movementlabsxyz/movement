// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import {IAtomicBridgeInitiatorMOVE} from "./IAtomicBridgeInitiatorMOVE.sol";
import {MOVEToken} from "./MOVEToken.sol";
import {OwnableUpgradeable} from "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import {ERC20Upgradeable} from "@openzeppelin/contracts-upgradeable/token/ERC20/ERC20Upgradeable.sol";

contract AtomicBridgeInitiatorMOVE is IAtomicBridgeInitiatorMOVE, OwnableUpgradeable {
    enum MessageState {
        INITIALIZED,
        COMPLETED,
        REFUNDED
    }   

    struct BridgeTransfer {
        uint256 amount;
        address originator;
        bytes32 recipient;
        bytes32 hashLock;
        uint256 timeLock; // in blocks
        MessageState state;
    }

    // Mapping of bridge transfer ids to BridgeTransfer structs
    mapping(bytes32 => BridgeTransfer) public bridgeTransfers;

    // Total WETH pool balance
    uint256 public poolBalance;

    address public counterpartyAddress; 
    ERC20Upgradeable public moveToken;
    uint256 private nonce;

    error InsufficientMOVEBalance();
    error MOVETransferFailed();

    function initialize(address _moveToken, address owner) public initializer {
        if (_moveToken == address(0)) {
            revert ZeroAddress();
        }
        moveToken = ERC20Upgradeable(_moveToken);
        __Ownable_init(owner);
    }

    function setCounterpartyAddress(address _counterpartyAddress) external onlyOwner {
        if (_counterpartyAddress == address(0)) revert ZeroAddress();
        counterpartyAddress = _counterpartyAddress;
    }

    function initiateBridgeTransfer(uint256 moveAmount, bytes32 recipient, bytes32 hashLock, uint256 timeLock)
        external
        returns (bytes32 bridgeTransferId)
    {
        address originator = msg.sender;

        // Ensure there is a valid amount
        if (moveAmount == 0) {
            revert ZeroAmount();
        }

        // Transfer MOVE tokens from the user to the contract
        if (!moveToken.transferFrom(originator, address(this), moveAmount)) {
            revert MOVETransferFailed();
        }

        // Update the pool balance
        poolBalance += moveAmount;

        // Generate a unique nonce to prevent replay attacks, and generate a transfer ID
        bridgeTransferId = keccak256(abi.encodePacked(originator, recipient, hashLock, timeLock, block.number, nonce++));

        bridgeTransfers[bridgeTransferId] = BridgeTransfer({
            amount: moveAmount,
            originator: originator,
            recipient: recipient,
            hashLock: hashLock,
            timeLock: block.number + timeLock,
            state: MessageState.INITIALIZED
        });

        emit BridgeTransferInitiated(bridgeTransferId, originator, recipient, moveAmount, hashLock, timeLock);
        return bridgeTransferId;
    }

    function completeBridgeTransfer(bytes32 bridgeTransferId, bytes32 preImage) external {
        BridgeTransfer storage bridgeTransfer = bridgeTransfers[bridgeTransferId];
        if (bridgeTransfer.state != MessageState.INITIALIZED) revert BridgeTransferHasBeenCompleted();
        if (keccak256(abi.encodePacked(preImage)) != bridgeTransfer.hashLock) revert InvalidSecret();
        if (block.number > bridgeTransfer.timeLock) revert TimelockExpired();
        bridgeTransfer.state = MessageState.COMPLETED;

        emit BridgeTransferCompleted(bridgeTransferId, preImage);
    }

    function refundBridgeTransfer(bytes32 bridgeTransferId) external onlyOwner {
        BridgeTransfer storage bridgeTransfer = bridgeTransfers[bridgeTransferId];
        if (bridgeTransfer.state != MessageState.INITIALIZED) revert BridgeTransferStateNotInitialized();
        if (block.number < bridgeTransfer.timeLock) revert TimeLockNotExpired();
        bridgeTransfer.state = MessageState.REFUNDED;
        
        // Decrease pool balance and transfer MOVE tokens back to the originator
        poolBalance -= bridgeTransfer.amount;
        if (!moveToken.transfer(bridgeTransfer.originator, bridgeTransfer.amount)) revert MOVETransferFailed();

        emit BridgeTransferRefunded(bridgeTransferId);
    }

    function withdrawMOVE(address recipient, uint256 amount) external {
        if (msg.sender != counterpartyAddress) revert Unauthorized();
        if (poolBalance < amount) revert InsufficientMOVEBalance();
        poolBalance -= amount;
        if (!moveToken.transfer(recipient, amount)) revert MOVETransferFailed();
    }
}
