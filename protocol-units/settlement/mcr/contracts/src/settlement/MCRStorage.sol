// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;
import {EnumerableSet} from "@openzeppelin/contracts/utils/structs/EnumerableSet.sol";
import "forge-std/console.sol";
import {MovementStaking, IMovementStaking} from "../staking/MovementStaking.sol";

contract MCRStorage {

    IMovementStaking public stakingContract;

    // the number of superBlocks that can be submitted ahead of the lastAcceptedSuperBlockHeight
    // this allows for things like batching to take place without some attesters locking down the attester set by pushing too far ahead
    // ? this could be replaced by a 2/3 stake vote on the superBlock height to epoch assignment
    // ? however, this protocol becomes more complex as you to take steps to ensure that...
    // ? 1. superBlock heights have a non-decreasing mapping to epochs
    // ? 2. Votes get accumulated reasonable near the end of the epoch (i.e., your vote is cast for the epoch you vote fore and the next)
    // ? if howevever, you simply allow a race with the tolerance below, both of these are satisfied without the added complexity
    // TODO the above explanation is not clear and needs to be rephrased or further explained
    // TODO unless this is clarified or becomes relevant in the future, this comment should be removed
    uint256 public leadingSuperBlockTolerance;

    // track the last accepted superBlock height, so that we can require superBlocks are submitted in order and handle staking effectively
    uint256 public lastAcceptedSuperBlockHeight;

    /// Acceptor term time in seconds (determined by L1 blocks). The confimer remains the same for acceptorTerm period.
    // This means we accept that if the acceptor is not active the postconfirmations will be delayed. 
    // TODO permit that anyone can confirm but only the Acceptor gets rewarded. 
    // TODO The Acceptor should also get rewarded even if another attestor confirmed the postconfirmation.
    // The Acceptor term can be minimal, but it should not be O(1) as the acceptor should have some time 
    // to prepare and post L1-transactions that will start the validation of attestations.
    uint256 public acceptorTerm;


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
    mapping(uint256 superBlockHeight => mapping(bytes32 commitement => uint256 stake)) public commitmentStakes;

    // Track which attester postconfirmed a given superBlock height
    mapping(uint256 superBlockHeight => address attester) public postconfirmedBy;

    // Track if acceptor postconfirmed a given superBlock height 
    // TODO this may be redundant due to one of the mappings below
    mapping(uint256 superBlockHeight => bool) public postconfirmedByAcceptor;

    // Track the L1Block height when a superBlock height was postconfirmed
    mapping(uint256 superBlockHeight => uint256 L1BlockHeight) public postconfirmedAtL1BlockHeight;

    // Track the L1Block height when a superBlock height was postconfirmed by the acceptor
    mapping(uint256 superBlockHeight => uint256 L1BlockHeight) public postconfirmedAtL1BlockHeightByAcceptor;

    // map superBlock height to accepted superBlock hash 
    mapping(uint256 superBlockHeight => SuperBlockCommitment) public acceptedSuperBlocks;

    // whether we allow open attestation
    bool public openAttestationEnabled;

    // versioned scheme for accepted superBlocks
    mapping(uint256 => mapping(uint256 superBlockHeight => SuperBlockCommitment)) public versionedAcceptedSuperBlocks;
    uint256 public acceptedSuperBlocksVersion;

    uint256[47] internal __gap;

}