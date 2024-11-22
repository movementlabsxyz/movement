// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.22;

import {AccessControlUpgradeable} from "@openzeppelin/contracts-upgradeable/access/AccessControlUpgradeable.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";

contract RateLimiter is AccessControlUpgradeable {
    bytes32 public constant ATOMIC_BRIDGE = keccak256("ATOMIC_BRIDGE");

    mapping(uint256 day => uint256 amount) public outboundRateLimitBudget;
    mapping(uint256 day => uint256 amount) public inboundRateLimitBudget;
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

    function rateLimitOutbound(uint256 amount) external onlyRole(ATOMIC_BRIDGE) {
        uint256 day = block.timestamp / 1 days;
        outboundRateLimitBudget[day] += amount;
        require(outboundRateLimitBudget[day] < moveToken.balanceOf(insuranceFund) / 4, OutboundRateLimitExceeded());
    }

    function rateLimitInbound(uint256 amount) external onlyRole(ATOMIC_BRIDGE) {
        uint256 day = block.timestamp / 1 days;
        inboundRateLimitBudget[day] += amount;
        require(inboundRateLimitBudget[day] < moveToken.balanceOf(insuranceFund) / 4, InboundRateLimitExceeded());
    }
}
