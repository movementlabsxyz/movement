pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import {MOVEToken} from "../src/token/MOVEToken.sol";
import {ProxyAdmin} from "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {SafeProxyFactory} from "@safe-smart-account/contracts/proxies/SafeProxyFactory.sol";
import {SafeProxy} from "@safe-smart-account/contracts/proxies/SafeProxy.sol";
import {Safe} from "@safe-smart-account/contracts/Safe.sol";
import {CreateCall} from "@safe-smart-account/contracts/libraries/CreateCall.sol";
import {TimelockController} from "@openzeppelin/contracts/governance/TimelockController.sol";
import {Enum} from "@safe-smart-account/contracts/common/Enum.sol";

contract MOVETokenDeployerMultisig is Script {
    TransparentUpgradeableProxy public moveProxy;
    ProxyAdmin public admin;
    string public moveSignature = "initialize(address)";
    string public safeSetupSignature = "setup(address[],uint256,address,bytes,address,address,uint256,address)";
    SafeProxyFactory public safeProxyFactory;
    address public zero = address(0x0);
    address public movementFoundationMockMultisig = address(0x00db70A9e12537495C359581b7b3Bc3a69379A00);
    address public safeSingleton = 0x29fcB43b46531BcA003ddC8FCB67FFE91900C762;
    CreateCall public createCall = CreateCall(0x7cbB62EaA69F79e6873cD1ecB2392971036cFAa4);
    address payable public safeAddress;
    Safe public safe;
    uint256 public threshold = 2;
    TimelockController public timelock;

    function run() external {
        uint256 signer = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(signer);
        MOVEToken moveImplementation = new MOVEToken();

        // forge script DeployMoveTokenMultisig --fork-url https://eth-sepolia.api.onfinality.io/public

        address[] memory signers = new address[](5);

        signers[0] = vm.addr(signer);
        signers[1] = vm.addr(1);
        signers[2] = vm.addr(2);
        signers[3] = vm.addr(3);
        signers[4] = vm.addr(4);

        // DEPLOYMENT USING SAFE PROXY FACTORY
        // Safe Sepolia Proxy Factory
        safeProxyFactory = SafeProxyFactory(0x4e1DCf7AD4e460CfD30791CCC4F9c8a4f820ec67);
        safeAddress = payable(
            address(
                safeProxyFactory.createProxyWithNonce(
                    safeSingleton,
                    // Fallback Manager address
                    abi.encodeWithSignature(
                        safeSetupSignature,
                        signers,
                        threshold,
                        zero,
                        "0x",
                        // Fallback Handler address
                        0xfd0732Dc9E303f09fCEf3a7388Ad10A83459Ec99,
                        zero,
                        0,
                        payable(zero)
                    ),
                    0
                )
            )
        );
        safe = Safe(safeAddress);
        // DEPLOYMENT USING SAFE PROXY FACTORY

        // DEPLOYMENT USING SAFE PROXY
        // safe = Safe(payable(address(new SafeProxy(safeSingleton))));
        // safe.setup(signers, 3, zero, "0x", 0xfd0732Dc9E303f09fCEf3a7388Ad10A83459Ec99, zero, 0, payable(zero));
        // DEPLOYMENT USING SAFE PROXY

        uint256 minDelay = 2 days;
        address[] memory proposers = new address[](5);
        address[] memory executors = new address[](1);

        // these are unnamed addresses because we need the private key for signatures to simulate Safe multisig transactions
        // consider these to be the private keys of the signers
        proposers[0] = vm.addr(5);
        proposers[1] = vm.addr(6);
        proposers[2] = vm.addr(7);
        proposers[3] = vm.addr(8);
        proposers[4] = vm.addr(9);

        // Movement Foundation Mock Safe
        executors[0] = address(safe);

        moveImplementation = new MOVEToken();
        bytes memory proxyConstructorArgs = abi.encode(
            address(moveImplementation), address(timelock), abi.encodeWithSignature(moveSignature, address(safe))
        );
        bytes memory proxyDeploymentData =
            abi.encodePacked(type(TransparentUpgradeableProxy).creationCode, proxyConstructorArgs);

        bytes memory createCallData =
            abi.encodeWithSignature("performCreate2(uint256,bytes,bytes32)", 0, proxyDeploymentData, "");
        bytes32 digest = keccak256(abi.encodePacked("\x19Ethereum Signed Message:\n32", createCallData));

        bytes memory signatures = generateSignatures(signer, digest);

        safe.execTransaction(
            address(createCall), 0, createCallData, Enum.Operation.Call, 0, 0, 0, zero, payable(zero), signatures
        );

        console.log("Timelock deployed at: ", address(timelock));
        console.log("Safe deployed at: ", address(safe));
        console.log("Move Token deployed at: ", address(moveProxy));
        console.log("implementation deployed at: ", address(moveImplementation));
        vm.stopBroadcast();
    }

    function generateSignatures(uint256 privKey, bytes32 digest) internal returns (bytes memory signatures) {
    (uint8 v1, bytes32 r1, bytes32 s1) = vm.sign(privKey, digest);
    (uint8 v2, bytes32 r2, bytes32 s2) = vm.sign(2, digest);
    (uint8 v3, bytes32 r3, bytes32 s3) = vm.sign(3, digest);

    signatures = abi.encodePacked(r1, s1, v1, r2, s2, v2);
}
}


