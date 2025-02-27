// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;
import "forge-std/console.sol";
import {BaseStaking} from "./base/BaseStaking.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {IERC20} from "@openzeppelin/contracts/interfaces/IERC20.sol";
import {Address} from "@openzeppelin/contracts/utils/Address.sol";
import {ICustodianToken} from "../token/custodian/CustodianToken.sol";
import {Math} from "@openzeppelin/contracts/utils/math/Math.sol";
import {MovementStakingStorage, EnumerableSet} from "./MovementStakingStorage.sol";
import {IMovementStaking} from "./interfaces/IMovementStaking.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";

// TODO Error: "Contract "MovementStaking" should be marked as abstract.(3656)"
contract MovementStaking is
    MovementStakingStorage,
    IMovementStaking,
    BaseStaking,
    ReentrancyGuard
{
    using EnumerableSet for EnumerableSet.AddressSet;

    /// @notice Error thrown when trying to get epoch but duration not set
    error EpochDurationNotSet();

    function initialize(IERC20 _token) public initializer {
        __BaseStaking_init_unchained();
        token = _token;
    }

    /// @notice Registers a domain and sets the epoch duration
    function registerDomain(
        uint256 epochDuration,
        address[] calldata custodians
    ) external nonReentrant {
        address domain = msg.sender;
        epochDurationByDomain[domain] = epochDuration;

        for (uint256 i = 0; i < custodians.length; i++) {
            registeredCustodiansByDomain[domain].add(custodians[i]);
        }
    }

    /// @notice Gets all custodians who are registered for the given domain
    function getRegisteredCustodians(
        address domain
    ) public view returns (address[] memory) {
        // todo: we probably want to figure out a better API which still allows domains to interpret custodians as they see fit
        address[] memory custodians = new address[](
            registeredCustodiansByDomain[domain].length()
        );
        for (uint256 i = 0; i < registeredCustodiansByDomain[domain].length(); i++) {
            custodians[i] = registeredCustodiansByDomain[domain].at(i);
        }
        return custodians;
    }

    /// @notice Gets all attesters who are registered for the given domain
    function getRegisteredAttesters(
        address domain
    ) public view returns (address[] memory) {
        address[] memory attesters = new address[](
            registeredAttestersByDomain[domain].length()
        );
        for (uint256 i = 0; i < registeredAttestersByDomain[domain].length(); i++) {
            attesters[i] = registeredAttestersByDomain[domain].at(i);
        }
        return attesters;
    }

    /// @notice Gets all attesters who have stake in the current accepting epoch
    function getStakedAttestersForAcceptingEpoch(
        address domain
    ) public view returns (address[] memory) {
        // First get all registered attesters
        uint256 totalAttesters = registeredAttestersByDomain[domain].length();
        
        // Count attesters with stake
        uint256 activeAttesterCount = 0;
        for (uint256 i = 0; i < totalAttesters; i++) {
            address attester = registeredAttestersByDomain[domain].at(i);
            if (getAttesterStakeForAcceptingEpoch(domain, attester) > 0) {
                activeAttesterCount++;
            }
        }

        // Create array of active attesters
        address[] memory activeAttesters = new address[](activeAttesterCount);
        uint256 activeIndex = 0;
        for (uint256 i = 0; i < totalAttesters; i++) {
            address attester = registeredAttestersByDomain[domain].at(i);
            if (getAttesterStakeForAcceptingEpoch(domain, attester) > 0) {
                activeAttesters[activeIndex] = attester;
                activeIndex++;
            }
        }

        return activeAttesters;
    }

    /// @notice Gets the epoch duration for the given domain
    function getEpochDuration(address domain) public view returns (uint256) {
        return epochDurationByDomain[domain];
    }

    function acceptGenesisCeremony() public nonReentrant {
        address domain = msg.sender;

        if (domainGenesisAccepted[domain]) revert GenesisAlreadyAccepted();
        domainGenesisAccepted[domain] = true;
        
        assert(epochDurationByDomain[domain] > 0);

        // roll over from 0 (genesis) to current epoch by L1Block time
        currentAcceptingEpochByDomain[domain] = getEpochByL1BlockTime(domain);

        for (uint256 i = 0; i < registeredAttestersByDomain[domain].length(); i++) {
            address attester = registeredAttestersByDomain[domain].at(i);

            for (uint256 j = 0; j < registeredCustodiansByDomain[domain].length(); j++) {
                address custodian = registeredCustodiansByDomain[domain].at(j);

                // get the genesis stake for the attester
                uint256 attesterStake = getStake(
                    domain,
                    0,
                    custodian,
                    attester
                );

                // roll over the genesis stake to the current epoch
                // except if the current epoch is 0, because we are already in the first epoch
                if (getAcceptingEpoch(domain) > 0) {
                if (getAcceptingEpoch(domain) > 0) {
                    _addStake(
                        domain,
                        getAcceptingEpoch(domain),
                        custodian,
                        attester,
                        attesterStake
                    );
                }
            }
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
        stakesByDomainEpochCustodianAttester[domain][epoch][custodian][attester] += amount;
        stakesByDomainEpochCustodian[domain][epoch][custodian] += amount;
    }

    function _removeStake(
        address domain,
        uint256 epoch,
        address custodian,
        address attester,
        uint256 amount
    ) internal {
        stakesByDomainEpochCustodianAttester[domain][epoch][custodian][attester] -= amount;
        stakesByDomainEpochCustodian[domain][epoch][custodian] -= amount;
    }

    function _addUnstake(
        address domain,
        uint256 epoch,
        address custodian,
        address attester,
        uint256 amount
    ) internal {
        unstakesByDomainEpochCustodianAttester[domain][epoch][custodian][attester] += amount;
    }

    function _removeUnstake(
        address domain,
        uint256 epoch,
        address custodian,
        address attester,
        uint256 amount
    ) internal {
        unstakesByDomainEpochCustodianAttester[domain][epoch][custodian][attester] -= amount;
    }

    function _setUnstake(
        address domain,
        uint256 epoch,
        address custodian,
        address attester,
        uint256 amount
    ) internal {
        unstakesByDomainEpochCustodianAttester[domain][epoch][custodian][attester] = amount;
    }

    // gets the would be epoch for the current L1-block time. 
    // TODO: for liveness of the protocol it should be possible that newer epochs can accept L2-block-batches that are before the current epoch (IF the previous epoch has stopped being live)
    function getEpochByL1BlockTime(address domain) public view returns (uint256) {
        if (epochDurationByDomain[domain] == 0) revert EpochDurationNotSet();
        return block.timestamp / epochDurationByDomain[domain];
    }

    // gets the current epoch up to which superBlocks have been accepted
    function getAcceptingEpoch(address domain) public view returns (uint256) {
        return currentAcceptingEpochByDomain[domain];
    }

    /// @notice Gets the next accepting epoch number
    /// @dev Special handling for genesis state (epoch 0):
    /// @dev If getAcceptingEpoch(domain) == 0, returns 0 to stay in genesis until ceremony completes
    function getNextAcceptingEpochWithException(address domain) public view returns (uint256) {
        return getAcceptingEpoch(domain) == 0 ? 0 : getAcceptingEpoch(domain) + 1;
    }

    /// @notice Gets the next present epoch number
    /// @dev Special handling for genesis state (accepting epoch 0):
    /// @dev If getAcceptingEpoch(domain) == 0, returns 0 to stay in genesis until ceremony completes
    function getNextPresentEpochWithException(address domain) public view returns (uint256) {
        return getAcceptingEpoch(domain) == 0 ? 0 : getEpochByL1BlockTime(domain) + 1;
    }

    /// @dev gets the stake for a given epoch for a given {attester,custodian} tuple
    function getStake(
        address domain,
        uint256 epoch,
        address custodian,
        address attester
    ) public view returns (uint256) {
        return stakesByDomainEpochCustodianAttester[domain][epoch][custodian][attester];
    }

    /// @dev gets the stake for the accepting epoch for a given {attester,custodian} tuple
    function getStakeForAcceptingEpoch(
        address domain,
        address custodian,
        address attester
    ) public view returns (uint256) {
        return
            getStake(
                domain,
                getAcceptingEpoch(domain),
                custodian,
                attester
            );
    }

    /// @dev gets the unstake for a given epoch for a given {attester,custodian} tuple

    function getUnstake(
        address domain,
        uint256 epoch,
        address custodian,
        address attester
    ) public view returns (uint256) {
        return unstakesByDomainEpochCustodianAttester[domain][epoch][custodian][attester];
    }

    /// @dev gets the unstake for the accepting epoch for a given {attester,custodian} tuple
    function getUnstakeForAcceptingEpoch(
        address domain,
        address custodian,
        address attester
    ) public view returns (uint256) {
        return
            getUnstake(
                domain,
                getAcceptingEpoch(domain),
                custodian,
                attester
            );
    }

    /// @dev gets the total stake for a given epoch for a given custodian
    function getCustodianStake(
        address domain,
        uint256 epoch,
        address custodian
    ) public view returns (uint256) {
        return stakesByDomainEpochCustodian[domain][epoch][custodian];
    }

    /// @dev gets the total stake for the accepting epoch for a given custodian
    function getCustodianStakeForAcceptingEpoch(
        address domain,
        address custodian
    ) public view returns (uint256) {
        return
            getCustodianStake(domain, getAcceptingEpoch(domain), custodian);
    }

    function getAttesterStake(address domain, uint256 epoch, address attester) public view returns (uint256) {
        uint256 attesterStake = 0;
        for (uint256 i = 0; i < registeredCustodiansByDomain[domain].length(); i++) {
            attesterStake += getStake(domain, epoch, registeredCustodiansByDomain[domain].at(i), attester);
        }
        return attesterStake;
    }

    function getAttesterStakeForAcceptingEpoch(address domain, address attester) public view returns (uint256) {
        return getAttesterStake(domain, getAcceptingEpoch(domain), attester);
    }

    /// @notice Stakes for the next epoch
    function stake(
        address domain,
        IERC20 custodian,
        uint256 amount
    ) external onlyRole(WHITELIST_ROLE) nonReentrant {
        // add the attester to the list of attesters
        registeredAttestersByDomain[domain].add(msg.sender);

        // add the custodian to the list of custodians
        // registeredCustodiansByDomain[domain].add(address(custodian)); // Note: we don't want this to take place by default as it opens up an opportunity for a gas attack by generating a large number of custodians for the domain contract to track

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

        // set the attester to stake for the next accepting epoch
        _addStake(
            domain,
            // TODO should this not be getNextAcceptingEpochWithException(domain)?
            // getNextPresentEpochWithException(domain),
            getNextAcceptingEpochWithException(domain),
            address(custodian),
            msg.sender,
            amount
        );

        // Let the world know that the attester has staked
        emit AttesterStaked(
            domain,
            getNextAcceptingEpochWithException(domain),
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
            // TODO should this not be getNextAcceptingEpochWithException(domain)?
            // getNextPresentEpochWithException(domain),
            getNextAcceptingEpochWithException(domain),
            custodian,
            msg.sender,
            amount
        );

        emit AttesterUnstaked(
            domain,
            getNextAcceptingEpochWithException(domain),
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
        // the amount of stake rolled over is stake[currentAcceptingEpoch] - unstake[nextEpoch]
        uint256 stakeAmount = getStake(
            domain,
            epochNumber,
            custodian,
            attester
        );
        uint256 unstakeAmount = getUnstake(
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
        for (uint256 i = 0; i < registeredAttestersByDomain[domain].length(); i++) {
            address attester = registeredAttestersByDomain[domain].at(i);

            for (uint256 j = 0; j < registeredCustodiansByDomain[domain].length(); j++) {
                address custodian = registeredCustodiansByDomain[domain].at(j);

                _rollOverAttester(domain, epochNumber, custodian, attester);
            }
        }

        // increment the current epoch
        currentAcceptingEpochByDomain[domain] = epochNumber + 1;

        emit EpochRolledOver(domain, epochNumber);
    }

    function rollOverEpoch() external {
        _rollOverEpoch(msg.sender, getAcceptingEpoch(msg.sender));
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
        uint256 stakeForEpoch = getStake(
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
        uint256 stakeForEpoch = getStake(
            domain,
            epoch,
            custodian,
            attester
        );
        uint256 targetEpoch = epoch + 1;
        uint256 unstakeForEpoch = getUnstake(
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
                getStake(
                    msg.sender,
                    getAcceptingEpoch(attesters[i]),
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
                getAcceptingEpoch(msg.sender),
                custodians[i],
                attesters[i],
                amounts[i]
            );

            _slashUnstake(
                msg.sender,
                getAcceptingEpoch(msg.sender),
                custodians[i],
                attesters[i]
            );
        }
    }

    /// @notice Custodian pays an attester
    // TODO these multiple if statements are a bit confusing at best. 
    // TODO This should be refactored and more individual functions created.
    // TODO e.g. _payAttesterFromContract, _payAttesterFromCustodian, _payAttesterFromToken
    function _payAttester(
        address from,
        address attester,
        address custodian,
        uint256 amount
    ) internal {
        console.log("[payAttester] From:", from);
        console.log("[payAttester] Attester:", attester);
        console.log("[payAttester] Custodian:", custodian);
        console.log("[payAttester] Amount:", amount);
        console.log("[payAttester] Token address:", address(token));
        console.log("[payAttester] Address of this:", address(this));
        if (from == address(this)) {
            // this contract is paying the attester
            console.log("[payAttester] From = contract");
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
                console.log("[payAttester] From = address(token)");
                // make an if statement to check if there is enough balance
                console.log("[payAttester] Balance of token:", token.balanceOf(from));
                console.log("[payAttester] Amount:", amount);
                if (token.balanceOf(from) < amount) {
                    console.log("[payAttester] insuffienct balance");
                    console.log("[payAttester] Balance of token:", token.balanceOf(from));
                    console.log("[payAttester] Amount:", amount);
                }
                token.transferFrom(from, attester, amount); // just transfer the token
                console.log("[payAttester] Successfully transferred from:", from);
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

    /// @notice Custodian rewards an attester
    /// @param attester The attester to reward
    /// @param amount The amount to reward
    /// @param custodian The custodian of the token from which to reward the attester
    function reward(
        address attester,
        uint256 amount,
        address custodian
    ) public nonReentrant {
        _payAttester(msg.sender, attester, custodian, amount);
    }

    /// @notice An array of custodians reward an array of attesters
    /// @param attesters The attesters to reward
    /// @param amounts The amounts to reward
    /// @param custodians The custodians of the token from which to reward the attesters    
    function rewardArray(
        address[] calldata attesters,
        uint256[] calldata amounts,
        address[] calldata custodians
    ) public nonReentrant {
        // note: you may want to apply this directly to the attester's stake if the Domain sets an automatic restake policy
        for (uint256 i = 0; i < attesters.length; i++) {
            _payAttester(msg.sender, attesters[i], custodians[i], amounts[i]);
        }
    }



    /// @notice Whitelist an address to be used as an attester or custodian. 
    /// @notice Whitelisting means that the address is allowed to stake and unstake
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

    /// @notice Computes total stake across all custodians and attesters for an epoch
    function computeAllStake(
        address domain,
        uint256 epoch
    ) public view returns (uint256) {
        address[] memory custodians = getRegisteredCustodians(domain);
        address[] memory attesters = getRegisteredAttesters(domain);
        uint256 totalStake = 0;

        for (uint256 i = 0; i < custodians.length; i++) {
            for (uint256 j = 0; j < attesters.length; j++) {
                totalStake += getStake(domain, epoch, custodians[i], attesters[j]);
            }
        }
        return totalStake;
    }

    /// @notice Computes total stake across all custodians and attesters for the current accepting epoch
    /// @param domain The domain to compute total stake for
    function computeAllStakeForAcceptingEpoch(
        address domain
    ) public view returns (uint256) {
        return computeAllStake(domain, getAcceptingEpoch(domain));
    }

}
