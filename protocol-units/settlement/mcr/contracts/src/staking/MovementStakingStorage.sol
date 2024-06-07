// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;
import "forge-std/console.sol";
import { SafeERC20 } from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import { IERC20 } from "@openzeppelin/contracts/interfaces/IERC20.sol";

contract MovementStakingStorage {

    using SafeERC20 for IERC20;

    mapping(address => uint256) public epochDurationByDomain;
    mapping(address => uint256) public currentEpochByDomain;

    // the token used for staking
    IERC20 public token;

    // preserved records of stake by address per epoch
    mapping(address => 
        mapping(uint256 => 
            mapping(address => 
                mapping(address => uint256)))) public epochStakesByDomain;

    // preserved records of unstake by address per epoch
    mapping(address => 
        mapping(uint256 => 
            mapping(address =>
                mapping(address => uint256))))  public epochUnstakesByDomain;

    // track the total stake of the epoch (computed at rollover)
    mapping(address =>
        mapping(uint256 =>
            mapping(address=> uint256))) public epochTotalStakeByDomain;

}