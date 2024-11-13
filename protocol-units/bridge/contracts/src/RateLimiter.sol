// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import {OwnableUpgradeable} from "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";

contract RateLimiter is OwnableUpgradeable {
    enum TransferDirection {
        L1_TO_L2,
        L2_TO_L1
    }

    // Maximum amount that can be transferred in each direction within the risk period
    uint256 public rateLimitL1L2;
    uint256 public rateLimitL2L1;

    // Track the accumulated budget per transfer direction
    uint256 public budgetL1L2;
    uint256 public budgetL2L1;

    // Risk period for rate limiting (in seconds)
    uint256 public riskPeriod;

    // Security fund balance
    uint256 public securityFund;

    event RateLimitExceeded(TransferDirection direction);
    event RateLimitUpdated(uint256 newRateLimitL1L2, uint256 newRateLimitL2L1);
    event SecurityFundUpdated(uint256 newSecurityFund);

    // Initialize the contract with initial rate limits and risk period
    function initialize(address owner, uint256 _riskPeriod, uint256 _securityFund) public initializer {
        riskPeriod = _riskPeriod;
        securityFund = _securityFund;
        __Ownable_init(owner);
        _updateRateLimits();
    }

    // Modifier to check if a transfer exceeds the rate limit
    modifier withinRateLimit(uint256 amount, TransferDirection direction) {
        uint256 currentBudget = (direction == TransferDirection.L1_TO_L2) ? budgetL1L2 : budgetL2L1;
        uint256 rateLimit = (direction == TransferDirection.L1_TO_L2) ? rateLimitL1L2 : rateLimitL2L1;

        require(currentBudget + amount <= rateLimit, "RATE_LIMIT_EXCEEDED");
        _;
    }

    function initiateTransfer(uint256 amount, TransferDirection direction) external returns (bool) {
        uint256 currentBudget = (direction == TransferDirection.L1_TO_L2) ? budgetL1L2 : budgetL2L1;
        uint256 rateLimit = (direction == TransferDirection.L1_TO_L2) ? rateLimitL1L2 : rateLimitL2L1;

        if (currentBudget + amount > rateLimit) {
            emit RateLimitExceeded(direction);
            return false;
        }

        // Update the budget for the specified direction
        if (direction == TransferDirection.L1_TO_L2) {
            budgetL1L2 += amount;
        } else {
            budgetL2L1 += amount;
        }

        return true;
    }

    // Update the security fund and recalculate rate limits
    function updateSecurityFund(uint256 newSecurityFund) external onlyOwner {
        securityFund = newSecurityFund;
        _updateRateLimits();
        emit SecurityFundUpdated(newSecurityFund);
    }

    // Private function to update the rate limits based on the security fund and risk period
    function _updateRateLimits() private {
        rateLimitL1L2 = (securityFund * 5) / (riskPeriod * 10); // 0.5 * securityFund / riskPeriod
        rateLimitL2L1 = (securityFund * 5) / (riskPeriod * 10); // Same calculation as for L1 to L2

        emit RateLimitUpdated(rateLimitL1L2, rateLimitL2L1);
    }

    // Reset the budget for each direction; this could be called periodically or by governance if all transfers are confirmed
    function resetBudget() external onlyOwner {
        budgetL1L2 = 0;
        budgetL2L1 = 0;
    }
}
