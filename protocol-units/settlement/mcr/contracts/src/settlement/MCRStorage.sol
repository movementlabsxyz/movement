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
    uint256 public leadingSuperBlockTolerance;

    // track the last accepted superBlock height, so that we can require superBlocks are submitted in order and handle staking effectively
    uint256 public lastAcceptedSuperBlockHeight;

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

    // map superBlock height to accepted superBlock hash 
    mapping(uint256 superBlockHeight => SuperBlockCommitment) public acceptedSuperBlocks;

    // whether we allow open attestation
    bool public openAttestationEnabled;

    // versioned scheme for accepted superBlocks
    mapping(uint256 => mapping(uint256 superBlockHeight => SuperBlockCommitment)) public versionedAcceptedSuperBlocks;
    uint256 public acceptedSuperBlocksVersion;

    uint256[47] internal __gap;

}