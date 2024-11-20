// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.22;
pragma abicoder v2;

import {AccessControlUpgradeable} from "@openzeppelin/contracts-upgradeable/access/AccessControlUpgradeable.sol";
import {PausableUpgradeable} from "@openzeppelin/contracts-upgradeable/utils/PausableUpgradeable.sol";
import {INativeBridge} from "./INativeBridge.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {BokkyPooBahsDateTimeLibrary} from "@DateTimeLibrary/contracts/BookyPooBahsDateTimeLibrary.sol";

// import {RateLimiter} from "./RateLimiter.sol";

contract NativeBridge is AccessControlUpgradeable, PausableUpgradeable, INativeBridge {
    using BokkyPooBahsDateTimeLibrary for uint256;
    struct OutgoingTransfer {
        bytes32 bridgeTransferId;
        address initiator;
        bytes32 recipient;
        uint256 amount;
    }

    mapping(uint256 nonce => OutgoingTransfer) public noncesToOutgoingTransfers;
    mapping(bytes32 bridgeTransferId => uint256 nonce) public idsToIncomingNonces;
    mapping(uint256 year => mapping(uint256 month => (uint256 day => uint256 amount))) public outboundRateLimitBudget;
    mapping(uint256 year => mapping(uint256 month => (uint256 day => uint256 amount))) public incomingRateLimitBudget;

    uint256 public outboundRateLimit;
    uint256 public incomingRateLimit;
    IERC20 public moveToken;
    bytes32 public constant RELAYER_ROLE = keccak256(abi.encodePacked("RELAYER_ROLE"));
    uint256 private _nonce;

    // Prevents initialization of implementation contract exploits
    constructor() {
        _disableInitializers();
    }
    // TODO: include rate limit

    function initialize(address _moveToken, address _admin, address _relayer, address _maintainer, uint256 _outboundRateLimit, uint256 _incomingRateLimit) public initializer {
        require(_moveToken != address(0) && _admin != address(0) && _relayer != address(0), ZeroAddress());
        __Pausable_init();
        moveToken = IERC20(_moveToken);
        _grantRole(DEFAULT_ADMIN_ROLE, _admin);
        _grantRole(RELAYER_ROLE, _relayer);

        // Set the rate limits
        outboundRateLimit = _outboundRateLimit;
        incomingRateLimit = _incomingRateLimit;

        // Maintainer is optional
        _grantRole(RELAYER_ROLE, _maintainer);
    }

    function initiateBridgeTransfer(bytes32 recipient, uint256 amount)
        external
        whenNotPaused
        returns (bytes32 bridgeTransferId)
    {
        // Ensure there is a valid amount
        require(amount > 0, ZeroAmount());
        _outboundRateLimit(amount);
        //   _l1l2RateLimit(amount);
        address initiator = msg.sender;

        // Transfer the MOVE tokens from the user to the contract
        if (!moveToken.transferFrom(initiator, address(this), amount)) revert MOVETransferFailed();

        // Generate a unique nonce to prevent replay attacks, and generate a transfer ID
        bridgeTransferId = keccak256(abi.encodePacked(initiator, recipient, amount, ++_nonce));

        // Store the bridge transfer details
        noncesToOutgoingTransfers[_nonce] = OutgoingTransfer(bridgeTransferId, initiator, recipient, amount);

        emit BridgeTransferInitiated(bridgeTransferId, initiator, recipient, amount, _nonce);
        return bridgeTransferId;
    }

    function completeBridgeTransfer(
        bytes32 bridgeTransferId,
        bytes32 initiator,
        address recipient,
        uint256 amount,
        uint256 nonce
    ) external onlyRole(RELAYER_ROLE) {
        _completeBridgeTransfer(bridgeTransferId, initiator, recipient, amount, nonce);
    }

    function batchCompleteBridgeTransfer(
        bytes32[] memory bridgeTransferIds,
        bytes32[] memory initiators,
        address[] memory recipients,
        uint256[] memory amounts,
        uint256[] memory nonces
    ) external onlyRole(RELAYER_ROLE) {
        uint256 length = bridgeTransferIds.length;
        // checks if all arrays are of the same length
        require(
            initiators.length == length && recipients.length == length && amounts.length == length
                && nonces.length == length,
            InvalidLenghts()
        );
        // iterate over the arrays and complete the bridge transfer
        for (uint256 i; i < length; i++) {
            _completeBridgeTransfer(bridgeTransferIds[i], initiators[i], recipients[i], amounts[i], nonces[i]);
        }
    }

    function _completeBridgeTransfer(
        bytes32 bridgeTransferId,
        bytes32 initiator,
        address recipient,
        uint256 amount,
        uint256 nonce
    ) internal {
        _incomingRateLimit(amount);
        // Ensure the bridge transfer has not already been completed
        require(nonce > 0, InvalidNonce());
        require(idsToIncomingNonces[bridgeTransferId] == 0, CompletedBridgeTransferId());
        // Ensure the bridge transfer ID is valid against the initiator, recipient, amount, and nonce
        require(
            bridgeTransferId == keccak256(abi.encodePacked(initiator, recipient, amount, nonce)),
            InvalidBridgeTransferId()
        );

        // Store the nonce to bridge transfer ID
        idsToIncomingNonces[bridgeTransferId] = nonce;

        // Transfer the MOVE tokens to the recipient
        if (!moveToken.transfer(recipient, amount)) revert MOVETransferFailed();

        emit BridgeTransferCompleted(bridgeTransferId, initiator, recipient, amount, nonce);
    }

    function setRateLimits(uint256 _outboundRateLimit, uint256 _incomingRateLimit) external onlyRole(DEFAULT_ADMIN_ROLE) {
        outboundRateLimit = _outboundRateLimit;
        incomingRateLimit = _incomingRateLimit;
        emit RateLimitsUpdated(_outboundRateLimit, _incomingRateLimit);
    }

    function togglePause() external onlyRole(DEFAULT_ADMIN_ROLE) {
        paused() ? _pause() : _unpause();
    }

    _rateLimitOutbound(uint256 amount) internal {
        (uint256 year, uint256 month, uint256 day) = block.timestamp.timestampToDate();
        // does += amount <= outboundRateLimit mean that the amount is added to the budget and then checked if it is less than the rate limit?
        require(outboundRateLimitBudget[year][month][day] += amount <= outboundRateLimit, OutboundRateLimitExceeded());
    }

    _rateLimitIncoming(uint256 amount) internal {
        (uint256 year, uint256 month, uint256 day) = block.timestamp.timestampToDate();
        require(incomingRateLimitBudget[year][month][day] += amount <= incomingRateLimit, IncomingRateLimitExceeded());
    }
}
