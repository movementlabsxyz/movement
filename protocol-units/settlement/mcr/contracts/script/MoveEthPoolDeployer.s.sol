pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import "v2-core/interfaces/IUniswapV2Factory.sol";
import "v2-core/interfaces/IUniswapV2Pair.sol";
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
    uint256 public moveAndEthDepositValue = 250_000;

    uint256 public targetValuation = 500_000_000;
    uint256 public currentEthPrice = 4000;

    function run() external {
        vm.startBroadcast(privateKey);

        uint256 ethAmount = (moveAndEthDepositValue * 1e18) / currentEthPrice;
        uint256 movePriceInEth = targetValuation * 1e18 / moveTotalSupply;
        uint256 moveAmount = (moveAndEthDepositValue * 1e18) / movePriceInEth;
        console.log("moveAmount: ", moveAmount);
        console.log("ethAmount: ", ethAmount);
        pair = IUniswapV2Pair(factory.createPair(moveAddress, wethAddress));
        router.addLiquidity(moveAddress, wethAddress, moveAmount, ethAmount, 0, 0, address(this), block.timestamp + 1000);
    }
}