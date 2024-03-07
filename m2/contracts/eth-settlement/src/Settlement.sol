// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import {Output, OutputLib, Receipt, ReceiptClaim, ReceiptClaimLib} from "./IRiscZeroVerifier.sol";

/// @notice A Groth16 seal over the claimed receipt claim.
struct Seal {
    uint256[2] a;
    uint256[2][2] b;
    uint256[2] c;
}

// A toy settlement contract to post and retrieve proofs of settlements
contract Settlement {
    using ReceiptClaimLib for ReceiptClaim;
    using OutputLib for Output;

    struct Proof {
        bytes proofData;
        address signer; // Address of the signer who posted the proof
    }

    // Mapping from block height to array of Proofs
    mapping(uint64 => Proof[]) public proofsByHeight;

    // Mapping to keep track of allowed signers
    mapping(address => bool) public isSigner;

    event ProofAdded(uint64 indexed blockHeight, bytes proofData, address indexed signer);
    event SignerAdded(address indexed signer);
    event SignerRemoved(address indexed signer);

    // Scalar field size
    uint256 constant r = 21888242871839275222246405745257275088548364400416034343698204186575808495617;
    // Base field size
    uint256 constant q = 21888242871839275222246405745257275088696311157297823662689037894645226208583;

    // Verification Key data
    uint256 constant alphax = 20491192805390485299153009773594534940189261866228447918068658471970481763042;
    uint256 constant alphay = 9383485363053290200918347156157836566562967994039712273449902621266178545958;
    uint256 constant betax1 = 4252822878758300859123897981450591353533073413197771768651442665752259397132;
    uint256 constant betax2 = 6375614351688725206403948262868962793625744043794305715222011528459656738731;
    uint256 constant betay1 = 21847035105528745403288232691147584728191162732299865338377159692350059136679;
    uint256 constant betay2 = 10505242626370262277552901082094356697409835680220590971873171140371331206856;
    uint256 constant gammax1 = 11559732032986387107991004021392285783925812861821192530917403151452391805634;
    uint256 constant gammax2 = 10857046999023057135944570762232829481370756359578518086990519993285655852781;
    uint256 constant gammay1 = 4082367875863433681332203403145435568316851327593401208105741076214120093531;
    uint256 constant gammay2 = 8495653923123431417604973247489272438418190587263600148770280649306958101930;
    uint256 constant deltax1 = 18518940221910320856687047018635785128750837022059566906616608708313475199865;
    uint256 constant deltax2 = 9492326610711013918333865133991413442330971822743127449106067493230447878125;
    uint256 constant deltay1 = 19483644759748826533215810634368877792922012485854314246298395665859158607201;
    uint256 constant deltay2 = 21375251776817431660251933179512026180139877181625068362970095925425149918084;

    uint256 constant IC0x = 5283414572476013565779278723585415063371186194506872223482170607932178811733;
    uint256 constant IC0y = 18704069070102836155408936676819275373965966640372164023392964533091458933020;

    uint256 constant IC1x = 4204832149120840018317309580010992142700029278901617154852760187580780425598;
    uint256 constant IC1y = 12454324579480242399557363837918019584959512625719173397955145140913291575910;

    uint256 constant IC2x = 14956117485756386823219519866025248834283088288522682527835557402788427995664;
    uint256 constant IC2y = 6968527870554016879785099818512699922114301060378071349626144898778340839382;

    uint256 constant IC3x = 6512168907754184210144919576616764035747139382744482291187821746087116094329;
    uint256 constant IC3y = 17156131719875889332084290091263207055049222677188492681713268727972722760739;

    uint256 constant IC4x = 5195346330747727606774560791771406703229046454464300598774280139349802276261;
    uint256 constant IC4y = 16279160127031959334335024858510026085227931356896384961436876214395869945425;

    // Memory data
    uint16 constant pVk = 0;
    uint16 constant pPairing = 128;

    uint16 constant pLastMem = 896;

    /// @notice Control ID hash for the identity_p254 predicate decomposed by `splitDigest`.
    /// @dev This value controls what set of recursion programs, and therefore what version of the
    /// zkVM circuit, will be accepted by this contract. Each instance of this verifier contract
    /// will accept a single release of the RISC Zero circuits.
    ///
    /// New releases of RISC Zero's zkVM require updating these values. These values can be
    /// obtained by running `cargo run --bin bonsai-ethereum-contracts -F control-id`
    uint256 public immutable CONTROL_ID_0;
    uint256 public immutable CONTROL_ID_1; 


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
 

    function verifyProof(
        uint256[2] calldata _pA,
        uint256[2][2] calldata _pB,
        uint256[2] calldata _pC,
        uint256[4] calldata _pubSignals
    ) public view returns (bool) {

        assembly {
            function checkField(v) {
                if iszero(lt(v, q)) {
                    mstore(0, 0)
                    return(0, 0x20)
                }
            }

            // G1 function to multiply a G1 value(x,y) to value in an address
            function g1_mulAccC(pR, x, y, s) {
                let success
                let mIn := mload(0x40)
                mstore(mIn, x)
                mstore(add(mIn, 32), y)
                mstore(add(mIn, 64), s)

                success := staticcall(sub(gas(), 2000), 7, mIn, 96, mIn, 64)

                if iszero(success) {
                    mstore(0, 0)
                    return(0, 0x20)
                }

                mstore(add(mIn, 64), mload(pR))
                mstore(add(mIn, 96), mload(add(pR, 32)))

                success := staticcall(sub(gas(), 2000), 6, mIn, 128, pR, 64)

                if iszero(success) {
                    mstore(0, 0)
                    return(0, 0x20)
                }
            }

            function checkPairing(pA, pB, pC, pubSignals, pMem) -> isOk {
                let _pPairing := add(pMem, pPairing)
                let _pVk := add(pMem, pVk)

                mstore(_pVk, IC0x)
                mstore(add(_pVk, 32), IC0y)

                // Compute the linear combination vk_x

                g1_mulAccC(_pVk, IC1x, IC1y, calldataload(add(pubSignals, 0)))

                g1_mulAccC(_pVk, IC2x, IC2y, calldataload(add(pubSignals, 32)))

                g1_mulAccC(_pVk, IC3x, IC3y, calldataload(add(pubSignals, 64)))

                g1_mulAccC(_pVk, IC4x, IC4y, calldataload(add(pubSignals, 96)))

                // -A
                mstore(_pPairing, calldataload(pA))
                mstore(add(_pPairing, 32), mod(sub(q, calldataload(add(pA, 32))), q))

                // B
                mstore(add(_pPairing, 64), calldataload(pB))
                mstore(add(_pPairing, 96), calldataload(add(pB, 32)))
                mstore(add(_pPairing, 128), calldataload(add(pB, 64)))
                mstore(add(_pPairing, 160), calldataload(add(pB, 96)))

                // alpha1
                mstore(add(_pPairing, 192), alphax)
                mstore(add(_pPairing, 224), alphay)

                // beta2
                mstore(add(_pPairing, 256), betax1)
                mstore(add(_pPairing, 288), betax2)
                mstore(add(_pPairing, 320), betay1)
                mstore(add(_pPairing, 352), betay2)

                // vk_x
                mstore(add(_pPairing, 384), mload(add(pMem, pVk)))
                mstore(add(_pPairing, 416), mload(add(pMem, add(pVk, 32))))

                // gamma2
                mstore(add(_pPairing, 448), gammax1)
                mstore(add(_pPairing, 480), gammax2)
                mstore(add(_pPairing, 512), gammay1)
                mstore(add(_pPairing, 544), gammay2)

                // C
                mstore(add(_pPairing, 576), calldataload(pC))
                mstore(add(_pPairing, 608), calldataload(add(pC, 32)))

                // delta2
                mstore(add(_pPairing, 640), deltax1)
                mstore(add(_pPairing, 672), deltax2)
                mstore(add(_pPairing, 704), deltay1)
                mstore(add(_pPairing, 736), deltay2)

                let success := staticcall(sub(gas(), 2000), 8, _pPairing, 768, _pPairing, 0x20)

                isOk := and(success, mload(_pPairing))
            }

            let pMem := mload(0x40)
            mstore(0x40, add(pMem, pLastMem))

            // Validate that all evaluations ∈ F

            checkField(calldataload(add(_pubSignals, 0)))

            checkField(calldataload(add(_pubSignals, 32)))

            checkField(calldataload(add(_pubSignals, 64)))

            checkField(calldataload(add(_pubSignals, 96)))

            checkField(calldataload(add(_pubSignals, 128)))

            // Validate all evaluations
            let isValid := checkPairing(_pA, _pB, _pC, _pubSignals, pMem)

            mstore(0, isValid)
            return(0, 0x20)
        }
    }
}
