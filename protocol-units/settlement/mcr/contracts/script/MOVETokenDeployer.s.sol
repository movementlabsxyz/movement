pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import {MOVEToken} from "../src/token/MOVEToken.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {SafeProxyFactory} from "@safe-smart-account/contracts/proxies/SafeProxyFactory.sol";
import {Safe} from "@safe-smart-account/contracts/Safe.sol";
import {TimelockController} from "@openzeppelin/contracts/governance/TimelockController.sol";
import {Vm} from "forge-std/Vm.sol";

function string2Address(bytes memory str) returns (address addr) {
    bytes32 data = keccak256(str);
    assembly {
        mstore(0, data)
        addr := mload(0)
    }
}

interface create {
    function deploy(bytes32 _salt, bytes memory _bytecode) external returns (address);
}

contract DeployMoveToken is Script {
    TransparentUpgradeableProxy public moveProxy;
    string public moveSignature = "initialize(address)";

    // COMMANDS
    // mainnet
    // forge script MOVETokenDeployer --fork-url https://eth.llamarpc.com --verify --etherscan-api-key ETHERSCAN_API_KEY
    // testnet
    // forge script MOVETokenDeployer --fork-url https://eth-sepolia.api.onfinality.io/public
    // Safes should be already deployed
    Safe public movementLabsSafe = Safe(payable(address(block.chainid == 1 ?  0x1 : 0x493516F6dB02c9b7f649E650c5de244646022Aa0)));
    Safe public movementFoundationSafe = Safe(payable(address( block.chainid == 1 ?  0x1 : 0x00db70A9e12537495C359581b7b3Bc3a69379A00)));
    TimelockController public timelock;
    address create3address = address(0x2Dfcc7415D89af828cbef005F0d072D8b3F23183);
    address moveAdmin;
    bytes32 public salt = 0xc000000000000000000000002774b8b4881d594b03ff8a93f4cad69407c90350;
    bytes32 public constant DEFAULT_ADMIN_ROLE = 0x00;

    function run() external {
        uint256 signer = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(signer);

        uint256 minDelay = 2 days;
        address[] memory proposers = new address[](1);
        address[] memory executors = new address[](1);

        proposers[0] = address(movementLabsSafe);
        executors[0] = address(movementFoundationSafe);

        timelock = new TimelockController(minDelay, proposers, executors, address(0x0));
        
        _deployMove();
        
        console.log("Safe balance: ", MOVEToken(address(moveProxy)).balanceOf(address(movementFoundationSafe)));
        console.log("Move Token decimals: ", MOVEToken(address(moveProxy)).decimals());
        console.log("Move Token supply: ", MOVEToken(address(moveProxy)).totalSupply());
        console.log("Timelock deployed at: ", address(timelock));
        console.log("foundation Safe deployed at: ", address(movementFoundationSafe));
        console.log("foundation multisig has admin role", MOVEToken(address(moveProxy)).hasRole(DEFAULT_ADMIN_ROLE, address(movementFoundationSafe)));
        console.log("timelock has admin role", MOVEToken(address(moveProxy)).hasRole(DEFAULT_ADMIN_ROLE, address(timelock)));
        vm.stopBroadcast();
    }

    function _deployMove() internal {
        console.log("MOVE: deploying");
        MOVEToken moveImplementation = new MOVEToken();
        // moveProxy = new TransparentUpgradeableProxy(
        //     address(moveImplementation), address(timelock), abi.encodeWithSignature(moveSignature, address(safe))
        // );
        bytes memory bytecode = abi.encodePacked(
            type(TransparentUpgradeableProxy).creationCode,
            abi.encode(address(moveImplementation), address(timelock), abi.encodeWithSignature(moveSignature, address(movementFoundationSafe)))
        );
        vm.recordLogs();
        moveProxy = TransparentUpgradeableProxy(payable(create(create3address).deploy(salt, bytecode)));
        Vm.Log[] memory logs = vm.getRecordedLogs();
        console.log("MOVE deployment records:");
        console.log("proxy", address(moveProxy));
        console.log("implementation", address(moveImplementation));
        // deployment.move = address(moveProxy);
        // deployment.moveAdmin = _storeAdminDeployment();
        
        moveAdmin = logs[logs.length - 2].emitter;
        console.log("MOVE admin", moveAdmin);
    }

    function _upgradeMove() internal {
        console.log("MOVE: upgrading");
        MOVEToken newMoveImplementation = new MOVEToken();
        timelock.schedule(
            address(moveAdmin),
            0,
            abi.encodeWithSignature(
                "upgradeAndCall(address,address,bytes)",
                address(moveProxy),
                address(newMoveImplementation),
                abi.encodeWithSignature("initialize(address)", address(movementFoundationSafe))
            ),
            bytes32(0),
            bytes32(0),
            block.timestamp + 1 days
        );
    }
}
