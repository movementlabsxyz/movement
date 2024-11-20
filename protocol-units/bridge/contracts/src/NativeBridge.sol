// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.22;
pragma abicoder v2;

import {AccessControlUpgradeable} from "@openzeppelin/contracts-upgradeable/access/AccessControlUpgradeable.sol";
import {PausableUpgradeable} from "@openzeppelin/contracts-upgradeable/utils/PausableUpgradeable.sol";
import {INativeBridge} from "./INativeBridge.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";

// import {RateLimiter} from "./RateLimiter.sol";

contract NativeBridge is AccessControlUpgradeable, PausableUpgradeable, INativeBridge {
    struct OutgoingBridgeTransfer {
        address initiator;
        bytes32 recipient;
        uint256 amount;
        uint256 nonce;
    }
    mapping(bytes32 bridgeTransferId => OutgoingBridgeTransfer) public outgoingBridgeTransfers;
    mapping(uint256 nonce => bytes32 incomingBridgeTransferId) public noncesToIncomingBridgeTransferIds;

    IERC20 public moveToken;
    bytes32 public constant RELAYER_ROLE = keccak256(abi.encodePacked("RELAYER_ROLE"));
    uint256 private _nonce;
    address private feeCollector;
    uint256 private collectedBridgeFee;

    // Prevents initialization of implementation contract exploits
    constructor() {
        _disableInitializers();
    }
    // TODO: include rate limit

    function initialize(address _moveToken, address _admin, address _relayer, address _maintainer) public initializer {
        require(_moveToken != address(0) && _admin != address(0) && _relayer != address(0), ZeroAddress());
        __Pausable_init();
        moveToken = IERC20(_moveToken);
        _grantRole(DEFAULT_ADMIN_ROLE, _admin);
        _grantRole(RELAYER_ROLE, _relayer);

        // Maintainer is optional
        _grantRole(RELAYER_ROLE, _maintainer);

        feeCollector = _maintainer;
    }

    function setFeeCollector(address _feeCollector) external onlyRole(DEFAULT_ADMIN_ROLE) {
        feeCollector = _feeCollector;
    }

    function initiateBridgeTransfer(bytes32 recipient, uint256 amount)
        external
        whenNotPaused
        returns (bytes32 bridgeTransferId)
    {
        // Ensure there is a valid amount
        require(amount > 0, ZeroAmount());
        //   _l1l2RateLimit(amount);
        address initiator = msg.sender;

        // Transfer the MOVE tokens from the user to the contract
        if (!moveToken.transferFrom(initiator, address(this), amount)) revert MOVETransferFailed();

        // Generate a unique nonce to prevent replay attacks, and generate a transfer ID
        bridgeTransferId = keccak256(abi.encodePacked(initiator, recipient, amount, ++_nonce));

        // Store the bridge transfer details
        outgoingBridgeTransfers[bridgeTransferId] = OutgoingBridgeTransfer(initiator, recipient, amount, _nonce);

        emit BridgeTransferInitiated(bridgeTransferId, initiator, recipient, amount, _nonce);
        return bridgeTransferId;
    }

    function completeBridgeTransfer(
        bytes32 bridgeTransferId,
        bytes32 initiator,
        address recipient,
        uint256 amount,
        uint256 nonce,
        uint256 bridgeFee
    ) external onlyRole(RELAYER_ROLE) {
        _completeBridgeTransfer(bridgeTransferId, initiator, recipient, amount, nonce);
        _transferBridgeFee();
    }

    function batchCompleteBridgeTransfer(
        bytes32[] memory bridgeTransferIds,
        bytes32[] memory initiators,
        address[] memory recipients,
        uint256[] memory amounts,
        uint256[] memory nonces,
        uint256 bridgeFee
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
            _completeBridgeTransfer(bridgeTransferIds[i], initiators[i], recipients[i], amounts[i], nonces[i], bridgeFee);
        }
        _transferBridgeFee();
    }

    function _completeBridgeTransfer(
        bytes32 bridgeTransferId,
        bytes32 initiator,
        address recipient,
        uint256 amount,
        uint256 nonce,
        uint256 bridgeFee
    ) internal {
        // _l2l1RateLimit(amount);
        // Ensure the bridge transfer ID is valid against the initiator, recipient, amount, and nonce
        require(
            bridgeTransferId == keccak256(abi.encodePacked(initiator, recipient, amount, nonce)),
            InvalidBridgeTransferId()
        );
        // Ensure the bridge transfer has not already been completed
        require(noncesToIncomingBridgeTransferIds[nonce] == bytes32(0x0), CompletedBridgeTransferId());

        // Store the nonce to bridge transfer ID
        noncesToIncomingBridgeTransferIds[nonce] = bridgeTransferId;

        uint256 newAmount = amount - bridgeFee;
        // Transfer the MOVE tokens to the recipient
        if (!moveToken.transfer(recipient, newAmount)) revert MOVETransferFailed();

        emit BridgeTransferCompleted(bridgeTransferId, initiator, recipient, amount, nonce);
    }

    function _transferBridgeFee() internal {
        uint256 fee = collectedBridgeFee;
        collectedBridgeFee = 0;
        if (!moveToken.transfer(feeCollector, fee)) revert MOVETransferFailed();
    }


    function togglePause() external onlyRole(DEFAULT_ADMIN_ROLE) {
        paused() ? _pause() : _unpause();
    }
}
