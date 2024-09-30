pragma solidity ^0.8.22;

import {IAtomicBridgeInitiator} from "./IAtomicBridgeInitiator.sol";
import {IWETH9} from "./IWETH9.sol";
import {OwnableUpgradeable} from "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";

contract AtomicBridgeInitiator is IAtomicBridgeInitiator, OwnableUpgradeable {
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
    mapping(bytes32 => BridgeTransfer> public bridgeTransfers;

    // Total WETH pool balance
    uint256 public poolBalance;

    address public counterpartyAddress;
    IWETH9 public weth;
    uint256 private nonce;

    // Configurable time lock duration
    uint256 public initiatorTimeLockDuration;

    // State variable to track if the one-shot function was called
    bool private oneShotCalled;

    // Initialize the contract with WETH address, owner, and a custom time lock duration
    function initialize(address _weth, address owner, uint256 _timeLockDuration) public initializer {
        if (_weth == address(0)) {
            revert ZeroAddress();
        }
        weth = IWETH9(_weth);
        __Ownable_init(owner);

        // Set the custom time lock duration
        initiatorTimeLockDuration = _timeLockDuration;

        // Initialize the one-shot flag to false
        oneShotCalled = false;
    }

    function setCounterpartyAddress(address _counterpartyAddress) external onlyOwner {
        if (_counterpartyAddress == address(0)) revert ZeroAddress();
        counterpartyAddress = _counterpartyAddress;
    }

    function initiateBridgeTransfer(uint256 wethAmount, bytes32 recipient, bytes32 hashLock)
        external
        payable
        returns (bytes32 bridgeTransferId)
    {
        address originator = msg.sender;
        uint256 ethAmount = msg.value;
        uint256 totalAmount = wethAmount + ethAmount;

        // Ensure there is a valid total amount
        if (totalAmount == 0) {
            revert ZeroAmount();
        }
        
        // If msg.value is greater than 0, convert ETH to WETH
        if (ethAmount > 0) weth.deposit{value: ethAmount}();
        
        // Transfer WETH to this contract, revert if transfer fails
        if (wethAmount > 0) {
            if (!weth.transferFrom(originator, address(this), wethAmount)) revert WETHTransferFailed();
        }

        // Update the pool balance
        poolBalance += totalAmount;

        // Generate a unique nonce to prevent replay attacks, and generate a transfer ID
        bridgeTransferId = keccak256(abi.encodePacked(originator, recipient, hashLock, initiatorTimeLockDuration, block.timestamp, nonce++));

        bridgeTransfers[bridgeTransferId] = BridgeTransfer({
            amount: totalAmount,
            originator: originator,
            recipient: recipient,
            hashLock: hashLock,
            timeLock: block.timestamp + initiatorTimeLockDuration,
            state: MessageState.INITIALIZED
        });

        emit BridgeTransferInitiated(bridgeTransferId, originator, recipient, totalAmount, hashLock, initiatorTimeLockDuration);
        return bridgeTransferId;
    }

    function completeBridgeTransfer(bytes32 bridgeTransferId, bytes32 preImage) external {
        BridgeTransfer storage bridgeTransfer = bridgeTransfers[bridgeTransferId];
        if (bridgeTransfer.state != MessageState.INITIALIZED) revert BridgeTransferHasBeenCompleted();
        if (keccak256(abi.encodePacked(preImage)) != bridgeTransfer.hashLock) revert InvalidSecret();
        if (block.timestamp > bridgeTransfer.timeLock) revert TimeLockExpired();
        bridgeTransfer.state = MessageState.COMPLETED;

        emit BridgeTransferCompleted(bridgeTransferId, preImage);
    }

    function refundBridgeTransfer(bytes32 bridgeTransferId) external onlyOwner {
        BridgeTransfer storage bridgeTransfer = bridgeTransfers[bridgeTransferId];
        if (bridgeTransfer.state != MessageState.INITIALIZED) revert BridgeTransferStateNotInitialized();
        if (block.timestamp < bridgeTransfer.timeLock) revert TimeLockNotExpired();
        bridgeTransfer.state = MessageState.REFUNDED;
        
        // Decrease pool balance and transfer WETH back to originator
        poolBalance -= bridgeTransfer.amount;
        if (!weth.transfer(bridgeTransfer.originator, bridgeTransfer.amount)) revert WETHTransferFailed();

        emit BridgeTransferRefunded(bridgeTransferId);
    }

    // Counterparty contract to withdraw WETH for originator
    function withdrawWETH(address recipient, uint256 amount) external {
        if (msg.sender != counterpartyAddress) revert Unauthorized();
        if (poolBalance < amount) revert InsufficientWethBalance();
        poolBalance -= amount;
        if (!weth.transfer(recipient, amount)) revert WETHTransferFailed();
    }

    // One-shot function to update poolBalance, callable only once by the owner
    function updatePoolBalance(uint256 newBalance) external onlyOwner {
        // Ensure this function can only be called once
        if (oneShotCalled) revert OneShotFunctionAlreadyCalled();
        
        // Update the pool balance
        poolBalance = newBalance;
        
        // Mark the one-shot function as called
        oneShotCalled = true;
    }
}

