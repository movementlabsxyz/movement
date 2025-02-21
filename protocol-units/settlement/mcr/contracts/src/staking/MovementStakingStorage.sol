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

    mapping(address domain => uint256 epochDuration) public epochDurationByDomain;
    mapping(address domain => uint256 currentAcceptingEpoch) public currentAcceptingEpochByDomain;
    mapping(address domain => EnumerableSet.AddressSet attester) internal attestersByDomain;
    mapping(address domain => EnumerableSet.AddressSet custodian) internal custodiansByDomain;

    // preserved records of stake by address per epoch
    /// @dev this is a mapping of domain => epoch => custodian => attester => stake
    mapping(address domain => 
        mapping(uint256 epoch => 
            mapping(address custodian => 
                mapping(address attester => uint256 stake)))) public stakesByDomainEpochCustodianAttester;

    // preserved records of unstake by address per epoch
    /// @dev this is a mapping of domain => epoch => custodian => attester => unstake
    mapping(address domain => 
        mapping(uint256 epoch => 
            mapping(address custodian =>
                mapping(address attester => uint256 stake))))  public unstakesByDomainEpochCustodianAttester;

    // track the total stake of the epoch (computed at rollover)
    /// @dev this is a mapping of domain => epoch => custodian => stake
    mapping(address domain =>
        mapping(uint256 epoch =>
            mapping(address custodian => uint256 stake))) public stakesByDomainEpochCustodian;

    mapping(address domain => bool) public domainGenesisAccepted;

    // the whitelist role needed to stake/unstake
    bytes32 public constant WHITELIST_ROLE = keccak256("WHITELIST_ROLE");
}