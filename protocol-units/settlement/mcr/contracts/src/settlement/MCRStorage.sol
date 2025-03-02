// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;
import {EnumerableSet} from "@openzeppelin/contracts/utils/structs/EnumerableSet.sol";
import "forge-std/console.sol";
import {MovementStaking, IMovementStaking} from "../staking/MovementStaking.sol";

contract MCRStorage {

    IMovementStaking public stakingContract;

    // The MOVE token address, which is the primary custodian for rewards in the staking contract
    address public moveTokenAddress;

    // the number of superBlocks that can be submitted ahead of the lastPostconfirmedSuperBlockHeight
    // this allows for things like batching to take place without some attesters locking down the attester set by pushing too far ahead
    // ? this could be replaced by a 2/3 stake vote on the superBlock height to epoch assignment
    // ? however, this protocol becomes more complex as you to take steps to ensure that...
    // ? 1. superBlock heights have a non-decreasing mapping to epochs
    // ? 2. Votes get accumulated reasonable near the end of the epoch (i.e., your vote is cast for the epoch you vote fore and the next)
    // ? if howevever, you simply allow a race with the tolerance below, both of these are satisfied without the added complexity
    // TODO the above explanation is not clear and needs to be rephrased or further explained
    // TODO unless this is clarified or becomes relevant in the future, this comment should be removed
    uint256 public leadingSuperBlockTolerance;

    // track the last postconfirmed superBlock height, so that we can require superBlocks are submitted in order and handle staking effectively
    uint256 public lastPostconfirmedSuperBlockHeight;

    /// Postconfirmer term time in seconds. The postconfirmer remains the same for postconfirmerDuration period.
    // The Postconfirmer term can be minimal, but it should not be too small as the postconfirmer should have some time 
    // to prepare and post L1-transactions that will start the validation of attestations.
    uint256 public postconfirmerDuration;

    /// @notice Minimum time that must pass before a commitment can be postconfirmed
    uint256 public minCommitmentAgeForPostconfirmation;

    /// @notice Max time the postconfirmer can be non-reactive to an honest superBlock commitment
    uint256 public maxPostconfirmerNonReactivityTime;

    // the postconfirmer for the accepting epoch
    address public currentPostconfirmer;

    // TODO i added these param descriptions. are these correct?
    /// Struct to store block commitment details
    /// @param height The height of the block
    /// @param commitment The hash of the committment
    /// @param blockId The unique identifier of the block (hash of the block)
    struct SuperBlockCommitment {
        // currently, to simplify the api, we'll say 0 is uncommitted all other numbers are legitimate heights
        uint256 height;
        bytes32 commitment;
        bytes32 blockId;
    }

    // map each superBlock height to an epoch
    mapping(uint256 superBlockHeight => uint256 epoch) public superBlockHeightAssignedEpoch;

    // track each commitment from each attester for each superBlock height
    mapping(uint256 superBlockHeight => mapping(address attester => SuperBlockCommitment)) public commitments;

    // track the total stake accumulate for each commitment for each superBlock height
    mapping(uint256 superBlockHeight => mapping(bytes32 commitement => uint256 stake)) public commitmentStake;

    // track when each commitment was first seen for each superBlock height
    mapping(uint256 superBlockHeight => mapping(bytes32 commitment => uint256 timestamp)) public commitmentFirstSeenAt;

    // Track which attester postconfirmed a given superBlock height
    mapping(uint256 superBlockHeight => address attester) public postconfirmedBy;

    // Track if postconfirmer postconfirmed a given superBlock height 
    // TODO this may be redundant due to one of the mappings below
    mapping(uint256 superBlockHeight => bool) public postconfirmedByPostconfirmer;

    // Track the L1Block height when a superBlock height was postconfirmed
    mapping(uint256 superBlockHeight => uint256 L1BlockHeight) public postconfirmedAtL1BlockHeight;

    // TODO: either the L1Block timestamp or L1Block height should be tracked, both are not needed, but keep both until we know which one is not needed
    // Track the L1Block timestamp when a superBlock height was postconfirmed
    mapping(uint256 superBlockHeight => uint256 L1BlockTimestamp) public postconfirmedAtL1BlockTimestamp;

    // Track the L1Block height when a superBlock height was postconfirmed by the postconfirmer
    mapping(uint256 superBlockHeight => uint256 L1BlockHeight) public postconfirmedAtL1BlockHeightByPostconfirmer;

    // map superBlock height to postconfirmed superBlock hash 
    mapping(uint256 superBlockHeight => SuperBlockCommitment) public postconfirmedSuperBlocks;

    // whether we allow open attestation
    bool public openAttestationEnabled;

    // versioned scheme for postconfirmed superBlocks
    mapping(uint256 => mapping(uint256 superBlockHeight => SuperBlockCommitment)) public versionedPostconfirmedSuperBlocks;
    uint256 public postconfirmedSuperBlocksVersion;

    // track reward points for attesters
    mapping(uint256 epoch => mapping(address attester => uint256 points)) public attesterRewardPoints;

    // track reward points for postconfirmers
    mapping(uint256 epoch => mapping(address postconfirmer => uint256 points)) public postconfirmerRewardPoints;

    // track the reward per point for attesters
    uint256 public rewardPerAttestationPoint;

    // track the reward per point for postconfirmers
    uint256 public rewardPerPostconfirmationPoint;

    uint256[45] internal __gap; // Reduced by 1 for new mapping

}