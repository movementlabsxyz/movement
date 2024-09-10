pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import {MOVEToken} from "../src/token/MOVEToken.sol";
import {ProxyAdmin} from "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {SafeProxyFactory} from "@safe-smart-account/contracts/proxies/SafeProxyFactory.sol";
import {Safe} from "@safe-smart-account/contracts/Safe.sol";
import {CreateCall} from "@safe-smart-account/contracts/libraries/CreateCall.sol";
import {TimelockController} from "@openzeppelin/contracts/governance/TimelockController.sol";
import {MessageHashUtils} from "@openzeppelin/contracts/utils/cryptography/MessageHashUtils.sol";

function string2Address(bytes memory str) returns (address addr) {
    bytes32 data = keccak256(str);
    assembly {
        mstore(0, data)
        addr := mload(0)
    }
}

contract DeployMoveTokenMultisig is Script {
    TransparentUpgradeableProxy public moveProxy;
    ProxyAdmin public admin;
    string public moveSignature = "initialize(address)";
    string public safeSetupSignature = "setup(address[],uint256,address,bytes,address,address,uint256,address)";
    CompatibilityFallbackHandler public compatibilityFallbackHandler;
    SafeProxyFactory public safeProxyFactory;
    address public safeSingleton = 0x29fcB43b46531BcA003ddC8FCB67FFE91900C762;
    CreateCall public createCall = 0x7cbB62EaA69F79e6873cD1ecB2392971036cFAa4;
    address payable public safeAddress;
    Safe public safe;
    TimelockController public timelock;

    function run() external {
        uint256 signer = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(signer);
        MOVEToken moveImplementation = new MOVEToken();

        // forge script DeployMoveToken --fork-url https://eth-sepolia.api.onfinality.io/public

        safeProxyFactory = SafeProxyFactory(0x4e1DCf7AD4e460CfD30791CCC4F9c8a4f820ec67);

        address[] memory signers = new address[](5);

        signers[0] = vm.addr(signer);
        signers[1] = string2Address("Bob");
        signers[2] = string2Address("Charlie");
        signers[3] = string2Address("David");
        signers[4] = string2Address("Eve");

        safeAddress = payable(
            address(
                safeProxyFactory.createProxyWithNonce(
                    safeSingleton,
                    abi.encodeWithSignature(
                        safeSetupSignature,
                        signers,
                        3,
                        address(compatibilityFallbackHandler),
                        "0x",
                        address(0x0),
                        address(0x0),
                        0,
                        payable(address(0x0))
                    ),
                    0
                )
            )
        );

        safe = Safe(safeAddress);

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

        // build the deployment data
        bytes data =
            abi.encodePacked(type(MOVEToken).creationCode, address(safe), 0, data, 0, 0, 0, address(0), address(0));
        // generate 3 signatures for the safe transaction
        // ecsda signatures
        bytes32[] memory signatures = new bytes32[](3);

        // NOT VALID, SIGNATURE HAS TO BE DONE SOME OTHER WAY
        vm.broadcast(proposers[0]);
        signatures[0] = MessageHashUtils.toEthSignedMessageHash(data);
        vm.broadcast(proposers[1]);
        signatures[1] = MessageHashUtils.toEthSignedMessageHash(data);
        vm.broadcast(proposers[2]);
        signatures[2] = MessageHashUtils.toEthSignedMessageHash(data);

        safe.execTransaction(address(createCall), 0, data, operation, 0, 0, 0, address(0), address(0), signatures);
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
