// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.22;
pragma abicoder v2;

import {AccessControlUpgradeable} from "@openzeppelin/contracts-upgradeable/access/AccessControlUpgradeable.sol";
import {INativeBridge} from "./INativeBridge.sol";
import {ERC20Upgradeable} from "@openzeppelin/contracts-upgradeable/token/ERC20/ERC20Upgradeable.sol";

// import {RateLimiter} from "./RateLimiter.sol";

contract NativeBridge is AccessControlUpgradeable, INativeBridge {
    struct OutgoingBridgeTransfer {
        address originator;
        bytes32 recipient;
        uint256 amount;
        uint256 nonce;
    }

    mapping(bytes32 bridgeTransferId => OutgoingBridgeTransfer) public outgoingBridgeTransfers;

    struct IncomingBridgeTransfer {
        bytes32 originator;
        address recipient;
        uint256 amount;
        uint256 nonce;
    }

    mapping(bytes32 bridgeTransferId => IncomingBridgeTransfer) public incomingBridgeTransfers;

    ERC20Upgradeable public moveToken;
    bytes32 public constant RELAYER_ROLE = keccak256(abi.encodePacked("RELAYER_ROLE"));
    uint256 private _nonce;

    // Prevents initialization of implementation contract exploits
    constructor() {
        _disableInitializers();
    }
    // TODO: include rate limit

    function initialize(address _moveToken, address _admin, address _relayer, address _maintainer) public initializer {
        require(_moveToken != address(0) && _admin != address(0) && _relayer != address(0), ZeroAddress());
        moveToken = ERC20Upgradeable(_moveToken);
        _grantRole(DEFAULT_ADMIN_ROLE, _admin);
        _grantRole(RELAYER_ROLE, _relayer);
        _grantRole(RELAYER_ROLE, _maintainer);
    }

    function initiateBridge(bytes32 recipient, uint256 amount) external returns (bytes32 bridgeTransferId) {
        // Ensure there is a valid amount
        require(amount > 0, ZeroAmount());
        //   _l1l2RateLimit(amount);
        address originator = msg.sender;

        // Transfer the MOVE tokens from the user to the contract
        if (!moveToken.transferFrom(originator, address(this), amount)) revert MOVETransferFailed();

        // Generate a unique nonce to prevent replay attacks, and generate a transfer ID
        bridgeTransferId = keccak256(abi.encodePacked(originator, recipient, amount, ++_nonce));

        outgoingBridgeTransfers[bridgeTransferId] = OutgoingBridgeTransfer(originator, recipient, amount, _nonce);

        emit BridgeTransferInitiated(bridgeTransferId, originator, recipient, amount, _nonce);
        return bridgeTransferId;
    }

    function completeBridge(
        bytes32 bridgeTransferId,
        bytes32 originator,
        address recipient,
        uint256 amount,
        uint256 nonce
    ) external onlyRole(RELAYER_ROLE) {
        _completeBridge(bridgeTransferId, originator, recipient, amount, nonce);
    }

    function batchCompleteBridge(
        bytes32[] memory bridgeTransferIds,
        bytes32[] memory originators,
        address[] memory recipients,
        uint256[] memory amounts,
        uint256[] memory nonces
    ) external onlyRole(RELAYER_ROLE) {
        uint256 length = bridgeTransferIds.length;
        require(
            originators.length == length && recipients.length == length && amounts.length == length
                && nonces.length == length,
            InvalidLenghts()
        );
        for (uint256 i; i < length; i++) {
            _completeBridge(bridgeTransferIds[i], originators[i], recipients[i], amounts[i], nonces[i]);
        }
    }

    function _completeBridge(
        bytes32 bridgeTransferId,
        bytes32 originator,
        address recipient,
        uint256 amount,
        uint256 nonce
    ) internal {
        // _l2l1RateLimit(amount);
        require(
            bridgeTransferId == keccak256(abi.encodePacked(originator, recipient, amount, nonce)),
            InvalidBridgeTransferId()
        );
        require(incomingBridgeTransfers[bridgeTransferId].amount == 0);
        incomingBridgeTransfers[bridgeTransferId] = IncomingBridgeTransfer(originator, recipient, amount, nonce);

        if (!moveToken.transfer(recipient, amount)) revert MOVETransferFailed();

        emit BridgeTransferCompleted(bridgeTransferId, originator, recipient, amount, nonce);
    }
}
