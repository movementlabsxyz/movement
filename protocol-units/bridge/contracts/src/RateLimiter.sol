// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.22;

import {AccessControlUpgradeable} from "@openzeppelin/contracts-upgradeable/access/AccessControlUpgradeable.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";

contract RateLimiter is AccessControlUpgradeable {
    bytes32 public constant ATOMIC_BRIDGE = keccak256("ATOMIC_BRIDGE");

    // Rate limit budget for outbound transfers
    mapping(uint256 day => uint256 amount) public outboundRateLimitBudget;
    // Rate limit budget for inbound transfers
    mapping(uint256 day => uint256 amount) public inboundRateLimitBudget;
    // Address of the insurance fund
    address public insuranceFund;
    IERC20 public moveToken;

    error OutboundRateLimitExceeded();
    error InboundRateLimitExceeded();

    constructor() {
        _disableInitializers();
    }

    function initialize(
        address _moveToken,
        address _owner,
        address _initiatorAddress,
        address _counterpartyAddress,
        address _insuranceFund
    ) public initializer {
        _grantRole(DEFAULT_ADMIN_ROLE, _owner);
        _grantRole(ATOMIC_BRIDGE, _initiatorAddress);
        _grantRole(ATOMIC_BRIDGE, _counterpartyAddress);
        moveToken = IERC20(_moveToken);
        insuranceFund = _insuranceFund;
    }

    // Rate limit the amount of MOVE tokens that can be transferred to the Atomic Bridge
    // The rate limit is set to 25% of the total MOVE tokens in the insurance fund
    // Only instances of the AtomicBridge contract can call this function
    function rateLimitOutbound(uint256 amount) external onlyRole(ATOMIC_BRIDGE) {
        uint256 day = block.timestamp / 1 days;
        outboundRateLimitBudget[day] += amount;
        require(outboundRateLimitBudget[day] < moveToken.balanceOf(insuranceFund) / 4, OutboundRateLimitExceeded());
    }

    // Rate limit the amount of MOVE tokens that can be transferred from the Atomic Bridge
    // The rate limit is set to 25% of the total MOVE tokens in the insurance fund
    // Only instances of the AtomicBridge contract can call this function
    function rateLimitInbound(uint256 amount) external onlyRole(ATOMIC_BRIDGE) {
        uint256 day = block.timestamp / 1 days;
        inboundRateLimitBudget[day] += amount;
        require(inboundRateLimitBudget[day] < moveToken.balanceOf(insuranceFund) / 4, InboundRateLimitExceeded());
    }
}
