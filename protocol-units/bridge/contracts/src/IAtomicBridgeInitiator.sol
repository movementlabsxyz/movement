// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

interface IAtomicBridgeInitiator {
    // Event emitted when a new atomic bridge transfer is created
    event BridgeTransferInitiated(bytes32 indexed _bridgeTransferId, address indexed _originator, address indexed _recipient, bytes32 _hashLock, uint _timeLock);
    // Event emitted when a BridgeTransfer is completed (withdrawn)
    event BridgeTransferCompleted(bytes32 indexed _bridgeTransferId, bytes32 _secret);
    // Event emitted when a BridgeTransfer is refunded
    event BridgeTransferRefunded(bytes32 indexed _bridgeTransferId);

    /**
     * @dev Creates a new atomic bridge transfer
     * @param amount The amount of assets to be allocated and sent
     * @param _originator The address allowed to withdraw (claim) the funds once the correct secret is provided on timeout. Used to transfer the funds.
     * @param _recipient The address on the other chain to which to transfer the funds
     * @param _hashLock The hash of the secret (HASH) that will unlock the funds
     * @param _timeLock The number of blocks until which this BridgeTransfer is valid and can be executed
     * @return _bridgeTransferId A unique id representing this BridgeTransfer
     **/
    function initiateBridgeTransfer(
      uint amount, 
      address _originator, 
      address _recipient, 
      bytes32 _hashLock, 
      uint _timeLock
    ) external payable returns (bytes32 _bridgeTransferId);

    /**
     * @dev Completes the bridging Counterparty
     * @param _bridgeTransferId Unique identifier for the BridgeTransfer
     * @param _secret The secret that unlocks the funds
     **/
    function completeBridgeTransfer(bytes32 _bridgeTransferId, bytes32 _secret) external;

    /**
     * @dev Refunds the funds back to the initiator if the timelock has expired
     * @param _bridgeTransferId Unique identifier for the BridgeTransfer
    **/
    function refundBridgeTransfer(bytes32 _bridgeTransferId) external;

    /**
     * @dev Returns the details of a specific bridge transfer
     * @param _bridgeTransferId Unique identifier for the bridge transfer
     * @return exists Boolean indicating if the bridge transfer exists
     * @return amount The amount of assets to be allocated and sent
     * @return originator The address allowed to withdraw (claim) the funds
     * @return recipient The address intended to receive the assets
     * @return hashLock The hash of the secret that will unlock the funds
     * @return timeLock The timestamp until which this BridgeTransfer is valid
    **/
    function getBridgeTransferDetail(bytes32 _bridgeTransferId) external view returns (
        bool exists, 
        uint amount, 
        address originator, 
        address recipient, 
        bytes32 hashLock, 
        uint timeLock
    );
}
