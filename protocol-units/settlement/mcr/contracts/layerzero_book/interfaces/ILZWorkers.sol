// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

/// @title ILZWorkers
/// @notice Interface for LayerZero Workers Registry
interface ILZWorkers {
    function getDVNAddress(string memory dvnName, uint32 eid) external view returns (address dvnAddress);
    function getDVNAddressByChainName(string memory dvnName, string memory chainName) external view returns (address dvnAddress);
    function dvnExists(string memory dvnName, uint32 eid) external view returns (bool exists);
    function getAvailableDVNs() external view returns (string[] memory dvnNames);
    function getDVNsForChain(uint32 eid) external view returns (string[] memory names, address[] memory addresses);
    function getDVNsForChainByName(string memory chainName) external view returns (string[] memory names, address[] memory addresses);
}
