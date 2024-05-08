// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Output, OutputLib, Receipt, ReceiptClaim, ReceiptClaimLib, IRiscZeroVerifier, SystemExitCode, ExitCode} from "./IRiscZeroVerifier.sol";
import {Groth16Verifier} from "./groth16/Groth16Verifier.sol";
import {SafeCast} from "openzeppelin-contracts/contracts/utils/math/SafeCast.sol";
import "forge-std/console2.sol";

/// @notice reverse the byte order of the uint256 value.
/// @dev Soldity uses a big-endian ABI encoding. Reversing the byte order before encoding
/// ensure that the encoded value will be little-endian.
/// Written by k06a. https://ethereum.stackexchange.com/a/83627
function reverseByteOrderUint256(uint256 input) pure returns (uint256 v) {
    v = input;

    // swap bytes
    v = ((v & 0xFF00FF00FF00FF00FF00FF00FF00FF00FF00FF00FF00FF00FF00FF00FF00FF00) >> 8)
        | ((v & 0x00FF00FF00FF00FF00FF00FF00FF00FF00FF00FF00FF00FF00FF00FF00FF00FF) << 8);

    // swap 2-byte long pairs
    v = ((v & 0xFFFF0000FFFF0000FFFF0000FFFF0000FFFF0000FFFF0000FFFF0000FFFF0000) >> 16)
        | ((v & 0x0000FFFF0000FFFF0000FFFF0000FFFF0000FFFF0000FFFF0000FFFF0000FFFF) << 16);

    // swap 4-byte long pairs
    v = ((v & 0xFFFFFFFF00000000FFFFFFFF00000000FFFFFFFF00000000FFFFFFFF00000000) >> 32)
        | ((v & 0x00000000FFFFFFFF00000000FFFFFFFF00000000FFFFFFFF00000000FFFFFFFF) << 32);

    // swap 8-byte long pairs
    v = ((v & 0xFFFFFFFFFFFFFFFF0000000000000000FFFFFFFFFFFFFFFF0000000000000000) >> 64)
        | ((v & 0x0000000000000000FFFFFFFFFFFFFFFF0000000000000000FFFFFFFFFFFFFFFF) << 64);

    // swap 16-byte long pairs
    v = (v >> 128) | (v << 128);
}

/// @notice reverse the byte order of the uint32 value.
/// @dev Soldity uses a big-endian ABI encoding. Reversing the byte order before encoding
/// ensure that the encoded value will be little-endian.
/// Written by k06a. https://ethereum.stackexchange.com/a/83627
function reverseByteOrderUint32(uint32 input) pure returns (uint32 v) {
    v = input;

    // swap bytes
    v = ((v & 0xFF00FF00) >> 8) | ((v & 0x00FF00FF) << 8);

    // swap 2-byte long pairs
    v = (v >> 16) | (v << 16);
}

/// @notice A Groth16 seal over the claimed receipt claim.
struct Seal {
    uint256[2] a;
    uint256[2][2] b;
    uint256[2] c;
}

contract RStarM is IRiscZeroVerifier, Groth16Verifier {
    using ReceiptClaimLib for ReceiptClaim;
    using OutputLib for Output;
    using SafeCast for uint256;

    struct Validator {
        bool isRegistered;
        uint256 stake;
    }

    struct OptimisticCommitment {
        bytes32 blockHash;
        mapping(bytes => uint256) stateCommitments;
        bytes highestCommitState;
        uint256 highestCommitCount;
        uint256 agreeingValidatorCount; // Count of validators agreeing on the highest commit state
        bool isAccepted; // Flag indicating if the state is accepted
    }

    struct Dispute {
        bytes32 blockHash;
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

    uint256 public constant SECONDS_IN_DAY = 86400; // Number of seconds in a day
    uint256 public constant SECONDS_IN_MINUTE = 60; // Number of seconds in a minute


    uint256 public currentRound;
    uint256 public constant MIN_STAKE = 1 ether;
    uint256 public delta = 1 * SECONDS_IN_DAY; // Time window for filing a dispute (e.g., 1 day)
    uint256 public p = 1 * SECONDS_IN_MINUTE; 
    uint256 public m; // Minimum number of validators requried

    /// @notice Control ID hash for the identity_p254 predicate decomposed by `splitDigest`.
    /// @dev This value controls what set of recursion programs, and therefore what version of the
    /// zkVM circuit, will be accepted by this contract. Each instance of this verifier contract
    /// will accept a single release of the RISC Zero circuits.
    ///
    /// New releases of RISC Zero's zkVM require updating these values. These values can be
    /// obtained by running `cargo run --bin bonsai-ethereum-contracts -F control-id`
    uint256 public immutable CONTROL_ID_0;
    uint256 public immutable CONTROL_ID_1;
    uint256 public immutable BN254_CONTROL_ID;

    mapping(address => Validator) public validators;
    mapping(bytes32 => Dispute) public disputes;
    mapping(bytes32 => Proof) public verifiedProofs; // Maps blockHash to Proof
    mapping(bytes32 => OptimisticCommitment) public optimisticCommitments; // Maps blockHash to OptimisticCommitment
    mapping(uint256 => mapping(bytes => OptimisticCommitment)) public roundCommitments; //Maps round to blockHash to OptimisticCommitment


    IRiscZeroVerifier public verifier;

    event ValidatorRegistered(address indexed validator, uint256 stake);
    event ValidatorDeregistered(address indexed validator);
    event DisputeSubmitted(bytes32 indexed disputeHash, bytes32 blockHash, address indexed submitter);
    event DisputeResolved(bytes32 indexed disputeHash, DisputeState state);
    event ProofSubmitted(bytes32 indexed blockHash, bool isValid);
    event ProofVerified(bytes32 indexed blockHash, bool isValid);
    event BlockAccepted(bytes32 indexed blockHash, bytes stateCommitment);
    event OptimisticCommitmentSubmitted(bytes32 indexed blockHash, bytes stateCommitment, uint256 validatorCount);

    constructor(
        uint256 _delta, 
        uint256 _p, 
        uint256 _m, 
        uint256 control_id_0, 
        uint256 control_id_1,
        uint256 bn254_control_id
    ) {
        delta = _delta;
        p = _p;
        m = _m;
        CONTROL_ID_0 = control_id_0;
        CONTROL_ID_1 = control_id_1;
        BN254_CONTROL_ID = bn254_control_id;
    }

    /// @notice splits a digest into two 128-bit words to use as public signal inputs.
    /// @dev RISC Zero's Circom verifier circuit takes each of two hash digests in two 128-bit
    /// chunks. These values can be derived from the digest by splitting the digest in half and
    /// then reversing the bytes of each.
    function splitDigest(bytes32 digest) internal pure returns (uint256, uint256) {
        uint256 reversed = reverseByteOrderUint256(uint256(digest));
        return (uint256(uint128(uint256(reversed))), uint256(reversed >> 128));
    }

    function stake() external payable {
        require(msg.value >= MIN_STAKE, "Insufficient stake");
        require(!validators[msg.sender].isRegistered, "Validator already registered");
        validators[msg.sender] = Validator(true, msg.value);
        emit ValidatorRegistered(msg.sender, msg.value);
    }

    function getValidator(address validator) external view returns (bool, uint256) {
        return (validators[validator].isRegistered, validators[validator].stake);
    }

    function unstake() external {
        Validator storage validator = validators[msg.sender];
        require(validator.isRegistered, "Validator not registered");
        require(validator.stake > 0, "No stake to withdraw");
        uint256 validatorStake = validator.stake;
        validator.isRegistered = false;
        validator.stake = 0;
        payable(msg.sender).transfer(validatorStake);
        emit ValidatorDeregistered(msg.sender);
    }

    // We probably want to use bytes calldata here to reduce gas but going with bytes32 for now as we 
    // need it elsewhere.
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

    function submitProof(bytes32 blockHash, Receipt calldata receipt) external {
        require(!verifiedProofs[blockHash].exists, "Proof already submitted for this block");
        bool isValid = verifier.verify_integrity(receipt);
        verifiedProofs[blockHash] = Proof(blockHash, isValid, true);
        emit ProofSubmitted(blockHash, isValid);
    }

    function verify(bytes calldata seal, bytes32 imageId, bytes32 postStateDigest, bytes32 journalDigest)
        public
        view
        returns (bool)
    {
        //require(verifiedProofs[blockHash].exists, "Proof not found");
        Receipt memory receipt = Receipt(
            seal,
            ReceiptClaim(
                imageId,
                postStateDigest,
                ExitCode(SystemExitCode.Halted, 0),
                bytes32(0),
                Output(journalDigest, bytes32(0)).digest()
            )
        );
        return grothVerify(receipt);
    }

    function grothVerify(Receipt memory receipt) public view returns (bool) {
        (uint256 claim0, uint256 claim1) = splitDigest(receipt.claim.digest());
        Seal memory seal = abi.decode(receipt.seal, (Seal));
        return this.verifyProof(seal.a, seal.b, seal.c, [CONTROL_ID_0, CONTROL_ID_1, claim0, claim1, BN254_CONTROL_ID]);
    }

    // The camel case here is not standard solidity practice. But we use it because its the implemntation of the interface.
    function verify_integrity(Receipt memory receipt) public view returns (bool) {
        (uint256 claim0, uint256 claim1) = splitDigest(receipt.claim.digest());
        Seal memory seal = abi.decode(receipt.seal, (Seal));
        bool is_verified = this.verifyProof(seal.a, seal.b, seal.c, [CONTROL_ID_0, CONTROL_ID_1, claim0, claim1, BN254_CONTROL_ID]);
        return is_verified;
    }

    // Gets the stake amount for the current round which exponentially increases with each round.
    function getStakeAmount() public view returns (uint256) {
        uint256 incrementFactor = 2 ** currentRound;
        return MIN_STAKE * (100 + incrementFactor) / 100;
    }

    // Submit an optimistic commitment
    function submitOptimisticCommitment(bytes32 blockHash, bytes calldata stateCommitment) external payable {
        require(validators[msg.sender].isRegistered, "Validator not registered");

        uint256 requiredStake = getStakeAmount();
        require(msg.value >= requiredStake, "Insufficient stake for the current round");

        OptimisticCommitment storage commitment = roundCommitments[currentRound][stateCommitment];
        commitment.blockHash = blockHash;

        // Increment the count for the submitted stateCommitment
        uint256 currentCount = ++commitment.stateCommitments[stateCommitment];

        // Update the highest commit count and state if the current count is higher
        if (currentCount > commitment.highestCommitCount) {
            commitment.highestCommitCount = currentCount;
            commitment.highestCommitState = stateCommitment;
        }

        if (!commitment.isAccepted) {
            if (commitment.highestCommitCount >= m) {
                commitment.isAccepted = true;
                emit BlockAccepted(blockHash, commitment.highestCommitState);
            }
        } else {
            // Convert stateCommitment and highestCommitState to bytes memory for comparison
            bytes memory stateCommitmentBytes = bytes(stateCommitment);
            bytes memory highestCommitStateBytes = bytes(commitment.highestCommitState);

            // Update the agreeing validator count if the submitted commitment matches the highest commit state
            if (keccak256(stateCommitmentBytes) == keccak256(highestCommitStateBytes)) {
                commitment.agreeingValidatorCount++;
            }
        }

        emit OptimisticCommitmentSubmitted(blockHash, stateCommitment, currentCount);

        // Move to the next round if the block is not accepted
        if (!commitment.isAccepted) {
            currentRound++;
        }
    }
} 
