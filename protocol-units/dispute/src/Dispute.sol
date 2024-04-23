// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Output, OutputLib, Receipt, ReceiptClaim, ReceiptClaimLib, IRiscZeroVerifier, SystemExitCode, ExitCode} from "./IRiscZeroVerifier.sol";
import {Groth16Verifier} from "./Groth16Verifier.sol";
import {SafeCast} from "openzeppelin/contracts/utils/math/SafeCast.sol";

contract RStarMDisputeSystem is IRISC0Verifier {
    struct Validator {
        bool isRegistered;
        uint256 stake;
    }

    struct OptimisticCommitment {
        bytes blockHash;
        bytes stateCommitment; // The state commitment associated with the block
        uint256 validatorCount; // Number of validators who have submitted this commitment
    }

    struct Dispute {
        bytes blockHash;
        uint256 timestamp;
        DisputeState state;
    }

    struct Proof {
        bytes32 blockHash; // Hash of the block for which this proof is relevant
        bool isValid; // Whether the proof has been verified as valid
        bool exists; // To check if the struct is populated
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

    uint256 public constant MIN_STAKE = 1 ether;
    uint256 public delta; // Time window for filing a dispute
    uint256 public p; // Time to run the zero-knowledge proof

    mapping(address => Validator) public validators;
    mapping(bytes => Dispute) public disputes;
    mapping(bytes32 => Proof) public verifiedProofs; // Maps blockHash to Proof

    IRISC0Verifier public verifier;

    event ValidatorRegistered(address indexed validator, uint256 stake);
    event ValidatorDeregistered(address indexed validator);
    event DisputeSubmitted(bytes indexed disputeHash, bytes blockHash, address indexed submitter);
    event DisputeResolved(bytes indexed disputeHash, DisputeState state);
    event ProofSubmitted(bytes32 indexed blockHash, bool isValid);
    event ProofVerified(bytes32 indexed blockHash, bool isValid);

    constructor(uint256 _delta, uint256 _p, address _verifier) {
        delta = _delta;
        p = _p;
        verifier = IRISC0Verifier(_verifier);
    }

    function registerValidator() external payable {
        require(msg.value >= MIN_STAKE, "Insufficient stake");
        require(!validators[msg.sender].isRegistered, "Validator already registered");
        validators[msg.sender] = Validator(true, msg.value);
        emit ValidatorRegistered(msg.sender, msg.value);
    }

    function deregisterValidator() external {
        Validator storage validator = validators[msg.sender];
        require(validator.isRegistered, "Validator not registered");
        require(validator.stake > 0, "No stake to withdraw");
        uint256 stake = validator.stake;
        validator.isRegistered = false;
        validator.stake = 0;
        payable(msg.sender).transfer(stake);
        emit ValidatorDeregistered(msg.sender);
    }

    function submitDispute(bytes calldata blockHash) external {
        bytes memory disputeHash = abi.encodePacked(blockHash, msg.sender, block.timestamp);
        require(disputes[disputeHash].timestamp == 0, "Dispute already submitted");
        disputes[disputeHash] = Dispute(blockHash, block.timestamp, DisputeState.SUBMITTED);
        emit DisputeSubmitted(disputeHash, blockHash, msg.sender);
    }

    function resolveDispute(bytes calldata disputeHash, DisputeState state) external {
        require(state > DisputeState.SUBMITTED && state <= DisputeState.UNVERIFIABLE, "Invalid state transition");
        Dispute storage dispute = disputes[disputeHash];
        require(dispute.timestamp != 0, "Dispute not found");
        require(dispute.state == DisputeState.SUBMITTED, "Dispute already resolved");
        dispute.state = state;
        emit DisputeResolved(disputeHash, state);
    }

    function submitProof(bytes32 blockHash, bytes calldata proof, bytes32[] calldata publicInputs) external {
        require(!verifiedProofs[blockHash].exists, "Proof already submitted for this block");
        bool isValid = verifier.verifyProof(proof, publicInputs);
        verifiedProofs[blockHash] = Proof(blockHash, isValid, true);
        emit ProofSubmitted(blockHash, isValid);
    }

    // Additional checks or logic could be implemented here based on the application's needs
    function verifyProof(bytes32 blockHash) external view returns (bool isValid, bool exists) {
        require(verifiedProofs[blockHash].exists, "Proof not found");
        return (verifiedProofs[blockHash].isValid, verifiedProofs[blockHash].exists);
    }

    function submitOptimisticCommitment(bytes32 blockHash, bytes calldata stateCommitment) external {
        require(validators[msg.sender].isRegistered, "Validator not registered");
        OptimisticCommitment storage commitment = optimisticCommitments[blockHash];
        if (commitment.validatorCount == 0) {
            commitment.blockHash = blockHash;
            commitment.stateCommitment = stateCommitment;
            commitment.validatorCount = 1;
        } else {
            require(keccak256(commitment.stateCommitment) == keccak256(stateCommitment), "State commitment mismatch");
            commitment.validatorCount += 1;
        }

        emit OptimisticCommitmentSubmitted(blockHash, stateCommitment, commitment.validatorCount);

        if (commitment.validatorCount >= m) {
            // Block is accepted optimistically after receiving minimum number of commitments
            emit BlockAccepted(blockHash);
        }
    }
}
