// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "../base/BaseToken.sol";
import "../base/MintableToken.sol";
import "../base/WrappedToken.sol";
import "../custodian/CustodianToken.sol";
import "@openzeppelin/contracts-upgradeable/token/ERC20/ERC20Upgradeable.sol";
import {Math} from "@openzeppelin/contracts/utils/math/Math.sol";

contract LockedToken is CustodianToken {

    bytes32 public constant MINT_LOCKER_ROLE = keccak256("MINT_LOCKER_ROLE");
    bytes32 public constant MINT_LOCKER_ADMIN_ROLE = keccak256("MINT_LOCKER_ADMIN_ROLE");

    struct Lock {
        uint256 amount;
        uint256 releaseTime;
    }

    mapping(address => Lock[]) public locks;

    function initialize(
        string memory name, 
        string memory symbol,
        IMintableToken _underlyingToken
    ) public override virtual {
        super.initialize(name, symbol, _underlyingToken);
        _grantRole(MINT_LOCKER_ADMIN_ROLE, msg.sender);
        _grantRole(MINT_LOCKER_ROLE, msg.sender);
    }

    /**
    * @dev Mint and lock tokens
    * @param addresses The addresses to mint and lock tokens for
    * @param mintAmounts The amounts to mint.
    * @param lockAmounts The amount up to which the user is allowed to be unlock, respective of balance
    * @param lockTimes The times to lock the tokens for
    */
    function mintAndLock(
        address[] calldata addresses, 
        uint256[] calldata mintAmounts,
        uint256[] calldata lockAmounts,
        uint256[] calldata lockTimes
    ) external onlyRole(MINT_LOCKER_ROLE) {
        require(addresses.length == mintAmounts.length, "Addresses and amounts length mismatch");
        require(addresses.length == lockAmounts.length, "Addresses and lock amounts length mismatch");
        require(addresses.length == lockTimes.length, "Addresses and lock times length mismatch");

        for (uint256 i = 0; i < addresses.length; i++) {
            underlyingToken.mint(address(this), mintAmounts[i]);
            _mint(addresses[i], mintAmounts[i]);
            _lock(addresses[i], lockAmounts[i], lockTimes[i]);
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

                // compute the max possible amount to withdraw
                uint256 amount = Math.min(userLocks[i].amount, balanceOf(msg.sender));

                // burn the amount so that the user can't overdraw
                _transfer(msg.sender, address(this), amount);

                // add to the total unlocked amount
                totalUnlocked += amount;

                // deduct the amount from the lock
                userLocks[i].amount -= amount;

                // if the amount on the lock is now 0, remove the lock
                if (userLocks[i].amount == 0) {
                    userLocks[i] = userLocks[userLocks.length - 1];
                    userLocks.pop();
                }

            }
        }

        // transfer the underlying token
        underlyingToken.transfer(msg.sender, totalUnlocked);

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
            totalLocked += userLocks[i].amount;
        }
        return totalLocked;
    }
    
}
