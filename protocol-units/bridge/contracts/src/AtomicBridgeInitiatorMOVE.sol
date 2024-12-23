// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import {IAtomicBridgeInitiatorMOVE} from "./IAtomicBridgeInitiatorMOVE.sol";
import {AccessControlUpgradeable} from "@openzeppelin/contracts-upgradeable/access/AccessControlUpgradeable.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {RateLimiter} from "./RateLimiter.sol";

contract AtomicBridgeInitiatorMOVE is IAtomicBridgeInitiatorMOVE, AccessControlUpgradeable {
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

    address public counterpartyAddress;
    RateLimiter public rateLimiter;
    IERC20 public moveToken;
    uint256 private nonce;

    // Configurable time lock duration
    uint256 public initiatorTimeLockDuration;

    bytes32 public constant RELAYER_ROLE = keccak256("RELAYER_ROLE");
    bytes32 public constant REFUNDER_ROLE = keccak256("REFUNDER_ROLE");

    // Prevents initialization of implementation contract exploits
    constructor() {
        _disableInitializers();
    }

    // Initialize the contract with MOVE token address, owner, and custom time lock duration
    function initialize(
        address _moveToken,
        address _owner,
        address _relayer,
        address _refunder,
        uint256 _timeLockDuration
    ) public initializer {
        if (_moveToken == address(0) && _owner == address(0) && _relayer == address(0) && _refunder == address(0)) {
            revert ZeroAddress();
        }
        require(_timeLockDuration > 0, ZeroAmount());
        moveToken = IERC20(_moveToken);
        _grantRole(DEFAULT_ADMIN_ROLE, _owner);
        _grantRole(RELAYER_ROLE, _relayer);
        _grantRole(REFUNDER_ROLE, _refunder);

        // Set the custom time lock duration
        initiatorTimeLockDuration = _timeLockDuration;
    }

    function setCounterpartyAddress(address _counterpartyAddress) external onlyRole(DEFAULT_ADMIN_ROLE) {
        require(_counterpartyAddress != address(0), "ZeroAddress");
        counterpartyAddress = _counterpartyAddress;
    }

    function setRateLimiter(address _rateLimiter) external onlyRole(DEFAULT_ADMIN_ROLE) {
        if (_rateLimiter == address(0)) revert ZeroAddress();
        rateLimiter = RateLimiter(_rateLimiter);
    }

    function initiateBridgeTransfer(uint256 moveAmount, bytes32 recipient, bytes32 hashLock)
        external
        returns (bytes32 bridgeTransferId)
    {
        rateLimiter.rateLimitOutbound(moveAmount);
        address originator = msg.sender;
            
        require(moveAmount > 0, ZeroAmount());

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

    function completeBridgeTransfer(bytes32 bridgeTransferId, bytes32 preImage) external onlyRole(RELAYER_ROLE) {
        BridgeTransfer storage bridgeTransfer = bridgeTransfers[bridgeTransferId];

        rateLimiter.rateLimitInbound(bridgeTransfer.amount);
        require(bridgeTransfer.state == MessageState.INITIALIZED, "BridgeTransferHasBeenCompleted");
        require(keccak256(abi.encodePacked(preImage)) == bridgeTransfer.hashLock, "InvalidSecret");
        require(block.timestamp <= bridgeTransfer.timeLock, "TimelockExpired");

        bridgeTransfer.state = MessageState.COMPLETED;

        emit BridgeTransferCompleted(bridgeTransferId, preImage);
    }

    function refundBridgeTransfer(bytes32 bridgeTransferId) external onlyRole(REFUNDER_ROLE) {
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
        if (!moveToken.transfer(recipient, amount)) revert MOVETransferFailed();
    }
}
