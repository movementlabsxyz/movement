// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

// A toy settlement contract to post and retrieve proofs of settlements
contract Settlement {
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

    // Modifier to restrict function calls to allowed signers
    modifier onlySigner() {
        require(isSigner[msg.sender], "Caller is not an allowed signer");
        _;
    }

    constructor() {
        // Initialize the contract with the deployer as an allowed signer
        isSigner[msg.sender] = true;
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

}
