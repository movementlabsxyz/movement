// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

/// @title ILZProtocol
/// @notice Interface for LayerZero Protocol Address Provider
interface ILZProtocol {
    struct ProtocolAddresses {
        address endpointV2;
        address sendUln302;
        address receiveUln302;
        address executor;
        uint256 chainId;
        string chainName;
        bool exists;
    }

    function getProtocolAddresses(uint32 eid) external view returns (ProtocolAddresses memory);
    function getEidByChainId(uint256 chainId) external view returns (uint32);
    function isChainSupported(uint32 eid) external view returns (bool);
    function getSupportedEids() external view returns (uint32[] memory);
}
