// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

interface IMCR {
    event BlockAccepted(
        bytes32 indexed blockHash,
        bytes32 stateCommitment,
        uint256 height
    );
    event BlockCommitmentSubmitted(
        bytes32 indexed blockHash,
        bytes32 stateCommitment,
        uint256 attesterStake
    );
    error UnacceptableBlockCommitment();
    error AttesterAlreadyCommitted();
}