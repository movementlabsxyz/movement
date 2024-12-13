// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.22;
pragma abicoder v2;

import {AccessControlUpgradeable} from "@openzeppelin/contracts-upgradeable/access/AccessControlUpgradeable.sol";
import {PausableUpgradeable} from "@openzeppelin/contracts-upgradeable/utils/PausableUpgradeable.sol";
import {INativeBridge} from "./INativeBridge.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "forge-std/console.sol";

contract NativeBridge is AccessControlUpgradeable, PausableUpgradeable, INativeBridge {
    struct OutboundTransfer {
        bytes32 bridgeTransferId;
        address initiator;
        bytes32 recipient;
        uint256 amount;
    }

    mapping(uint256 nonce => OutboundTransfer) public noncesToOutboundTransfers;
    mapping(bytes32 bridgeTransferId => uint256 nonce) public idsToInboundNonces;
    mapping(uint256 day => uint256 amount) public inboundRateLimitBudget;

    bytes32 public constant RELAYER_ROLE = keccak256(abi.encodePacked("RELAYER_ROLE"));
    uint256 public constant MINIMUM_RISK_DENOMINATOR = 3;
    IERC20 public moveToken;
    address public insuranceFund;
    uint256 public riskDenominator;
    uint256 private _nonce;

    // Prevents initialization of implementation contract exploits
    constructor() {
        _disableInitializers();
    }

    /**
     * @dev Initializes the NativeBridge contract
     * @param _moveToken The address of the MOVE token contract
     * @param _admin The address of the admin role
     * @param _relayer The address of the relayer role
     * @param _maintainer The address of the maintainer role
     */
    function initialize(
        address _moveToken,
        address _admin,
        address _relayer,
        address _maintainer,
        address _insuranceFund
    ) public initializer {
        require(_moveToken != address(0) && _admin != address(0) && _relayer != address(0), ZeroAddress());
        __Pausable_init();
        moveToken = IERC20(_moveToken);
        _grantRole(DEFAULT_ADMIN_ROLE, _admin);
        _grantRole(RELAYER_ROLE, _relayer);

        // Set insurance fund
        insuranceFund = _insuranceFund;
        riskDenominator = MINIMUM_RISK_DENOMINATOR + 1;

        // Maintainer is optional
        _grantRole(RELAYER_ROLE, _maintainer);
    }

    /**
     * @dev Creates a new bridge
     * @param recipient The address on the other chain to which to transfer funds
     * @param amount The amount of MOVE to send
     * @return bridgeTransferId A unique id representing this BridgeTransfer
     *
     */
    function initiateBridgeTransfer(bytes32 recipient, uint256 amount)
        external
        whenNotPaused
        returns (bytes32 bridgeTransferId)
    {
        // Ensure there is a valid amount
        require(amount > 0, ZeroAmount());
        address initiator = msg.sender;

        // Transfer the MOVE tokens from the user to the contract
        if (!moveToken.transferFrom(initiator, address(this), amount)) revert MOVETransferFailed();

        // Generate a unique nonce to prevent replay attacks, and generate a transfer ID
        bridgeTransferId = keccak256(abi.encodePacked(initiator, recipient, amount, ++_nonce));

        // Store the bridge transfer details
        noncesToOutboundTransfers[_nonce] = OutboundTransfer(bridgeTransferId, initiator, recipient, amount);

        emit BridgeTransferInitiated(bridgeTransferId, initiator, recipient, amount, _nonce);
        return bridgeTransferId;
    }

    /**
     * @dev Completes the bridging of funds. Only the relayer can call this function.
     * @param bridgeTransferId Unique identifier for the BridgeTransfer
     * @param initiator The address on the other chain that originated the transfer of funds
     * @param recipient The address on this chain to which to transfer funds
     * @param amount The amount to transfer
     * @param nonce The seed nonce to generate the bridgeTransferId
     */
    function completeBridgeTransfer(
        bytes32 bridgeTransferId,
        bytes32 initiator,
        address recipient,
        uint256 amount,
        uint256 nonce
    ) external onlyRole(RELAYER_ROLE) {
        _completeBridgeTransfer(bridgeTransferId, initiator, recipient, amount, nonce);
    }

    /**
     * @dev Completes multiple bridge transfers
     * @param bridgeTransferIds Unique identifiers for the BridgeTransfers
     * @param initiators The addresses on the other chain that originated the transfer of funds
     * @param recipients The addresses on this chain to which to transfer funds
     * @param amounts The amounts to transfer
     * @param nonces The seed nonces to generate the bridgeTransferIds
     */
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
        _rateLimitInbound(amount);
        // Ensure the bridge transfer has not already been completed
        require(nonce > 0, InvalidNonce());
        require(idsToInboundNonces[bridgeTransferId] == 0, CompletedBridgeTransferId());
        // Ensure the bridge transfer ID is valid against the initiator, recipient, amount, and nonce
        require(
            bridgeTransferId == keccak256(abi.encodePacked(initiator, recipient, amount, nonce)),
            InvalidBridgeTransferId()
        );

        // Store the nonce to bridge transfer ID
        idsToInboundNonces[bridgeTransferId] = nonce;

        // Transfer the MOVE tokens to the recipient
        if (!moveToken.transfer(recipient, amount)) revert MOVETransferFailed();

        emit BridgeTransferCompleted(bridgeTransferId, initiator, recipient, amount, nonce);
    }

    function setInsuranceFund(address _insuranceFund) external onlyRole(DEFAULT_ADMIN_ROLE) {
        insuranceFund = _insuranceFund;
        emit InsuranceFundUpdated(_insuranceFund);
    }

    function setRiskDenominator(uint256 _riskDenominator) external onlyRole(DEFAULT_ADMIN_ROLE) {
        require(_riskDenominator > MINIMUM_RISK_DENOMINATOR, InvalidRiskDenominator());
        riskDenominator = _riskDenominator;
        emit RiskDenominatorUpdated(_riskDenominator);
    }

    function togglePause() external onlyRole(DEFAULT_ADMIN_ROLE) {
        paused() ? _pause() : _unpause();
        emit PauseToggled(paused());
    }

    /**
     * @dev Rate limits the inbound transfers based on the insurance fund and risk denominator
     * @param amount The amount to rate limit
     */

    function _rateLimitInbound(uint256 amount) public {
        uint256 day = block.timestamp / 1 days;
        inboundRateLimitBudget[day] += amount;
        require(
            inboundRateLimitBudget[day] < moveToken.balanceOf(insuranceFund) / riskDenominator,
            InboundRateLimitExceeded()
        );
    }
}
