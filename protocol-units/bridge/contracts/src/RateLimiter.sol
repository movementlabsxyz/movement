// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.22;

import {AccessControlUpgradeable} from "@openzeppelin/contracts-upgradeable/access/AccessControlUpgradeable.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";

contract RateLimiter is AccessControlUpgradeable {
    bytes32 public constant ATOMIC_BRIDGE = keccak256("ATOMIC_BRIDGE");
    bytes32 public constant RATE_LIMITER_ADMIN = keccak256("RATE_LIMITER_ADMIN");

    mapping(uint256 day => uint256 amount) public outboundRateLimitBudget;
    mapping(uint256 day => uint256 amount) public inboundRateLimitBudget;
    address public insuranceFund;
    IERC20 public moveToken;

    uint256 public rateLimiterNumerator;
    uint256 public rateLimiterDenominator;

    // the period over which the rate limit is enforced
    mapping(uint256 version => uint256) public periodDuration;
    uint256 periodDurationVersion;

    error OutboundRateLimitExceeded();
    error InboundRateLimitExceeded();
    error RateLimitCoefficientTooLow();

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
        _grantRole(ATOMIC_BRIDGE, _owner);
        _grantRole(ATOMIC_BRIDGE, _initiatorAddress);
        _grantRole(ATOMIC_BRIDGE, _counterpartyAddress);
        _grantRole(RATE_LIMITER_ADMIN, _owner);
        _grantRole(RATE_LIMITER_ADMIN, _initiatorAddress);
        _grantRole(RATE_LIMITER_ADMIN, _counterpartyAddress);
        moveToken = IERC20(_moveToken);
        insuranceFund = _insuranceFund;
        periodDuration = 1 days;
        rateLimiterNumerator = 1;
        rateLimiterDenominator = 4;
    }

    function setRateLimitPeriodDuration(uint256 _periodDuration) external onlyRole(RATE_LIMITER_ADMIN) {
        periodDurationVersion++;
        periodDuration[periodDurationVersion] = _periodDuration;
    }

    function currentPeriodDuration() public view returns (uint256) {
        return periodDuration[periodDurationVersion];
    }

    function setRateLimiterCoefficients(uint256 numerator, uint256 denominator) external onlyRole(RATE_LIMITER_ADMIN) {

        require(numerator == 0 || denominator/numerator >= 4, RateLimitCoefficientTooLow());

        rateLimiterNumerator = numerator;
        rateLimiterDenominator = denominator;
    }

    function rateLimitOutbound(uint256 amount) external onlyRole(ATOMIC_BRIDGE) {
        uint256 period = block.timestamp / currentPeriodDuration();
        outboundRateLimitBudget[period] += amount;
        uint256 periodMax = moveToken.balanceOf(insuranceFund) * rateLimiterNumerator / rateLimiterDenominator;
        require(outboundRateLimitBudget[period] < periodMax, OutboundRateLimitExceeded());
    }

    function rateLimitInbound(uint256 amount) external onlyRole(ATOMIC_BRIDGE) {
        uint256 period = block.timestamp / currentPeriodDuration();
        inboundRateLimitBudget[period] += amount;
        uint256 periodMax = moveToken.balanceOf(insuranceFund) * rateLimiterNumerator / rateLimiterDenominator;
        require(inboundRateLimitBudget[period] < periodMax, InboundRateLimitExceeded());
    }
}
