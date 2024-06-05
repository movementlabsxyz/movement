// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "../base/BaseToken.sol";

contract LockedToken is BaseToken {

    struct Lock {
        uint256 amount;
        uint256 releaseTime;
    }

    mapping(address => Lock[]) public locks;
    IERC20Upgradeable public underlyingToken;

    function initialize(
        string memory name, 
        string memory symbol,
        IERC20Upgradeable _underlyingToken
    ) public initializer {
        super.initialize(name, symbol);
    }

    /**
     * @dev Mint new tokens
     * @param to The address to mint tokens to
     * @param amount The amount of tokens to mint
     */
    function mintAndLock(address[] calldata addresses, uint256[] calldata amounts, uint256[] calldata lockTimes) external onlyRole(MINTER_ROLE) {
        require(addresses.length == amounts.length, "Addresses and amounts length mismatch");
        require(addresses.length == lockTimes.length, "Addresses and lock times length mismatch");

        for (uint256 i = 0; i < addresses.length; i++) {
            _mint(address(this), amounts[i]);
            _lock(addresses[i], amounts[i], lockTimes[i]);
        }
    }

    /**
     * @dev Lock tokens
     * @param account The address to lock tokens for
     * @param amount The amount of tokens to lock
     * @param lockTime The time to lock the tokens for
     */
    function _lock(address account, uint256 amount, uint256 lockTime) internal {
        locks[account].push(Lock(amount, lockTime));
    }

    /**
     * @dev Release unlocked tokens
     */
    function release() external {
        uint256 totalUnlocked = 0;
        Lock[] storage userLocks = locks[msg.sender];
        for (uint256 i = 0; i < userLocks.length; i++) {
            if (block.timestamp >= userLocks[i].releaseTime) {
                totalUnlocked = totalUnlocked.add(userLocks[i].amount);
                userLocks[i] = userLocks[userLocks.length - 1];
                userLocks.pop();
            }
        }
        require(totalUnlocked > 0, "No tokens to release");
        moveToken.safeTransfer(msg.sender, totalUnlocked);
    }

    /**
     * @dev Get the total locked balance of an account
     * @param account The address to get the total locked balance of
     * @return The total locked balance of the account
     */
    function balanceOfLocked(address account) external view returns (uint256) {
        uint256 totalLocked = 0;
        Lock[] memory userLocks = locks[account];
        for (uint256 i = 0; i < userLocks.length; i++) {
            totalLocked = totalLocked.add(userLocks[i].amount);
        }
        return totalLocked;
    }

    /**
     * @dev Transfer tokens
     * @param from The address to transfer tokens from
     * @param to The address to transfer tokens to
     * @param amount The amount of tokens to transfer
     */
    function _transfer(address from, address to, uint256 amount) internal override {
        require(false, "Cannot transfer locked tokens");
    }

    /**
     * @dev Approve tokens
     * @param spender The address to approve tokens for
     * @param amount The amount of tokens to approve
     * @return A boolean indicating whether the approval was successful
     */
    function approve(address spender, uint256 amount) public override returns (bool) {
        require(false, "Cannot approve locked tokens");
    }

    /** 
     * @dev Transfer tokens from
     * @param sender The address to transfer tokens from
     * @param recipient The address to transfer tokens to
     * @param amount The amount of tokens to transfer
     * @return A boolean indicating whether the transfer was successful
     */
    function transferFrom(address sender, address recipient, uint256 amount) public override returns (bool) {
        require(false, "Cannot transfer locked tokens");
    }

    /**
     * @dev Transfer tokens
     * @param recipient The address to transfer tokens to
     * @param amount The amount of tokens to transfer
     * @return A boolean indicating whether the transfer was successful
     */
    function transfer(address recipient, uint256 amount) public override returns (bool) {
        require(false, "Cannot transfer locked tokens");
    }

}
