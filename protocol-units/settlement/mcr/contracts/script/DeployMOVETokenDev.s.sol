pragma solidity ^0.8.19;

import "forge-std/Script.sol";
import "../src/token/MOVETokenDev.sol";
import {IMintableToken, MintableToken} from "../src/token/base/MintableToken.sol";
import {ProxyAdmin} from "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {IERC20} from "@openzeppelin/contracts/interfaces/IERC20.sol";
import {ERC1967Proxy} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import {Helper} from "./helpers/Helper.sol";

contract DeployMOVETokenDev is Helper {
    address public manager = 0x5A368EDEbF574162B84f8ECFE48e9De4f520E087;
    uint256 public signer = vm.envUint("TEST_1");
    function run() external {
        vm.startBroadcast(signer);

        MOVETokenDev moveTokenImplementation = new MOVETokenDev();
        TransparentUpgradeableProxy moveTokenProxy = new TransparentUpgradeableProxy(
            address(moveTokenImplementation),
            manager,
            abi.encodeWithSignature("initialize(address)", manager)
        ); 

        console.log("Move Token Proxy: %s", address(moveTokenProxy));

        vm.stopBroadcast();
    }
}
