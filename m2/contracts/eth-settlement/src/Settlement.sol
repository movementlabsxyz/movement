// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import {Output, OutputLib, Receipt, ReceiptClaim, ReceiptClaimLib, IRiscZeroVerifier, SystemExitCode, ExitCode} from "./IRiscZeroVerifier.sol";
import {Groth16Verifier} from "./Groth16Verifier.sol";
import {SafeCast} from "openzeppelin/contracts/utils/math/SafeCast.sol";

/// @notice A Groth16 seal over the claimed receipt claim.
struct Seal {
    uint256[2] a;
    uint256[2][2] b;
    uint256[2] c;
}

// A toy settlement contract to post and retrieve proofs of settlements
contract Settlement is IRiscZeroVerifier, Groth16Verifier {
    using ReceiptClaimLib for ReceiptClaim;
    using OutputLib for Output;
    using SafeCast for uint256;

    struct Proof {
        bytes proofData;
        address signer; // Address of the signer who posted the proof
    }

    /// @notice Control ID hash for the identity_p254 predicate decomposed by `splitDigest`.
    /// @dev This value controls what set of recursion programs, and therefore what version of the
    /// zkVM circuit, will be accepted by this contract. Each instance of this verifier contract
    /// will accept a single release of the RISC Zero circuits.
    ///
    /// New releases of RISC Zero's zkVM require updating these values. These values can be
    /// obtained by running `cargo run --bin bonsai-ethereum-contracts -F control-id`
    uint256 public immutable CONTROL_ID_0;
    uint256 public immutable CONTROL_ID_1;

    // Mapping from block height to array of Proofs
    mapping(uint64 => Proof[]) public proofsByHeight;

    // Mapping to keep track of allowed signers
    mapping(address => bool) public isSigner;

    event ProofAdded(uint64 indexed blockHeight, bytes proofData, address indexed signer);
    event SignerAdded(address indexed signer);
    event SignerRemoved(address indexed signer);

    // Modifier to restrict function calls to allowed signers
    modifier onlySigner() {
        require(isSigner[msg.sender], "Caller is not an allowed signer");
        _;
    }

    constructor(uint256 control_id_0, uint256 control_id_1) {
        // Initialize the contract with the deployer as an allowed signer
        isSigner[msg.sender] = true;
        CONTROL_ID_0 = control_id_0;
        CONTROL_ID_1 = control_id_1;
    }

    // Function to add a signer
    function addSigner(address _signer) external { // todo: change back to only signer
        isSigner[_signer] = true;
        emit SignerAdded(_signer);
    }

    // Function to remove a signer
    function removeSigner(address _signer) external { // todo: change back to only signe
        isSigner[_signer] = false;
        emit SignerRemoved(_signer);
    }

    // Function to post a settlement
    function settle(uint64 blockHeight, bytes calldata proofData) external { // todo: change back to only signer
        proofsByHeight[blockHeight].push(Proof(proofData, msg.sender));
        emit ProofAdded(blockHeight, proofData, msg.sender);
    }

    // Function to get settlements by block height
    function getProofsAtHeight(uint64 blockHeight) external view returns (bytes[] memory) {
        Proof[] memory proofs = proofsByHeight[blockHeight];
        bytes[] memory proofData = new bytes[](proofs.length);
        for (uint i = 0; i < proofs.length; i++) {
            proofData[i] = proofs[i].proofData;
        }
        return proofData;
    }

    /// @notice splits a digest into two 128-bit words to use as public signal inputs.
    /// @dev RISC Zero's Circom verifier circuit takes each of two hash digests in two 128-bit
    /// chunks. These values can be derived from the digest by splitting the digest in half and
    /// then reversing the bytes of each.
    function splitDigest(bytes32 digest) internal pure returns (uint256, uint256) {
        uint256 reversed = reverseByteOrderUint256(uint256(digest));
        return (uint256(uint128(uint256(reversed))), uint256(reversed >> 128));
    }

    /// @inheritdoc IRiscZeroVerifier
    function verify(bytes calldata seal, bytes32 imageId, bytes32 postStateDigest, bytes32 journalDigest)
        public
        view
        returns (bool)
    {
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
        return verifyIntegrity(receipt);
    }

    function verifyIntegrity(Receipt memory receipt) public view returns (bool) {
        (uint256 claim0, uint256 claim1) = splitDigest(receipt.claim.digest());
        Seal memory seal = abi.decode(receipt.seal, (Seal));
        return this.verifyProof(seal.a, seal.b, seal.c, [CONTROL_ID_0, CONTROL_ID_1, claim0, claim1]);
    }

    /// @notice reverse the byte order of the uint256 value.
    /// @dev Soldity uses a big-endian ABI encoding. Reversing the byte order before encoding
    /// ensure that the encoded value will be little-endian.
    /// Written by k06a. https://ethereum.stackexchange.com/a/83627
    function reverseByteOrderUint256(uint256 input) public pure returns (uint256 v) {
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
}
