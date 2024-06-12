import "./IAtomicBridgeInitiator.sol";
import "./WETH/interfaces/IWETH10.sol";
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

contract AtomicBridgeInitiator is IAtomicBridgeInitiator {
    struct BridgeTransfer {
        uint256 amount;
        address originator;
        address recipient;
        bytes32 hashLock;
        uint timeLock;
    }

    mapping(bytes32 => BridgeTransfer) public bridgeTransfers;
    mapping(bytes32 => BridgeTransfer) public completedBridgeTransfers;
    IWETH10 public weth;

    constructor(address _weth) {
        weth = IWETH10(_weth);
    }

    function initiateBridgeTransferWithEth(
        address _originator, 
        address _recipient, 
        bytes32 _hashLock, 
        uint _timeLock
    ) external payable override returns (bytes32 _bridgeTransferId) {
        require(msg.value > 0, "ETH amount must be greater than 0");
        uint256 wethAmount = msg.value;

        // Wrap ETH into WETH and store the WETH in this contract
        weth.deposit{value: msg.value}();
        require(weth.transfer(address(this), msg.value), "WETH transfer failed");
        
        _bridgeTransferId = keccak256(
            abi.encodePacked(
                _originator, 
                _recipient, 
                _hashLock, 
                _timeLock, 
                block.timestamp
        ));
        require(!bridgeTransfers[_bridgeTransferId].exists, "Bridge transfer already exists");

        bridgeTransfers[_bridgeTransferId] = BridgeTransfer({
            amount: wethAmount,
            originator: _originator,
            recipient: _recipient,
            hashLock: _hashLock,
            timeLock: block.timestamp + _timeLock
        });

        emit BridgeTransferInitiated(_bridgeTransferId, _originator, _recipient, _hashLock, _timeLock);
        return _bridgeTransferId;
    }

    function initatieBridgeTransferWithWeth(
        uint256 _wethAmount, 
        address _originator, 
        address _recipient, 
        bytes32 _hashLock, 
        uint _timeLock
    ) external override returns (bytes32 _bridgeTransferId) {
        require(_wethAmount > 0, "WETH amount must be greater than 0");

        //Transfer WETH from the sender to this contract
        require(weth.transfer(address(this), _wethAmount), "WETH transfer failed");

        _bridgeTransferId = keccak256(
            abi.encodePacked(
                _originator, 
                _recipient, 
                _hashLock, 
                _timeLock, 
                block.timestamp
        ));
        require(!bridgeTransfers[_bridgeTransferId].exists, "Bridge transfer already exists");

        bridgeTransfers[_bridgeTransferId] = BridgeTransfer({
            amount: _wethAmount,
            originator: _originator,
            recipient: _recipient,
            hashLock: _hashLock,
            timeLock: block.timestamp + _timeLock
        });

        emit BridgeTransferInitiated(_bridgeTransferId, _originator, _recipient, _hashLock, _timeLock);
        return _bridgeTransferId;
    }

    function completeBridgeTransfer(bytes32 _bridgeTransferId, bytes32 _secret) external override {
        BridgeTransfer storage bridgeTransfer = bridgeTransfers[_bridgeTransferId];
        require(bridgeTransfer.exists, "Bridge transfer does not exist");
        require(keccak256(abi.encodePacked(_secret)) == bridgeTransfer.hashLock, "Invalid secret");

        // Move the bridge transfer to completed
        completedBridgeTransfers[_bridgeTransferId] = bridgeTransfer;

        // Delete from active bridgeTransfers
        delete bridgeTransfers[_bridgeTransferId];

        payable(bridgeTransfer.recipient).transfer(bridgeTransfer.amount);

        emit BridgeTransferCompleted(_bridgeTransferId, _secret);
    }

    function refundBridgeTransfer(bytes32 _bridgeTransferId) external override {
        BridgeTransfer storage bridgeTransfer = bridgeTransfers[_bridgeTransferId];
        require(bridgeTransfer.exists, "Bridge transfer does not exist");
        require(block.timestamp > bridgeTransfer.timeLock, "Timelock has not expired");

        delete bridgeTransfers[_bridgeTransferId];

        payable(bridgeTransfer.originator).transfer(bridgeTransfer.amount);

        emit BridgeTransferRefunded(_bridgeTransferId);
    }

    function getBridgeTransferDetail(bytes32 _bridgeTransferId) external view override returns (
        bool exists, 
        uint amount, 
        address originator, 
        address recipient, 
        bytes32 hashLock, 
        uint timeLock
    ) {
        BridgeTransfer storage bridgeTransfer = bridgeTransfers[_bridgeTransferId];
        return (
            bridgeTransfer.exists, 
            bridgeTransfer.amount,
            bridgeTransfer.originator,
            bridgeTransfer.recipient,
            bridgeTransfer.hashLock,
            bridgeTransfer.timeLock
        );
    }

    function getCompletedBridgeTransferDetail(bytes32 _bridgeTransferId) external view returns (
        bool exists, 
        uint amount, 
        address originator, 
        address recipient, 
        bytes32 hashLock, 
        uint timeLock
    ) {
        BridgeTransfer storage bridgeTransfer = completedBridgeTransfers[_bridgeTransferId];
        return (
            bridgeTransfer.exists, 
            bridgeTransfer.amount, 
            bridgeTransfer.originator, 
            bridgeTransfer.recipient, 
            bridgeTransfer.hashLock, 
            bridgeTransfer.timeLock
        );
    }
}

