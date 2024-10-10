// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import "forge-std/Script.sol";
import {AtomicBridgeInitiatorMOVE} from "../src/AtomicBridgeInitiatorMOVE.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";

contract AtomicBridgeInitiatorMOVEDeployer is Script {
    TransparentUpgradeableProxy public atomicBridgeProxy;
    string public atomicBridgeSignature = "initialize(address,address,uint256,uint256)";
    

    address public moveTokenAddress = address(0x1b37f0a65c1EeEFA9d4652317Af53F88F1e1a01C); 
    address public ownerAddress = address(0xYourOwnerAddress); 
    uint256 public timeLockDuration = 3600; 
    uint256 public initialPoolBalance = 1000 ether; 

    bytes32 public salt = 0xc000000000000000000000002774b8b4881d594b03ff8a93f4cad69407c90350;

    function run() external {
        uint256 signer = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(signer);

        // Deploy the TimelockController if needed
        _deployAtomicBridgeInitiator();

        // Add additional checks or initial state validations here
        require(IERC20(moveTokenAddress).balanceOf(address(atomicBridgeProxy)) == initialPoolBalance, "Initial pool balance is wrong");

        vm.stopBroadcast();
    }

    function _deployAtomicBridgeInitiator() internal {
        console.log("AtomicBridgeInitiatorMOVE: deploying");

        // Instantiate the implementation contract
        AtomicBridgeInitiatorMOVE atomicBridgeImplementation = new AtomicBridgeInitiatorMOVE();

        // Generate bytecode for proxy deployment
        bytes memory bytecode = abi.encodePacked(
            type(TransparentUpgradeableProxy).creationCode,
            abi.encode(
                address(atomicBridgeImplementation),
                ownerAddress,
                abi.encodeWithSignature(
                    atomicBridgeSignature,
                    moveTokenAddress,
                    ownerAddress,
                    timeLockDuration,
                    initialPoolBalance
                )
            )
        );

        // Deploy the proxy contract using CREATE2 with the specified salt
        atomicBridgeProxy = TransparentUpgradeableProxy(payable(vm.create2(salt, bytecode, 0)));

        console.log("AtomicBridgeInitiatorMOVE deployed at proxy address:", address(atomicBridgeProxy));
        console.log("Implementation address:", address(atomicBridgeImplementation));
    }
}
