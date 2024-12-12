pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import "v2-core/interfaces/IUniswapV2Factory.sol";
import "v2-core/interfaces/IUniswapV2Pair.sol";
import {IERC20} from "v2-core/interfaces/IERC20.sol";
import "v2-periphery/interfaces/IUniswapV2Router02.sol";
import "forge-std/console.sol";


contract PoolDeployer is Script {
    uint256 privateKey = vm.envUint("PRIVATE_KEY");
    IUniswapV2Router02 public router = IUniswapV2Router02(0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D);
    IUniswapV2Factory public factory = IUniswapV2Factory(0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f);
    IUniswapV2Pair public pair;

    address moveAddress = 0x3073f7aAA4DB83f95e9FFf17424F71D4751a3073;
    address wethAddress = 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2;

    uint256 public moveDecimals = 8;
    uint256 public ethDecimals = 18;
    uint256 public moveTotalSupply = 10* 10**9 * 10**moveDecimals;
    uint256 public moveAndEthDepositValue = 600;

    uint256 public targetValuation = 6_000_000_000;
    uint256 public currentEthPrice = 3600;

    address public deployer = 0xB2105464215716e1445367BEA5668F581eF7d063;

    function run() external {
        vm.startBroadcast(privateKey);
        address recipient = 0x706dd4707E2e84523463cCF8Ea8c49f07aA71601;

        uint256 ethAmount = (moveAndEthDepositValue * 1e18) / currentEthPrice;
        uint256 movePriceInEth = targetValuation * 1e18 / moveTotalSupply;
        uint256 moveAmount = (moveAndEthDepositValue * 1e18) / movePriceInEth;
        console.log("moveAmount: ", moveAmount);
        console.log("ethAmount: ", ethAmount);

        vm.assertEq(vm.addr(privateKey), deployer);

        // IERC20(moveAddress).approve(address(router), moveAmount);
        // pair = IUniswapV2Pair(factory.createPair(moveAddress, wethAddress));
        // console.log("pair: ", address(pair));
        router.addLiquidityETH{value: ethAmount}(moveAddress, moveAmount, moveAmount * 9 / 10, ethAmount * 9 / 10, recipient, block.timestamp + 1000);
        vm.stopBroadcast();
    }
}