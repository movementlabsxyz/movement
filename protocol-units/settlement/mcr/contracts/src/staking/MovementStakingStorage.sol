// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;
import "forge-std/console.sol";
import { SafeERC20 } from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import { IERC20 } from "@openzeppelin/contracts/interfaces/IERC20.sol";
import { EnumerableSet } from "@openzeppelin/contracts/utils/structs/EnumerableSet.sol";

contract MovementStakingStorage {

    using SafeERC20 for IERC20;
    using EnumerableSet for EnumerableSet.AddressSet;

    // the token used for staking
    IERC20 public token;

    /// @dev the duration of each epoch in seconds. 
    /// The stakes are organized into epochs, and where epochs are measured in L1-block timestamps.
    mapping(address domain => uint256 epochDuration) public epochDurationByDomain;
    /// @dev the current epoch for each domain. Commitments are submitted only for the current epoch 
    /// and validators may not submit commitments to epochs that are far in the past. 
    /// Hence, we need to treat each epoch separately.
    mapping(address domain => uint256 currentEpoch) public currentEpochByDomain;
    mapping(address domain => EnumerableSet.AddressSet attester) internal attestersByDomain;
    mapping(address domain => EnumerableSet.AddressSet custodian) internal custodiansByDomain;

    // preserved records of stake by address per epoch
    mapping(address domain => 
        mapping(uint256 epoch => 
            mapping(address custodian => 
                mapping(address attester => uint256 stake)))) public epochStakesByDomain;

    // preserved records of unstake by address per epoch
    mapping(address domain => 
        mapping(uint256 epoch => 
            mapping(address custodian =>
                mapping(address attester => uint256 stake))))  public epochUnstakesByDomain;

    // track the total stake of the epoch (computed at rollover)
    mapping(address domain =>
        mapping(uint256 epoch =>
            mapping(address attester => uint256 stake))) public epochTotalStakeByDomain;

    mapping(address domain => bool) public domainGenesisAccepted;

    // the whitelist role needed to stake/unstake
    bytes32 public constant WHITELIST_ROLE = keccak256("WHITELIST_ROLE");
}