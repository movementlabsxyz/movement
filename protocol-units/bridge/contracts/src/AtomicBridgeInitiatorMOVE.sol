// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import {IAtomicBridgeInitiatorMOVE} from "./IAtomicBridgeInitiatorMOVE.sol";
import {MockMOVEToken} from "./MockMOVEToken.sol";
import {OwnableUpgradeable} from "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import {ERC20Upgradeable} from "@openzeppelin/contracts-upgradeable/token/ERC20/ERC20Upgradeable.sol";
import {RateLimiter} from "./RateLimiter.sol";

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
        uint256 timeLock; // in seconds (timestamp)
        MessageState state;
    }

    // Mapping of bridge transfer ids to BridgeTransfer structs
    mapping(bytes32 => BridgeTransfer) public bridgeTransfers;

    // Total MOVE token pool balance
    uint256 public poolBalance;

    address public counterpartyAddress;
    RateLimiter public rateLimiter;
    ERC20Upgradeable public moveToken;
    uint256 private nonce;

    // Configurable time lock duration
    uint256 public initiatorTimeLockDuration;

    // Initialize the contract with MOVE token address, owner, custom time lock duration, and initial pool balance
    function initialize(
        address _moveToken,
        address owner,
        uint256 _timeLockDuration,
        uint256 _initialPoolBalance
    ) public initializer {
        require(_moveToken != address(0) && owner != address(0), "ZeroAddress");
        moveToken = ERC20Upgradeable(_moveToken);
        __Ownable_init(owner);

        // Set the custom time lock duration
        initiatorTimeLockDuration = _timeLockDuration;

        // Set the initial pool balance
        poolBalance = _initialPoolBalance;
    }

    function setCounterpartyAddress(address _counterpartyAddress) external onlyOwner {
        require(_counterpartyAddress != address(0), "ZeroAddress");
        counterpartyAddress = _counterpartyAddress;
    }

    function setRateLimiter(address _rateLimiter) external onlyOwner {
        if (_rateLimiter == address(0)) revert ZeroAddress();
        rateLimiter = RateLimiter(_rateLimiter);
    }

    function initiateBridgeTransfer(uint256 moveAmount, bytes32 recipient, bytes32 hashLock)
        external
        returns (bytes32 bridgeTransferId)
    {
        rateLimiter.rateLimitOutbound(moveAmount);
        address originator = msg.sender;
            
        require(moveAmount > 0, "ZeroAmount");

        // Ensure there is a valid amount
        if (moveAmount == 0) {
            revert ZeroAmount();
        }

        // Transfer the MOVE tokens from the user to the contract
        if (!moveToken.transferFrom(originator, address(this), moveAmount)) {
            revert MOVETransferFailed();
        }

        // Update the pool balance
        poolBalance += moveAmount;

        // Generate a unique nonce to prevent replay attacks, and generate a transfer ID
        bridgeTransferId = keccak256(abi.encodePacked(originator, recipient, hashLock, initiatorTimeLockDuration, block.timestamp, nonce++));

        bridgeTransfers[bridgeTransferId] = BridgeTransfer({
            amount: moveAmount,
            originator: originator,
            recipient: recipient,
            hashLock: hashLock,
            timeLock: block.timestamp + initiatorTimeLockDuration,
            state: MessageState.INITIALIZED
        });

        emit BridgeTransferInitiated(bridgeTransferId, originator, recipient, moveAmount, hashLock, initiatorTimeLockDuration);
        return bridgeTransferId;
    }

    function completeBridgeTransfer(bytes32 bridgeTransferId, bytes32 preImage) external onlyOwner {
        BridgeTransfer storage bridgeTransfer = bridgeTransfers[bridgeTransferId];

        rateLimiter.rateLimitInbound(bridgeTransfer.amount);
        require(bridgeTransfer.state == MessageState.INITIALIZED, "BridgeTransferHasBeenCompleted");
        require(keccak256(abi.encodePacked(preImage)) == bridgeTransfer.hashLock, "InvalidSecret");
        require(block.timestamp <= bridgeTransfer.timeLock, "TimelockExpired");

        bridgeTransfer.state = MessageState.COMPLETED;

        emit BridgeTransferCompleted(bridgeTransferId, preImage);
    }

    function refundBridgeTransfer(bytes32 bridgeTransferId) external onlyOwner {
        BridgeTransfer storage bridgeTransfer = bridgeTransfers[bridgeTransferId];
        rateLimiter.rateLimitInbound(bridgeTransfer.amount);
        require(bridgeTransfer.state == MessageState.INITIALIZED, "BridgeTransferStateNotInitialized");
        require(block.timestamp >= bridgeTransfer.timeLock, "TimeLockNotExpired");

        bridgeTransfer.state = MessageState.REFUNDED;

        if (!moveToken.transfer(bridgeTransfer.originator, bridgeTransfer.amount)) revert("MOVETransferFailed");

        emit BridgeTransferRefunded(bridgeTransferId);
    }

    function withdrawMOVE(address recipient, uint256 amount) external {
        if (msg.sender != counterpartyAddress) revert Unauthorized();
        if (poolBalance < amount) revert InsufficientMOVEBalance();
        poolBalance -= amount;
        if (!moveToken.transfer(recipient, amount)) revert MOVETransferFailed();
    }
}
