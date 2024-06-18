// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;
import {LockedTokenStorage} from "./LockedTokenStorage.sol";

contract LockedTokenStorage {
    bytes32 public constant MINT_LOCKER_ROLE = keccak256("MINT_LOCKER_ROLE");
    bytes32 public constant MINT_LOCKER_ADMIN_ROLE =
        keccak256("MINT_LOCKER_ADMIN_ROLE");

    struct Lock {
        uint256 amount;
        uint256 releaseTime;
    }
    mapping(address => Lock[]) public locks;

    error AddressesAndMintLengthMismatch();
    error AddressesAndLockLengthMismatch();
    error AddressesAndTimeLengthMismatch();
}
