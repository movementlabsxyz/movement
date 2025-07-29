// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;
import "forge-std/console.sol";
import {BaseStaking} from "./base/BaseStaking.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {IERC20} from "@openzeppelin/contracts/interfaces/IERC20.sol";
import {ICustodianToken} from "../token/custodian/CustodianToken.sol";
import {Math} from "@openzeppelin/contracts/utils/math/Math.sol";
import {MovementStakingStorage, EnumerableSet} from "./MovementStakingStorage.sol";
import {IMovementStaking} from "./interfaces/IMovementStaking.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";

contract MovementStaking is
    MovementStakingStorage,
    IMovementStaking,
    BaseStaking,
    ReentrancyGuard
{
    using EnumerableSet for EnumerableSet.AddressSet;

    function initialize(IERC20 _token) public initializer {
        __BaseStaking_init_unchained();
        token = _token;
    }

    function registerDomain(
        uint256 epochDuration,
        address[] calldata custodians
    ) external nonReentrant {
        address domain = msg.sender;
        epochDurationByDomain[domain] = epochDuration;

        for (uint256 i = 0; i < custodians.length; i++) {
            custodiansByDomain[domain].add(custodians[i]);
        }
    }

    function getCustodiansByDomain(
        address domain
    ) public view returns (address[] memory) {
        // todo: we probably want to figure out a better API which still allows domains to interpret custodians as they see fit
        address[] memory custodians = new address[](
            custodiansByDomain[domain].length()
        );
        for (uint256 i = 0; i < custodiansByDomain[domain].length(); i++) {
            custodians[i] = custodiansByDomain[domain].at(i);
        }
        return custodians;
    }

    function getAttestersByDomain(
        address domain
    ) public view returns (address[] memory) {
        address[] memory attesters = new address[](
            attestersByDomain[domain].length()
        );
        for (uint256 i = 0; i < attestersByDomain[domain].length(); i++) {
            attesters[i] = attestersByDomain[domain].at(i);
        }
        return attesters;
    }

    function acceptGenesisCeremony() public nonReentrant {
        address domain = msg.sender;
        if (domainGenesisAccepted[domain]) revert GenesisAlreadyAccepted();
        domainGenesisAccepted[domain] = true;
        // roll over from 0 (genesis) to current epoch by block time
        currentEpochByDomain[domain] = getEpochByBlockTime(domain);

        for (uint256 i = 0; i < attestersByDomain[domain].length(); i++) {
            address attester = attestersByDomain[domain].at(i);

            for (uint256 j = 0; j < custodiansByDomain[domain].length(); j++) {
                address custodian = custodiansByDomain[domain].at(j);

                // get the genesis stake for the attester
                uint256 attesterStake = getStakeAtEpoch(
                    domain,
                    0,
                    custodian,
                    attester
                );

                // roll over the genesis stake to the current epoch
                _addStake(
                    domain,
                    getCurrentEpoch(domain),
                    custodian,
                    attester,
                    attesterStake
                );
            }
        }
    }

    function _addStake(
        address domain,
        uint256 epoch,
        address custodian,
        address attester,
        uint256 amount
    ) internal {
        epochStakesByDomain[domain][epoch][custodian][attester] += amount;
        epochTotalStakeByDomain[domain][epoch][custodian] += amount;
    }

    function _removeStake(
        address domain,
        uint256 epoch,
        address custodian,
        address attester,
        uint256 amount
    ) internal {
        epochStakesByDomain[domain][epoch][custodian][attester] -= amount;
        epochTotalStakeByDomain[domain][epoch][custodian] -= amount;
    }

    function _addUnstake(
        address domain,
        uint256 epoch,
        address custodian,
        address attester,
        uint256 amount
    ) internal {
        epochUnstakesByDomain[domain][epoch][custodian][attester] += amount;
    }

    function _removeUnstake(
        address domain,
        uint256 epoch,
        address custodian,
        address attester,
        uint256 amount
    ) internal {
        epochUnstakesByDomain[domain][epoch][custodian][attester] -= amount;
    }

    function _setUnstake(
        address domain,
        uint256 epoch,
        address custodian,
        address attester,
        uint256 amount
    ) internal {
        epochUnstakesByDomain[domain][epoch][custodian][attester] = amount;
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
    function getNextEpoch(address domain) public view returns (uint256) {
        return getCurrentEpoch(domain) == 0 ? 0 : getCurrentEpoch(domain) + 1;
    }

    function getNextEpochByBlockTime(
        address domain
    ) public view returns (uint256) {
        return
            getCurrentEpoch(domain) == 0 ? 0 : getEpochByBlockTime(domain) + 1;
    }

    // gets the stake for a given attester at a given epoch
    function getStakeAtEpoch(
        address domain,
        uint256 epoch,
        address custodian,
        address attester
    ) public view returns (uint256) {
        return epochStakesByDomain[domain][epoch][custodian][attester];
    }

    // gets the stake for a given attester at the current epoch
    function getCurrentEpochStake(
        address domain,
        address custodian,
        address attester
    ) public view returns (uint256) {
        return
            getStakeAtEpoch(
                domain,
                getCurrentEpoch(domain),
                custodian,
                attester
            );
    }

    // gets the unstake for a given attester at a given epoch
    function getUnstakeAtEpoch(
        address domain,
        uint256 epoch,
        address custodian,
        address attester
    ) public view returns (uint256) {
        return epochUnstakesByDomain[domain][epoch][custodian][attester];
    }

    // gets the unstake for a given attester at the current epoch
    function getCurrentEpochUnstake(
        address domain,
        address custodian,
        address attester
    ) public view returns (uint256) {
        return
            getUnstakeAtEpoch(
                domain,
                getCurrentEpoch(domain),
                custodian,
                attester
            );
    }

    // gets the total stake for a given epoch
    function getTotalStakeForEpoch(
        address domain,
        uint256 epoch,
        address custodian
    ) public view returns (uint256) {
        return epochTotalStakeByDomain[domain][epoch][custodian];
    }

    // gets the total stake for the current epoch
    function getTotalStakeForCurrentEpoch(
        address domain,
        address custodian
    ) public view returns (uint256) {
        return
            getTotalStakeForEpoch(domain, getCurrentEpoch(domain), custodian);
    }

    // stakes for the next epoch
    function stake(
        address domain,
        IERC20 custodian,
        uint256 amount
    ) external onlyRole(WHITELIST_ROLE) nonReentrant {
        // add the attester to the list of attesters
        attestersByDomain[domain].add(msg.sender);

        // add the custodian to the list of custodians
        // custodiansByDomain[domain].add(address(custodian)); // Note: we don't want this to take place by default as it opens up an opportunity for a gas attack by generating a large number of custodians for the domain contract to track

        // check the balance of the token before transfer
        uint256 balanceBefore = token.balanceOf(address(this));

        // transfer the stake to the contract
        // if the transfer is not using a custodian, the custodian is the token itself
        // hence this works
        // ! In general with this pattern, the custodian must be careful about not over-approving the token.
        custodian.transferFrom(msg.sender, address(this), amount);

        // require that the balance of the actual token has increased by the amount
        if (token.balanceOf(address(this)) != balanceBefore + amount)
            revert CustodianTransferAmountMismatch();

        // set the attester to stake for the next epoch
        _addStake(
            domain,
            getNextEpochByBlockTime(domain),
            address(custodian),
            msg.sender,
            amount
        );

        // Let the world know that the attester has staked
        emit AttesterStaked(
            domain,
            getNextEpoch(domain),
            address(custodian),
            msg.sender,
            amount
        );
    }

    // unstakes an amount for the next epoch
    function unstake(
        address domain,
        address custodian,
        uint256 amount
    ) external onlyRole(WHITELIST_ROLE) nonReentrant {
        // indicate that we are going to unstake this amount in the next epoch
        // ! this doesn't actually happen until we roll over the epoch
        // note: by tracking in the next epoch we need to make sure when we roll over an epoch we check the amount rolled over from stake by the unstake in the next epoch
        _addUnstake(
            domain,
            getNextEpochByBlockTime(domain),
            custodian,
            msg.sender,
            amount
        );

        emit AttesterUnstaked(
            domain,
            getNextEpoch(domain),
            custodian,
            msg.sender,
            amount
        );
    }

    // rolls over the stake and unstake for a given attester
    function _rollOverAttester(
        address domain,
        uint256 epochNumber,
        address custodian,
        address attester
    ) internal {
        // the amount of stake rolled over is stake[currentEpoch] - unstake[nextEpoch]
        uint256 stakeAmount = getStakeAtEpoch(
            domain,
            epochNumber,
            custodian,
            attester
        );
        uint256 unstakeAmount = getUnstakeAtEpoch(
            domain,
            epochNumber + 1,
            custodian,
            attester
        );
        if (unstakeAmount > stakeAmount) {
            unstakeAmount = stakeAmount;
        }
        uint256 remainder = stakeAmount - unstakeAmount;

        _addStake(domain, epochNumber + 1, custodian, attester, remainder);

        // the unstake is then paid out
        // note: this is the only place this takes place
        // there's not risk of double payout, so long as rollOverattester is only called once per epoch
        // this should be guaranteed by the implementation, but we may want to create a withdrawal mapping to ensure this
        _payAttester(address(this), attester, custodian, unstakeAmount);

        emit AttesterEpochRolledOver(
            attester,
            epochNumber,
            custodian,
            stakeAmount,
            unstakeAmount
        );
    }

    function _rollOverEpoch(address domain, uint256 epochNumber) internal {
        // iterate over the attester set
        // * complexity here can be reduced by actually mapping attesters to their token and custodian
        for (uint256 i = 0; i < attestersByDomain[domain].length(); i++) {
            address attester = attestersByDomain[domain].at(i);

            for (uint256 j = 0; j < custodiansByDomain[domain].length(); j++) {
                address custodian = custodiansByDomain[domain].at(j);

                _rollOverAttester(domain, epochNumber, custodian, attester);
            }
        }

        // increment the current epoch
        currentEpochByDomain[domain] = epochNumber + 1;

        emit EpochRolledOver(domain, epochNumber);
    }

    function rollOverEpoch() external {
        _rollOverEpoch(msg.sender, getCurrentEpoch(msg.sender));
    }

    /**
     * @dev Slash an attester's stake
     * @param domain The domain of the attester
     * @param epoch The epoch in which the slash is attempted
     * @param custodian The custodian of the token
     * @param attester The attester to slash
     * @param amount The amount to slash
     */
    function _slashStake(
        address domain,
        uint256 epoch,
        address custodian,
        address attester,
        uint256 amount
    ) internal {
        // stake slash will always target this epoch
        uint256 targetEpoch = epoch;
        uint256 stakeForEpoch = getStakeAtEpoch(
            domain,
            targetEpoch,
            custodian,
            attester
        );

        // deduct the amount from the attester's stake, account for underflow
        if (stakeForEpoch < amount) {
            _removeStake(
                domain,
                targetEpoch,
                custodian,
                attester,
                stakeForEpoch
            );
        } else {
            _removeStake(domain, targetEpoch, custodian, attester, amount);
        }
    }

    /**
     * @dev Slash an attester's unstake
     * @param domain The domain of the attester
     * @param epoch The epoch in which the slash is attempted, i.e., epoch - 1 of the epoch where the unstake will be removed
     * @param custodian The custodian of the token
     * @param attester The attester to slash
     */
    function _slashUnstake(
        address domain,
        uint256 epoch,
        address custodian,
        address attester
    ) internal {
        // unstake slash will always target the next epoch
        uint256 stakeForEpoch = getStakeAtEpoch(
            domain,
            epoch,
            custodian,
            attester
        );
        uint256 targetEpoch = epoch + 1;
        uint256 unstakeForEpoch = getUnstakeAtEpoch(
            domain,
            targetEpoch,
            custodian,
            attester
        );

        if (unstakeForEpoch > stakeForEpoch) {
            // if you are trying to unstake more than is staked

            // set the unstake to the maximum possible amount
            _setUnstake(
                domain,
                targetEpoch,
                custodian,
                attester,
                stakeForEpoch
            );
        }
    }

    function slash(
        address[] calldata custodians,
        address[] calldata attesters,
        uint256[] calldata amounts,
        uint256[] calldata refundAmounts
    ) public nonReentrant {
        for (uint256 i = 0; i < attesters.length; i++) {
            // issue a refund that is the min of the stake balance, the amount to be slashed, and the refund amount
            // this is to prevent a Domain from trying to have this contract pay out more than has been staked
            uint256 refundAmount = Math.min(
                getStakeAtEpoch(
                    msg.sender,
                    getCurrentEpoch(attesters[i]),
                    custodians[i],
                    attesters[i]
                ),
                Math.min(amounts[i], refundAmounts[i])
            );
            _payAttester(
                address(this), // this contract is paying the attester, it should always have enough balance
                attesters[i],
                custodians[i],
                refundAmount
            );

            // slash both stake and unstake so that the weight of the attester is reduced and they can't withdraw the unstake at the next epoch
            _slashStake(
                msg.sender,
                getCurrentEpoch(msg.sender),
                custodians[i],
                attesters[i],
                amounts[i]
            );

            _slashUnstake(
                msg.sender,
                getCurrentEpoch(msg.sender),
                custodians[i],
                attesters[i]
            );
        }
    }

    function _payAttester(
        address from,
        address attester,
        address custodian,
        uint256 amount
    ) internal {
        if (from == address(this)) {
            // this contract is paying the attester
            if (address(token) == custodian) {
                // if there isn't a custodian...
                token.transfer(attester, amount); // just transfer the token
            } else {
                // approve the custodian to spend the base token
                token.approve(custodian, amount);

                // purchase the custodial token for the attester
                ICustodianToken(custodian).buyCustodialToken(attester, amount);
            }
        } else {
            // This can be used by the domain to pay the attester, but it's just as convenient for the domain to reward the attester directly.
            // This is, currently, there is no added benefit of issuing a reward through this contract--other than Riccardian clarity.

            // somebody else is trying to pay the attester, e.g., the domain
            if (address(token) == custodian) {
                // if there isn't a custodian...
                token.transferFrom(from, attester, amount); // just transfer the token
            } else {
                // purchase the custodial token for the attester
                ICustodianToken(custodian).buyCustodialTokenFrom(
                    from,
                    attester,
                    amount
                );
            }
        }
    }

    function reward(
        address[] calldata attesters,
        uint256[] calldata amounts,
        address[] calldata custodians
    ) public nonReentrant {
        // note: you may want to apply this directly to the attester's stake if the Domain sets an automatic restake policy
        for (uint256 i = 0; i < attesters.length; i++) {
            // pay the attester
            _payAttester(msg.sender, attesters[i], custodians[i], amounts[i]);
        }
    }

    function whitelistAddress(
        address addr
    ) external onlyRole(DEFAULT_ADMIN_ROLE) {
        grantRole(WHITELIST_ROLE, addr);
    }

    function removeAddressFromWhitelist(
        address addr
    ) external onlyRole(DEFAULT_ADMIN_ROLE) {
        revokeRole(WHITELIST_ROLE, addr);
    }
}
