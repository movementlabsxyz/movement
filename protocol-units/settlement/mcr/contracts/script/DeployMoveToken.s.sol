pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import {MOVEToken} from "../src/token/MOVEToken.sol";
import {ProxyAdmin} from "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {SafeProxyFactory} from "@safe-smart-account/contracts/proxies/SafeProxyFactory.sol";
import {SafeProxy} from "@safe-smart-account/contracts/proxies/SafeProxy.sol";
import {Safe} from "@safe-smart-account/contracts/Safe.sol";
import {CompatibilityFallbackHandler} from "@safe-smart-account/contracts/handler/CompatibilityFallbackHandler.sol";
import {TimelockController} from "@openzeppelin/contracts/governance/TimelockController.sol";

function string2Address(bytes memory str) returns (address addr) {
    bytes32 data = keccak256(str);
    assembly {
        mstore(0, data)
        addr := mload(0)
    }
}

contract DeployMoveToken is Script {
    TransparentUpgradeableProxy public moveProxy;
    ProxyAdmin public admin;
    string public moveSignature = "initialize(address)";
    string public safeSetupSignature = "setup(address[],uint256,address,bytes,address,address,uint256,address)";
    CompatibilityFallbackHandler public compatibilityFallbackHandler;
    SafeProxyFactory public safeProxyFactory;
    Safe public safeSingleton;
    Safe public safe;
    TimelockController public timelock;

    function run() external {
        uint256 signer = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(signer);
        MOVEToken moveImplementation = new MOVEToken();

        // forge script DeployMoveToken --fork-url https://eth-sepolia.api.onfinality.io/public
        safe = Safe(payable(address(0x00db70A9e12537495C359581b7b3Bc3a69379A00)));

        // safeProxyFactory = new SafeProxyFactory();
        // safeSingleton = new Safe();
        // compatibilityFallbackHandler = new CompatibilityFallbackHandler();

        // address[] memory signers = new address[](5);

        // signers[0] = vm.addr(signer);
        // signers[1] = string2Address("Bob");
        // signers[2] = string2Address("Charlie");
        // signers[3] = string2Address("David");
        // signers[4] = string2Address("Eve");

        // safe = Safe(
        //     payable(
        //         address(
        //             safeProxyFactory.createProxyWithNonce(
        //                 address(safeSingleton),
        //                 abi.encodeWithSignature(
        //                     safeSetupSignature,
        //                     signers,
        //                     3,
        //                     address(compatibilityFallbackHandler),
        //                     "0x",
        //                     address(0x0),
        //                     address(0x0),
        //                     0,
        //                     payable(address(0x0))
        //                 ),
        //                 0
        //             )
        //         )
        //     )
        // );

        uint256 minDelay = 1 days;
        address[] memory proposers = new address[](5);
        address[] memory executors = new address[](1);

        proposers[0] = string2Address("Andy");
        proposers[1] = string2Address("Bob");
        proposers[2] = string2Address("Charlie");
        proposers[3] = string2Address("David");
        proposers[4] = string2Address("Eve");

        executors[0] = address(safe);

        timelock = new TimelockController(minDelay, proposers, executors, address(0x0));

        moveProxy = new TransparentUpgradeableProxy(
            address(moveImplementation), address(timelock), abi.encodeWithSignature(moveSignature, address(safe))
        );

        console.log("Timelock deployed at: ", address(timelock));
        console.log("Safe deployed at: ", address(safe));
        console.log("Move Token deployed at: ", address(moveProxy));
        console.log("implementation deployed at: ", address(moveImplementation));
        vm.stopBroadcast();
    }
}
