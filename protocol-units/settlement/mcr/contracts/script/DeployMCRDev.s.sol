pragma solidity ^0.8.19;

import "forge-std/Script.sol";
import "../src/token/MOVEToken.sol";
import "../src/staking/MovementStaking.sol";
import "../src/settlement/MCR.sol";
import {IMintableToken, MintableToken} from "../src/token/base/MintableToken.sol";
import {ProxyAdmin} from "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {IERC20} from "@openzeppelin/contracts/interfaces/IERC20.sol";

contract DeployMCRDev is Script {

    string public moveTokenSignature = "initialize(string,string)";
    string public stakingSignature = "initialize(address)";
    string public mcrSignature = "initialize(address,uint256,uint256,uint256,address[])";

    function run() external {
        vm.startBroadcast();

        MintableToken moveTokenImplementation = new MintableToken();
        MovementStaking stakingImplementation = new MovementStaking();
        MCR mcrImplementation = new MCR();

        ProxyAdmin admin = new ProxyAdmin(vm.addr(1));

        TransparentUpgradeableProxy moveTokenProxy = new TransparentUpgradeableProxy(
            address(moveTokenImplementation),
            address(admin),
            abi.encodeWithSignature(moveTokenSignature, "Move Token", "MOVE")
        );

        TransparentUpgradeableProxy stakingProxy = new TransparentUpgradeableProxy(
            address(stakingImplementation),
            address(admin),
            abi.encodeWithSignature(
                stakingSignature, 
                IMintableToken(address(moveTokenProxy))
            )
        );

        address[] memory custodians = new address[](1);
        custodians[0] = address(moveTokenProxy);
        TransparentUpgradeableProxy mcrProxy = new TransparentUpgradeableProxy(
            address(mcrImplementation),
            address(admin),
            abi.encodeWithSignature(
                mcrSignature,
                address(stakingProxy),
                5,
                100 ether,
                100 ether,
                custodians
            )
        );

        vm.stopBroadcast();

    }
}
