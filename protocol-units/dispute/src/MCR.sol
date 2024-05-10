// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Output, OutputLib, Receipt, ReceiptClaim, ReceiptClaimLib, IRiscZeroVerifier, SystemExitCode, ExitCode} from "./IRiscZeroVerifier.sol";
import {SafeCast} from "openzeppelin-contracts/contracts/utils/math/SafeCast.sol";
import "forge-std/console2.sol";

contract MCR {
    using ReceiptClaimLib for ReceiptClaim;
    using OutputLib for Output;
    using SafeCast for uint256;

    struct Validator {
        bool isRegistered;
        uint256 stake;
    }

    struct OptimisticCommitment {
        bytes32 blockHash;
        uint256 stateCommitmentCount;
        uint256 totalStake;
        bool isAccepted;
    }

    struct Dispute {
        bytes32 blockHash;
        uint256 timestamp;
        DisputeState state;
    }

    struct Proof {
        bytes32 blockHash;
        bool isValid;
        bool exists;
    }

    enum DisputeState {
        SUBMITTED,
        VALID,
        INVALID,
        ACCEPTED,
        REJECTED,
        CORRECT,
        INCORRECT,
        UNVERIFIABLE
    }

    uint256 public constant SECONDS_IN_DAY = 86400;
    uint256 public constant SECONDS_IN_MINUTE = 60;

    uint256 public currentEpoch;
    uint256 public epochDuration;
    uint256 public constant MIN_STAKE = 1 ether;
    uint256 public delta = 1 * SECONDS_IN_DAY;
    uint256 public p = 1 * SECONDS_IN_MINUTE;
    uint256 public supermajorityStake;
    uint256 public epochStartTimestamp;

    mapping(address => Validator) public validators;
    mapping(bytes32 => Dispute) public disputes;
    mapping(bytes32 => Proof) public verifiedProofs;
    mapping(bytes32 => OptimisticCommitment) public optimisticCommitments;
    mapping(uint256 => OptimisticCommitment) public epochs;

    IRiscZeroVerifier public verifier;

    event ValidatorRegistered(address indexed validator, uint256 stake);
    event ValidatorDeregistered(address indexed validator);
    event DisputeSubmitted(bytes32 indexed disputeHash, bytes32 blockHash, address indexed submitter);
    event DisputeResolved(bytes32 indexed disputeHash, DisputeState state);
    event ProofSubmitted(bytes32 indexed blockHash, bool isValid);
    event ProofVerified(bytes32 indexed blockHash, bool isValid);
    event BlockAccepted(bytes32 indexed blockHash, bytes stateCommitment);
    event OptimisticCommitmentSubmitted(bytes32 indexed blockHash, bytes stateCommitment, uint256 validatorStake);

    constructor(
        uint256 _delta,
        uint256 _supermajorityStake,
        uint256 _epochDurationInDays
    ) {
        delta = _delta;
        supermajorityStake = _supermajorityStake;
        epochDuration = _epochDurationInDays * SECONDS_IN_DAY;
    }

    function updateEpoch() public {
        uint256 epochsPassed = (block.timestamp - epochStartTimestamp) / epochDuration;
        currentEpoch += epochsPassed;
        epochStartTimestamp += epochsPassed * epochDuration;
    }

    function getCurrentEpoch() public view returns (uint256) {
        uint256 epochsPassed = (block.timestamp - epochStartTimestamp) / epochDuration;
        return currentEpoch + epochsPassed;
    }

    function stake() external payable {
        validators[msg.sender].isRegistered = true;
        validators[msg.sender].stake += msg.value;
        emit ValidatorRegistered(msg.sender, msg.value);
    }

    function unstake(uint256 amount) external {
        Validator storage validator = validators[msg.sender];
        require(validator.isRegistered, "Validator not registered");
        require(validator.stake >= amount, "Insufficient stake");
        validator.stake -= amount;
        if (validator.stake == 0) {
            validator.isRegistered = false;
        }
        payable(msg.sender).transfer(amount);
        emit ValidatorDeregistered(msg.sender);
    }

    function getValidatorStatus() external view returns (bool, uint256) {
        return (validators[msg.sender].isRegistered, validators[msg.sender].stake);
    }

    function submitDispute(bytes32 blockHash) external {
        bytes32 disputeHash = keccak256(abi.encodePacked(blockHash, msg.sender, block.timestamp));
        require(disputes[disputeHash].timestamp == 0, "Dispute already submitted");
        disputes[disputeHash] = Dispute(blockHash, block.timestamp, DisputeState.SUBMITTED);
        emit DisputeSubmitted(disputeHash, blockHash, msg.sender);
    }

    function resolveDispute(bytes32 disputeHash, DisputeState state) external {
        require(state > DisputeState.SUBMITTED && state <= DisputeState.UNVERIFIABLE, "Invalid state transition");
        Dispute storage dispute = disputes[disputeHash];
        require(dispute.timestamp != 0, "Dispute not found");
        require(dispute.state == DisputeState.SUBMITTED, "Dispute already resolved");
        dispute.state = state;
        emit DisputeResolved(disputeHash, state);
    }

    function isCommitmentAccepted(uint256 epoch) public view returns (bool) {
        return epochs[epoch].isAccepted;
    }

    function submitOptimisticCommitment(bytes32 blockHash, bytes calldata stateCommitment) external {
        require(validators[msg.sender].isRegistered, "Validator not registered");

        updateEpoch();

        OptimisticCommitment storage commitment = epochs[currentEpoch];
        commitment.blockHash = blockHash;

        commitment.stateCommitmentCount++;
        commitment.totalStake += validators[msg.sender].stake;

        if (!commitment.isAccepted) {
            if (commitment.totalStake >= supermajorityStake) {
                commitment.isAccepted = true;
                emit BlockAccepted(blockHash, stateCommitment);

                currentEpoch++;
                OptimisticCommitment storage newCommitment = epochs[currentEpoch];
                newCommitment.blockHash = bytes32(0);
                newCommitment.stateCommitmentCount = 0;
                newCommitment.totalStake = 0;
                newCommitment.isAccepted = false;
            }
        }

        emit OptimisticCommitmentSubmitted(blockHash, stateCommitment, validators[msg.sender].stake);
    }
}