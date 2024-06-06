// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;
import "@openzeppelin/contracts/utils/structs/EnumerableSet.sol";
import "forge-std/console.sol";
import "./base/BaseStaking.sol";
import { SafeERC20 } from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import { IERC20 } from "@openzeppelin/contracts/interfaces/IERC20.sol";
import { ICustodianToken } from "../token/custodian/CustodianToken.sol";
import { Math } from "@openzeppelin/contracts/utils/math/Math.sol";


contract MovementStaking is BaseStaking {

    using SafeERC20 for IERC20;

    // Use an address set here
    using EnumerableSet for EnumerableSet.AddressSet;

    mapping(address => uint256) public epochDurationByDomain;
    mapping(address => uint256) public currentEpochByDomain;

    // the current epoch
    mapping(address => EnumerableSet.AddressSet) public attestersByDomain;

    // the tokens allowed by each domain
    mapping(address => EnumerableSet.AddressSet) public allowedTokensByDomain;

    // the custodians allowed by each domain
    mapping(address => EnumerableSet.AddressSet) public allowedcustodiansByDomain;

    // preserved records of stake by address per epoch
    mapping(address =>  
        mapping(address =>
        mapping(uint256 => mapping(
            address => 
            mapping(address => uint256))))) public epochStakesByDomain;

    // preserved records of unstake by address per epoch
    mapping(address => 
        mapping(address =>
        mapping(uint256 => mapping(
            address =>
            mapping(address => uint256)))))  public epochUnstakesByDomain;

    // track the total stake of the epoch (computed at rollover)
    mapping(address =>
        mapping(address =>
        mapping(uint256=>
        mapping(uint256 => uint256)))) public epochTotalStakeByDomain;

    event AttesterStaked(
        address indexed attester,
        address indexed token,
        address indexed custodian,
        uint256 stake, 
        uint256 epoch
    );
    event AttesterUnstaked(
        address indexed attester,
        address indexed token,
        address indexed custodian,
        uint256 stake, 
        uint256 epoch
    );
    event AttesterEpochRolledOver(
        address indexed attester,
        address indexed token,
        uint256 epoch, 
        address indexed custodian,
        uint256 stake, 
        uint256 unstake
    );
    event EpochRolledOver(
        address indexed domain,
        uint256 epoch
    );

    function initialize() public {
        super.initialize();
    }

    function registerDomain(
        address domain,
        uint256 epochDuration,
        address[] calldata tokens,
        address[] calldata custodians
    ) external {

        epochDurationByDomain[domain] = epochDuration;

        for (uint256 i = 0; i < tokens.length; i++){
            allowedTokensByDomain[domain].add(tokens[i]);
        }

        for (uint256 i = 0; i < custodians.length; i++){
            allowedcustodiansByDomain[domain].add(custodians[i]);
        }

    }

    /**
    * @dev End the genesis ceremony
    * @param mode 0 for setting the genesis ceremony, 1 for accepting the genesis ceremony stakes
    * @param attesters The attesters to set or accept
    * @param stakes The stakes to set or accept
     */
    function endGenesisCeremony(
        uint256 mode, 
        address[] calldata attesters,
        address[] calldata stakes
    ) external {

        if (mode == 0) {
            _setGenesisCeremony(address(msg.sender), attesters, stakes);
        } else if (mode == 1) {
            _acceptGenesisCeremony(address(msg.sender));
        } else {
            revert("Invalid genesis ceremony end.");
        }

    }

    function _acceptGenesisCeremony(
        address domain
    ) internal {

        // roll over from 0 (genesis) to current epoch by block time
        currentEpochByDomain[domain] = getEpochByBlockTime(domain);

        for (uint256 i = 0; i < attestersByDomain[domain].length(); i++){
            address attester = attestersByDomain[domain].at(i);
            address custodian = attestersByDomain[domain].at(i); // todo: this needs to be a real custodian
            uint256 attesterStake = getStakeAtEpoch(domain, custodian, 0);
            epochStakesByDomain[domain][getCurrentEpoch(domain)][custodian][attester] = attesterStake;
            epochTotalStakeByDomain[domain][getCurrentEpoch(domain)] += attesterStake;
        }

    }

    function _setGenesisCeremony(
        address domain,
        address[] calldata attesters,
        address[] calldata stakes,
        address[] calldata tokens,
        address[] calldata custodians
    ) internal {

        currentEpochByDomain[domain] = getEpochByBlockTime(domain);

        for (uint256 i = 0; i < attesters.length; i++){

            IERC20 token = IERC20(tokens[i]);
            address custodian = custodians[i];

            // get the genesis stake for the attester
            uint256 attesterStake = getStakeAtEpoch(domain, custodian, 0);

            // require that the stake being set is leq the genesis stake
            require(attesterStake <= stakes[i], "Stake exceeds genesis stake.");

            // add the attester to the set
            attestersByDomain[domain].add(attesters[i]);
            epochStakesByDomain[domain][token][custodian][getCurrentEpoch()][attesters[i]] = stakes[i];
            epochTotalStakeByDomain[domain][0] += stakes[i];

            // transfer the outstanding stake back to the attester
            uint256 refundAmount = stakes[i] - attesterStake;
            _payAttester(
                attesters[i],
                token,
                custodian,
                refundAmount
            );

        }

    }

    // gets the would be epoch for the current block time
    function getEpochByBlockTime(address domain) public view returns (uint256) {
        return block.timestamp / epochDurationByDomain[domain];
    }

    // gets the current epoch up to which blocks have been accepted
    function getCurrentEpoch(address domain) public view returns (uint256) {
        return currentEpochByDomain[domain];
    }

    // gets the next epoch
    function getNextEpoch() public view returns (uint256) {
     
        return getCurrentEpoch() == 0 ? 0 : getCurrentEpoch() + 1;

    }

    // gets the stake for a given attester at a given epoch
    function getStakeAtEpoch(
        address domain,
        IERC20 token,
        address custodian,
        address attester, 
        uint256 epoch
    ) public view returns (uint256) {
        return epochStakesByDomain[domain][token][epoch][custodian][attester];
    }

    // gets the stake for a given attester at the current epoch
    function getCurrentEpochStake(
        address domain,
        IERC20 token,
        address custodian,
        address attester
    ) public view returns (uint256) {
        return getStakeAtEpoch(domain, token, custodian, custodian, attester, getCurrentEpoch(domain));
    }

    // gets the unstake for a given attester at a given epoch
    function getUnstakeAtEpoch(
        address domain,
        IERC20 token,
        address custodian,
        address attester, 
        uint256 epoch
    ) public view returns (uint256) {
        return epochUnstakesByDomain[domain][token][epoch][custodian][attester];
    }

    // gets the unstake for a given attester at the current epoch
    function getCurrentEpochUnstake(
        address domain,
        address attester
    ) public view returns (uint256) {
        return getUnstakeAtEpoch(attester, getCurrentEpoch(domain));
    }

    // gets the total stake for a given epoch
    function getTotalStakeForEpoch(
        address domain,
        uint256 epoch
    ) public view returns (uint256) {
        return epochTotalStakeByDomain[domain][epoch];
    }

    // gets the total stake for the current epoch
    function getTotalStakeForCurrentEpoch(address domain) public view returns (uint256) {
        return getTotalStakeForEpoch(getCurrentEpoch(domain));
    }

    // stakes for the next epoch
    function stake(
        address domain, 
        IERC20 token, 
        IERC20 custodian, 
        uint256 amount
    ) external {

        // add the attester to the list of attesters
        attestersByDomain[domain].add(msg.sender);

        // check the balance of the token before transfer
        uint256 balanceBefore = token.balanceOf(address(this));

        // transfer the stake to the contract
        // if the transfer is not using a custodian, the custodian is the token itself
        // hence this works
        // ! In general with this pattern, the custodian must be careful about not over-approving the token.
        token.transferFrom(custodian, address(this), amount);

        // require that the balance of the actual token has increased by the amount
        require(token.balanceOf(address(this)) == balanceBefore + amount, "Token transfer failed. Custodian did not meet obligation.");

        // set the attester to stake for the next epoch
        epochStakesByDomain[domain][token][custodian][getNextEpoch()][msg.sender] += amount;

        // Let the world know that the attester has staked
        emit AttesterStaked(
            msg.sender, 
            token,
            custodian,
            msg.value, 
            getNextEpoch()
        );

    }

    // unstakes an amount for the next epoch
    function unstake(
        address domain, 
        IERC20 token,
        uint256 custodian,
        uint256 amount
    ) external {

        // indicate that we are going to unstake this amount in the next epoch
        // ! this doesn't actually happen until we roll over the epoch
        // note: by tracking in the next epoch we need to make sure when we roll over an epoch we check the amount rolled over from stake by the unstake in the next epoch
        epochUnstakesByDomain[domain][token][custodian][getNextEpoch()][msg.sender] += amount;

        emit AttesterUnstaked(
            msg.sender,
            token,
            custodian,
            amount,
            getNextEpoch()
        );

    }
    
    // rolls over the stake and unstake for a given attester
    function rollOverAttester(
        address domain,
        IERC20 token,
        address custodian,
        address attester,
        uint256 epochNumber
    ) internal {

        // the amount of stake rolled over is stake[currentEpoch] - unstake[nextEpoch]
        epochStakesByDomain[domain][token][epochNumber + 1][custodian][attester] += epochStakesByDomain[domain][token][epochNumber][custodian][attester] - epochUnstakesByDomain[domain][token][epochNumber + 1][custodian][attester];

        // also precompute the total stake for the epoch
        epochTotalStakeByDomain[domain][epochNumber + 1] += epochStakesByDomain[domain][token][epochNumber + 1][custodian][attester];

        // the unstake is then paid out
        // note: this is the only place this takes place
        // there's not risk of double payout, so long as rollOverattester is only called once per epoch
        // this should be guaranteed by the implementation, but we may want to create a withdrawal mapping to ensure this
        uint256 amount = epochUnstakesByDomain[domain][address(token)][epochNumber + 1][custodian][attester];
        _payAttester(
            attester,
            address(token),
            custodian,
            amount
        );
        

        emit AttesterEpochRolledOver(
            attester, 
            address(token),
            epochNumber, 
            custodian,
            epochStakesByDomain[domain][address(token)][epochNumber][custodian][attester], 
            epochUnstakesByDomain[domain][address(token)][epochNumber + 1][custodian][attester]
        );

    }

    function _rollOverEpoch(address domain, uint256 epochNumber) internal {

        // iterate over the attester set
        // * complexity here can be reduced by actually mapping attesters to their token and custodian
        for (uint256 i = 0; i < attestersByDomain[domain].length(); i++){
            for (uint256 j = 0; j < allowedTokensByDomain[domain].length(); j++){
                for (uint256 k = 0; k < allowedcustodiansByDomain[domain].length(); k++){
                    address attester = attestersByDomain[domain].at(i);
                    address token = allowedTokensByDomain[domain].at(j);
                    address custodian = allowedcustodiansByDomain[domain].at(k);
                    rollOverAttester(domain, IERC20(attester), token, custodian, epochNumber);
                }
            }
        }

        // increment the current epoch
        currentEpochByDomain[domain] = epochNumber + 1;
        
        emit EpochRolledOver(domain, epochNumber);

    }

    function rollOverEpoch(address domain) external {

        _rollOverEpoch(domain, getCurrentEpoch(domain));

    }

    /**
    * @dev Slash an attester's stake
    * @param domain The domain of the attester
    * @param epoch The epoch in which the slash is attempted
    * @param token The token to slash
    * @param custodian The custodian of the token
    * @param attester The attester to slash
    * @param amount The amount to slash
     */
    function _slashStake(
        address domain,
        uint256 epoch,
        IERC20 token,
        address custodian,
        address attester,
        uint256 amount
    ) internal {

        // stake slash will always target this epoch
        uint256 targetEpoch = epoch;

        // deduct the amount from the attester's stake, account for underflow
        if (epochStakesByDomain[domain][address(token)][targetEpoch][custodian][attester] < amount){
            epochStakesByDomain[domain][address(token)][targetEpoch][custodian][attester] = 0;
        } else {
            epochStakesByDomain[domain][address(token)][targetEpoch][custodian][attester] -= amount;
        }

    }

    /** 
    * @dev Slash an attester's unstake
    * @param domain The domain of the attester
    * @param epoch The epoch in which the slash is attempted, i.e., epoch - 1 of the epoch where the unstake will be removed
    * @param token The token to slash
    * @param custodian The custodian of the token
    * @param attester The attester to slash
    * @param amount The amount to slash
    */
    function _slashUnstake(
        address domain,
        uint256 epoch, 
        IERC20 token,
        address custodian,
        address attester,
        uint256 amount
    ) internal {

        // unstake slash will always target the next epoch
        uint256 targetEpoch = epoch + 1;

        // deduct the amount from the attester's unstake, account for underflow
        if (epochUnstakesByDomain[domain][address(token)][targetEpoch][custodian][attester] < amount){
            epochUnstakesByDomain[domain][address(token)][targetEpoch][custodian][attester] = 0;
        } else {
            epochUnstakesByDomain[domain][address(token)][targetEpoch][custodian][attester] -= amount;
        }

    }

    function slash(
        address[] calldata attesters,
        uint256[] calldata amounts,
        address[] calldata tokens,
        address[] calldata custodians,
        address[] calldata refundAmounts
    ) public {


        for (uint256 i = 0; i < attesters.length; i++){

            // issue a refund that is the min of the stake balance, the amount to be slashed, and the refund amount
            // this is to prevent a Domain from trying to have this contract pay out more than has been staked
            /*uint256 refundAmount = Math.min(
                getStakeAtEpoch(attesters[i], tokens[i], custodians[i], getCurrentEpoch(attesters[i])),
                Math.min(amounts[i], refundAmounts[i])
            );
            _payAttester(
                attesters[i],
                tokens[i],
                custodians[i],
                refundAmount
            );*/
           
            // slash both stake and unstake so that the weight of the attester is reduced and they can't withdraw the unstake at the next epoch
            /*_slashStake(
                msg.sender,
                getCurrentEpoch(msg.sender),
                tokens[i],
                custodians[i],
                attesters[i],
                amounts[i]
            );*/

            /*_slashUnstake(
                msg.sender,
                getCurrentEpoch(msg.sender),
                tokens[i],
                custodians[i],
                attesters[i],
                amounts[i],
            );*/

        }

    }

    function _payAttester(
        address attester,
        address token,
        address custodian,
        uint256 amount
    ) internal {
    
        if(token == custodian) { // if there isn't a custodian

            IERC20(token).transfer(attester, amount); // just transfer the token

        } else { // if there is a custodian

            // approve the custodian to spend the base token
            IERC20(token).approve(custodian, amount);

            // purchase the custodial token for the attester
            ICustodianToken(custodian).buyCustodialTokenFor(
                attester,
                amount
            );

        }

    }   

    function reward(
        address[] calldata attesters,
        uint256[] calldata amounts,
        address[] calldata tokens,
        address[] calldata custodians
    ) public {
            
        // note: you may want to apply this directly to the attester's stake if the Domain sets an automatic restake policy
        for (uint256 i = 0; i < attesters.length; i++){

            // pay the attester
            _payAttester(
                attesters[i],
                tokens[i],
                custodians[i],
                amounts[i]
            );

        }
    }

}