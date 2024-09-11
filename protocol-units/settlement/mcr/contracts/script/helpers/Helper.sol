// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

contract Helper {
    // string to address
    function s2a(bytes memory str) public returns (address addr) {
        bytes32 data = keccak256(str);
        assembly {
            mstore(0, data)
            addr := mload(0)
        }
    }
}