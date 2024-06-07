// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;
import "@openzeppelin/contracts/utils/structs/EnumerableSet.sol";
import "forge-std/console.sol";
import "./base/BaseStaking.sol";
import { SafeERC20 } from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import { IERC20 } from "@openzeppelin/contracts/interfaces/IERC20.sol";
import { ICustodianToken } from "../token/custodian/CustodianToken.sol";
import { Math } from "@openzeppelin/contracts/utils/math/Math.sol";

contract MovementStaking is IMovementStaking, BaseStaking {

    using SafeERC20 for IERC20;

    // Use an address set here
    using EnumerableSet for EnumerableSet.AddressSet;

    mapping(address => uint256) public epochDurationByDomain;
    mapping(address => uint256) public currentEpochByDomain;

    // the token used for staking
    IERC20 public token;

    // the current epoch
    mapping(address => EnumerableSet.AddressSet) private attestersByDomain;

    // the custodians allowed by each domain
    mapping(address => EnumerableSet.AddressSet) private custodiansByDomain;

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