// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {EnumerableSet} from "@openzeppelin/contracts/utils/structs/EnumerableSet.sol";
import {Initializable} from "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";
import {MovementStaking, IMovementStaking} from "../staking/MovementStaking.sol";
import {MCRStorage} from "./MCRStorage.sol";
import {BaseSettlement} from "./settlement/BaseSettlement.sol";
import {IMCR} from "./interfaces/IMCR.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "forge-std/console.sol";

contract MCR is Initializable, BaseSettlement, MCRStorage, IMCR {

    // A role for setting commitments
    bytes32 public constant COMMITMENT_ADMIN = keccak256("COMMITMENT_ADMIN");

    // Trusted attesters admin
    bytes32 public constant TRUSTED_ATTESTER = keccak256("TRUSTED_ATTESTER");

    /// @notice Error thrown when postconfirmer term is greater than 256 blocks
    error PostconfirmerDurationTooLong();

    /// @notice Error thrown when postconfirmer term is too large for epoch duration
    error PostconfirmerDurationTooLongForEpoch();

    /// @notice Error thrown when minimum commitment age is greater than epoch duration
    error minCommitmentAgeForPostconfirmationTooLong();

    /// @notice Error thrown when maximum postconfirmer non-reactivity time is greater than epoch duration
    error postconfirmerPrivilegeDurationTooLong();

    // ----------------------------------------------------------------
    // -------- 1. Role & Permission Management -----------------------
    // ----------------------------------------------------------------

    function grantCommitmentAdmin(address account) public {
        require(
            hasRole(DEFAULT_ADMIN_ROLE, msg.sender),
            "ADD_COMMITMENT_ADMIN_IS_ADMIN_ONLY"
        );
        grantRole(COMMITMENT_ADMIN, account);
    }

    function batchGrantCommitmentAdmin(address[] memory accounts) public {
        require(
            hasRole(DEFAULT_ADMIN_ROLE, msg.sender),
            "ADD_COMMITMENT_ADMIN_IS_ADMIN_ONLY"
        );
        for (uint256 i = 0; i < accounts.length; i++) {
            grantRole(COMMITMENT_ADMIN, accounts[i]);
        }
    }

    function grantTrustedAttester(address attester) public onlyRole(COMMITMENT_ADMIN) {
        grantRole(TRUSTED_ATTESTER, attester);
    }

    function batchGrantTrustedAttester(address[] memory attesters) public onlyRole(COMMITMENT_ADMIN) {
        for (uint256 i = 0; i < attesters.length; i++) {
            grantRole(TRUSTED_ATTESTER, attesters[i]);
        }
    }

    // ----------------------------------------------------------------
    // -------- 2. Contract Initialization & Configuration ------------
    // ----------------------------------------------------------------

    function initialize(
        IMovementStaking _stakingContract,
        uint256 _lastPostconfirmedSuperBlockHeight,
        uint256 _leadingSuperBlockTolerance,
        uint256 _epochDuration, // in time units
        address[] memory _custodians,
        uint256 _postconfirmerDuration, // in time units
        address _moveTokenAddress  // the primary custodian for rewards in the staking contract
    ) public initializer {
        __BaseSettlement_init_unchained();
        stakingContract = _stakingContract;
        leadingSuperBlockTolerance = _leadingSuperBlockTolerance;
        lastPostconfirmedSuperBlockHeight = _lastPostconfirmedSuperBlockHeight;
        stakingContract.registerDomain(_epochDuration, _custodians);
        grantCommitmentAdmin(msg.sender);
        grantTrustedAttester(msg.sender);
        postconfirmerDuration = _postconfirmerDuration;
        moveTokenAddress = _moveTokenAddress;

        // Set default values to 1/10th of epoch duration
        // NOTE since epochduration divided by 10 may not be an exact integer, the start and end of these windows may drift within an epoch over time.
        // NOTE Consequently to remain on the safe side, these values should remain a small fraction of the epoch duration. 
        // NOTE If they are small at most only the last fraction within an epoch will behave differently.
        // TODO Examine the effects of the above.
        minCommitmentAgeForPostconfirmation = _epochDuration / 10;
        postconfirmerPrivilegeDuration = _epochDuration / 10;
        rewardPerAttestationPoint = 1;
        rewardPerPostconfirmationPoint = 1;
    }

    /// @notice Accepts the genesis ceremony.
    function acceptGenesisCeremony() public {
        require(hasRole(DEFAULT_ADMIN_ROLE, msg.sender), "ACCEPT_GENESIS_CEREMONY_IS_ADMIN_ONLY");
        stakingContract.acceptGenesisCeremony();
    }

    /// @notice Sets the postconfirmer term duration, must be less than epoch duration
    /// @param _postconfirmerDuration New postconfirmer term duration in time units
    function setPostconfirmerDuration(uint256 _postconfirmerDuration) public onlyRole(COMMITMENT_ADMIN) {
        // Ensure postconfirmer term is sufficiently small compared to epoch duration
        uint256 epochDuration = stakingContract.getEpochDuration(address(this));

        // TODO If we would use block heights instead of timestamps we could handle everything much smoother.
        if (2 * _postconfirmerDuration >= epochDuration ) {
            revert PostconfirmerDurationTooLongForEpoch();
        }
        postconfirmerDuration = _postconfirmerDuration;
    }

    function getPostconfirmerDuration() public view returns (uint256) {
        return postconfirmerDuration;
    }

    /// @notice Sets the maximum time the postconfirmer can be non-reactive to an honest superBlock commitment
    /// @param _postconfirmerPrivilegeDuration  maximum time the postconfirmer is permitted to be non-reactive to an honest superBlock commitment
    function setPostconfirmerPrivilegeDuration(uint256 _postconfirmerPrivilegeDuration) public onlyRole(COMMITMENT_ADMIN) {
        // Ensure max privilege time is not too large
        if (_postconfirmerPrivilegeDuration >= stakingContract.getEpochDuration(address(this)) - getMinCommitmentAgeForPostconfirmation()) {
            revert postconfirmerPrivilegeDurationTooLong();
        }
        postconfirmerPrivilegeDuration = _postconfirmerPrivilegeDuration;
    }

    /// @notice Gets the maximum time the postconfirmer can be non-reactive to an honest superBlock commitment
    /// @return The maximum time the postconfirmer can be non-reactive to an honest superBlock commitment
    function getPostconfirmerPrivilegeDuration() public view returns (uint256) {
        return postconfirmerPrivilegeDuration;
    }

    /// @notice Sets the minimum time that must pass before a commitment can be postconfirmed
    /// @param _minCommitmentAgeForPostconfirmation New minimum commitment age 
    // TODO we also require a check when setting the epoch length that it is larger than the min commitment age
    // TODO we need to set these values such that it works for postconfirmer Term and postconfirmerPrivilegeDuration, etc... there are many constraints here.
    function setMinCommitmentAgeForPostconfirmation(uint256 _minCommitmentAgeForPostconfirmation) public onlyRole(COMMITMENT_ADMIN) {
        // Ensure min age is less than epoch duration to allow postconfirmation within same epoch
        if (_minCommitmentAgeForPostconfirmation >= stakingContract.getEpochDuration(address(this)) - getPostconfirmerPrivilegeDuration()) {
            revert minCommitmentAgeForPostconfirmationTooLong();
        }
        minCommitmentAgeForPostconfirmation = _minCommitmentAgeForPostconfirmation;
    }

    function getMinCommitmentAgeForPostconfirmation() public view returns (uint256) {
        return minCommitmentAgeForPostconfirmation;
    }

    function setOpenAttestationEnabled(bool enabled) public onlyRole(COMMITMENT_ADMIN) {
        openAttestationEnabled = enabled;
    }

    // ----------------------------------------------------------------
    // -------- 3. Epoch & Time Management ---------------------------
    // ----------------------------------------------------------------

    /// @notice Gets the time at which the current epoch started
    function getEpochStartTime() public view returns (uint256) {
        uint256 currentTime = block.timestamp;
        return currentTime - (currentTime % stakingContract.getEpochDuration(address(this)));
    }

    // gets the present epoch
    function getPresentEpoch() public view returns (uint256) {
        return stakingContract.getEpochByL1BlockTime(address(this));
    }

    // gets the accepting epoch
    function getAcceptingEpoch() public view returns (uint256) {
        return stakingContract.getAcceptingEpoch(address(this));
    }

    // gets the next accepting epoch (unless we are at genesis)
    function getNextAcceptingEpochWithException() public view returns (uint256) {
        return stakingContract.getNextAcceptingEpochWithException(address(this));
    }

    /// @notice Gets the time at which the current postconfirmer's term started
    function getPostconfirmerStartTime() public view returns (uint256) {
        uint256 currentTime = block.timestamp;
        // We reset the times to match the start of epochs. This ensures every epoch has the same setup.
        uint256 currentTimeCorrected = currentTime % stakingContract.getEpochDuration(address(this));
        return currentTimeCorrected - (currentTimeCorrected % postconfirmerDuration);
    }

    /// @notice Determines the postconfirmer in the accepting epoch using L1 block hash as a source of randomness
    // At the border between epochs this is not ideal as getPostconfirmer works on blocks and epochs works with time. 
    // Thus we must consider the edge cases where the postconfirmer is only active for a short time.
    function getPostconfirmer() public view returns (address) {
        // TODO unless we swap with everything, including epochs, we must use block.timestamp. 
        // However, to get easy access to L1 randomness we need to know the block at which the postconfirmer started, which is expensive (unless we count in blocks instead of time)
        // TODO as long as we do not swap to block.number, the randomness below is precictable.
        uint256 randSeed1 = getPostconfirmerStartTime();
        uint256 randSeed2 = getEpochStartTime();
        address[] memory attesters = stakingContract.getStakedAttestersForAcceptingEpoch(address(this));
        uint256 postconfirmerIndex = uint256(keccak256(abi.encodePacked(randSeed1, randSeed2))) % attesters.length; // randomize the order of the attesters
        return attesters[postconfirmerIndex];
    }

    // ----------------------------------------------------------------
    // -------- 4. Commitment Management ----------
    // ----------------------------------------------------------------

    // creates a commitment
    function createSuperBlockCommitment(
        uint256 height,
        bytes32 commitment,
        bytes32 blockId
    ) public pure returns (SuperBlockCommitment memory) {
        return SuperBlockCommitment(height, commitment, blockId);
    }

    /// @dev submits a superBlock commitment for an attester.
    function submitSuperBlockCommitmentForAttester(
        address attester,
        SuperBlockCommitment memory superBlockCommitment
    ) internal {
        // Attester has already committed to a superBlock at this height
        if (commitments[superBlockCommitment.height][attester].height != 0)
            revert AttesterAlreadyCommitted();

        // note: do no uncomment the below, we want to allow this in case we have lagging attesters
        // Attester has committed to an already postconfirmed superBlock
        // if ( lastPostconfirmedSuperBlockHeight > superBlockCommitment.height) revert AlreadyAcceptedSuperBlock();
        // Attester has committed to a superBlock too far ahead of the last postconfirmed superBlock
        if (lastPostconfirmedSuperBlockHeight + leadingSuperBlockTolerance < superBlockCommitment.height) {
            revert AttesterAlreadyCommitted();
        }

        // assign the superBlock height to the present epoch if it hasn't been assigned yet
        // since any attester can submit a comittment for a superBlock height, the epoch assignment could differ 
        // from when the superBlock gets actually postconfirmed. This is limited by by leadingSuperBlockTolerance
        if (superBlockHeightAssignedEpoch[superBlockCommitment.height] == 0) {
            superBlockHeightAssignedEpoch[superBlockCommitment.height] = getPresentEpoch();
        }

        // register the attester's commitment
        commitments[superBlockCommitment.height][attester] = superBlockCommitment;
        
        // Record first seen timestamp if not already set
        TrySetCommitmentFirstSeenAt(superBlockCommitment.height, superBlockCommitment.commitment, block.timestamp);

        // increment the commitment count by stake
        uint256 attesterStakeForAcceptingEpoch = getAttesterStakeForAcceptingEpoch(attester);
        commitmentStake[superBlockCommitment.height][superBlockCommitment.commitment] += attesterStakeForAcceptingEpoch;

        emit SuperBlockCommitmentSubmitted(
            superBlockCommitment.blockId,
            superBlockCommitment.commitment,
            attesterStakeForAcceptingEpoch
        );
    }
    function submitSuperBlockCommitment(SuperBlockCommitment memory superBlockCommitment) public {
        require(
            openAttestationEnabled || hasRole(TRUSTED_ATTESTER, msg.sender),
            "UNAUTHORIZED_SUPERBLOCK_COMMITMENT"
        );
        submitSuperBlockCommitmentForAttester(msg.sender, superBlockCommitment);
    }

    function submitBatchSuperBlockCommitment(SuperBlockCommitment[] memory superBlockCommitments) public {
        require(
            openAttestationEnabled || hasRole(TRUSTED_ATTESTER, msg.sender),
            "UNAUTHORIZED_SUPERBLOCK_COMMITMENT"
        );
        for (uint256 i = 0; i < superBlockCommitments.length; i++) {
            submitSuperBlockCommitmentForAttester(msg.sender, superBlockCommitments[i]);
        }
    }
    function getValidatorCommitmentAtSuperBlockHeight(
        uint256 height,
        address attester
    ) public view returns (SuperBlockCommitment memory) {
        return commitments[height][attester];
    }

    // gets the max tolerable superBlock height
    function getMaxTolerableSuperBlockHeight() public view returns (uint256) {
        return lastPostconfirmedSuperBlockHeight + leadingSuperBlockTolerance;
    }
    /// @notice Gets the commitment submitted by an attester for a given height
    function getCommitmentByAttester(uint256 height, address attester) public view returns (SuperBlockCommitment memory) {
        return commitments[height][attester];
    }

    /// @notice Gets the epoch assigned to a superblock height
    function getSuperBlockHeightAssignedEpoch(uint256 height) public view returns (uint256) {
        return superBlockHeightAssignedEpoch[height];
    }

    // TODO use this to limit the postconfirmations on new commits ( we need to give time to attesters to submit their commitments )
    /// @notice get the timestamp when a commitment was first seen
    function getCommitmentFirstSeenAt(SuperBlockCommitment memory superBlockCommitment) public view returns (uint256) {
        return commitmentFirstSeenAt[superBlockCommitment.height][superBlockCommitment.commitment];
    }

    /// @notice Sets the timestamp when a commitment was first seen
    function TrySetCommitmentFirstSeenAt(uint256 height, bytes32 commitment, uint256 timestamp) internal {
        if (commitmentFirstSeenAt[height][commitment] != 0) {
            // do not set if already set
            console.log("[TrySetCommitmentFirstSeenAt] commitment first seen at is already set");
            return;
        } else if (timestamp == 0) {
            // no need to set if timestamp is 0. This if may be redundant though.
            console.log("[TrySetCommitmentFirstSeenAt] timestamp is 0");
            return;
        }
        commitmentFirstSeenAt[height][commitment] = timestamp;
    }

    // ----------------------------------------------------------------
    // -------- 5. Postconfirmation and Rollover Management ----------
    // ----------------------------------------------------------------

    /// @notice Gets the height of the last postconfirmed superblock
    function getLastPostconfirmedSuperBlockHeight() public view returns (uint256) {
        return lastPostconfirmedSuperBlockHeight;
    }

    function postconfirmSuperBlocksAndRollover() public {
        postconfirmAndRolloverWithAttester(msg.sender);
    }

    /// @notice The current postconfirmer can postconfirm a superBlock height, given there is a supermajority of stake on a commitment
    /// @notice If the current postconfirmer is live, we should not accept postconfirmations from voluntary attesters
    // TODO: this will be improved, such that voluntary attesters can postconfirm but will not be rewarded before the liveness period has ended
    /// @notice If the current postconfirmer is not live, we should accept postconfirmations from any attester
    // TODO: this will be improved, such that the first voluntary attester to do sowill be rewarded
    function postconfirmAndRolloverWithAttester(address /* attester */) internal {

        // keep ticking through postconfirmations and rollovers as long as the postconfirmer is permitted to do
        // ! rewards need to be 
        // ! - at least the cost for gas cost of postconfirmation
        // ! - reward the postconfirmer well to incentivize postconfirmation at every height
        while (attemptPostconfirmOrRollover(lastPostconfirmedSuperBlockHeight + 1)) {
        }
    }

    // Sets the postconfirmed commitment at a given superBlock height
    function setPostconfirmedCommitmentAtBlockHeight(SuperBlockCommitment memory superBlockCommitment) public {
        require(
            hasRole(COMMITMENT_ADMIN, msg.sender),
            "SET_LAST_POSTCONFIRMED_COMMITMENT_AT_HEIGHT_IS_COMMITMENT_ADMIN_ONLY"
        );
        versionedPostconfirmedSuperBlocks[postconfirmedSuperBlocksVersion][superBlockCommitment.height] = superBlockCommitment;  
    }

    // Forces the latest attestation by setting the superBlock height
    // Note: this only safe when we are running with a single validator as it does not zero out follow-on commitments.
    function forceLatestCommitment(SuperBlockCommitment memory superBlockCommitment) public {
        require(
            hasRole(COMMITMENT_ADMIN, msg.sender),
            "FORCE_LATEST_COMMITMENT_IS_COMMITMENT_ADMIN_ONLY"
        );
        setPostconfirmedCommitmentAtBlockHeight(superBlockCommitment);
    }

    function getPostconfirmedCommitment(uint256 height) public view returns (SuperBlockCommitment memory) {
        return versionedPostconfirmedSuperBlocks[postconfirmedSuperBlocksVersion][height];
    }
    /// @dev Postconfirms a superBlock commitment.
    /// @dev This function and attemptPostconfirmOrRollover() could call each other recursively, so we must ensure it's safe from re-entrancy
    function _postconfirmSuperBlockCommitment(SuperBlockCommitment memory superBlockCommitment, address attester) internal {
        uint256 currentAcceptingEpoch = getAcceptingEpoch();
        
        // get the epoch for the superBlock commitment
        // SuperBlock commitment is not in the current epoch, it cannot be postconfirmed. 
        // TODO: double check liveness conditions for the following critera
        if (superBlockHeightAssignedEpoch[superBlockCommitment.height] != currentAcceptingEpoch) {
            revert UnacceptableSuperBlockCommitment();
        }

        // ensure that the lastPostconfirmedSuperBlockHeight is exactly the superBlock height - 1
        if (lastPostconfirmedSuperBlockHeight != superBlockCommitment.height - 1) {
            revert UnacceptableSuperBlockCommitment();
        }

        // Record reward points for all attesters who committed to the winning commitment
        address[] memory attesters = getStakedAttestersForAcceptingEpoch();
        for (uint256 i = 0; i < attesters.length; i++) {
            if (commitments[superBlockCommitment.height][attesters[i]].commitment == superBlockCommitment.commitment) {
                attesterRewardPoints[currentAcceptingEpoch][attesters[i]]++;
            }
        }

        // Award points to postconfirmer
        if (!isWithinPostconfirmerPrivilegeDuration(superBlockCommitment)) { 
            // if we are outside the privilege window, for the postconfirmer reward anyone who postconfirms
            console.log("[postconfirmSuperBlockCommitment] privilege window is over");
            postconfirmerRewardPoints[currentAcceptingEpoch][attester] += 1;
        } else {
            // if we are within the privilege window, only award points to the postconfirmer
            // TODO optimization: even if the height has been volunteer postconfirmed we need to allow that that postconfirmer gets rewards, 
            // TODO otherwise weak postconfirmers may could get played (rich volunteer postconfirmers pay the fees and poor postconfirmers never get any reward) 
            // TODO but check if this is really required game theoretically.
            console.log("[postconfirmSuperBlockCommitment] current Postconfirmer is %s", getPostconfirmer());
            console.log("[postconfirmSuperBlockCommitment] attester is %s", attester);
            if (getPostconfirmer() == attester) {
                postconfirmerRewardPoints[currentAcceptingEpoch][attester] += 1;
            }
        }

        versionedPostconfirmedSuperBlocks[postconfirmedSuperBlocksVersion][superBlockCommitment.height] = superBlockCommitment;
        lastPostconfirmedSuperBlockHeight = superBlockCommitment.height;
        postconfirmedBy[superBlockCommitment.height] = attester;
        postconfirmedAtL1BlockHeight[superBlockCommitment.height] = block.number;
        postconfirmedAtL1BlockTimestamp[superBlockCommitment.height] = block.timestamp;

        // emit the superBlock postconfirmed event
        emit SuperBlockPostconfirmed(
            superBlockCommitment.blockId,
            superBlockCommitment.commitment,
            superBlockCommitment.height
        );
    }

    /// @dev nonReentrant because there is no need to reenter this function. It should be called iteratively. 
    /// @dev Marked on the internal method to simplify risks from complex calling patterns. This also calls an external contract.
    function rollOverEpoch() internal {
        // Get all attesters who earned points in the current epoch
        uint256 acceptingEpoch = getAcceptingEpoch();
        address[] memory attesters = getStakedAttestersForAcceptingEpoch();
        
        console.log("[rollOverEpoch] Attesters length at epoch %s is %s", acceptingEpoch, attesters.length);
        // reward
        for (uint256 i = 0; i < attesters.length; i++) {
            if (attesterRewardPoints[acceptingEpoch][attesters[i]] > 0) {
                // TODO: make this configurable and set it on instance creation
                uint256 reward = attesterRewardPoints[acceptingEpoch][attesters[i]] * rewardPerAttestationPoint * getAttesterStakeForAcceptingEpoch(attesters[i]);
                // the staking contract is the custodian
                console.log("[rollOverEpoch] Rewarding attester %s with %s", attesters[i], reward);
                console.log("[rollOverEpoch] Staking contract is %s", address(stakingContract));
                console.log("[rollOverEpoch] Move token address is %s", moveTokenAddress);
                console.log("[rollOverEpoch] msg.sender is %s", msg.sender);
                // rewards are currently paid out from the mcr domain
                stakingContract.rewardFromDomain(attesters[i], reward, moveTokenAddress);
                // TODO : check if we really have to keep attesterRewardPoints per epoch, or whether we could simply delete the points here for a given attester.
            }

            // Add postconfirmation rewards
            if (postconfirmerRewardPoints[acceptingEpoch][attesters[i]] > 0) {
                uint256 reward = postconfirmerRewardPoints[acceptingEpoch][attesters[i]] * rewardPerPostconfirmationPoint * getAttesterStakeForAcceptingEpoch(attesters[i]);
                console.log("[rollOverEpoch] Rewarding postconfirmer %s with %s", attesters[i], reward);
                console.log("[rollOverEpoch] Staking contract is %s", address(stakingContract));
                console.log("[rollOverEpoch] Move token address is %s", moveTokenAddress);
                console.log("[rollOverEpoch] msg.sender is %s", msg.sender);
                stakingContract.rewardFromDomain(attesters[i], reward, moveTokenAddress);
                // TODO : check if we really have to keep postconfirmerRewardPoints per epoch, or whether we could simply delete the points here for a given postconfirmer.
                // TODO also the postconfirmer list is super short. typically for a given height only the postconfirmer and at most the postconfirmer and a volunteer postconfirmer.
                // TODO So this can be heavily optimized.
            }
        }

        stakingContract.rollOverEpoch();
    }

    /// @notice Checks, for a given superBlock commitment, if the current L1 block time is within the postconfirmer's privilege window
    /// @dev The postconfirmer's privilege window is the time period when only the postconfirmer will get rewarded for postconfirmation
    function isWithinPostconfirmerPrivilegeDuration(SuperBlockCommitment memory superBlockCommitment) public view returns (bool) {
        if (getCommitmentFirstSeenAt(superBlockCommitment) == 0) {
            console.log("[isWithinPostconfirmerPrivilegeDuration] timestamp is not set for this commitment");
            return false;
        }
        // based on the first timestamp for the commitment we can determine if the postconfirmer has been live sufficiently recently
        // use getCommitmentFirstSeenAt, and the mappings
        console.log("[isWithinPostconfirmerPrivilegeDuration] getCommitmentFirstSeenAt", getCommitmentFirstSeenAt(superBlockCommitment));
        console.log("[isWithinPostconfirmerPrivilegeDuration] getMinCommitmentAgeForPostconfirmation", getMinCommitmentAgeForPostconfirmation());
        console.log("[isWithinPostconfirmerPrivilegeDuration] getPostconfirmerPrivilegeDuration", getPostconfirmerPrivilegeDuration());
        if (getCommitmentFirstSeenAt(superBlockCommitment) 
            + getMinCommitmentAgeForPostconfirmation() 
            + getPostconfirmerPrivilegeDuration() 
            < block.timestamp) {
            return false;
        }
        return true;
    }

    /// @dev it is possible if the accepting epoch is behind the presentEpoch that heights dont obtain enough votes in the assigned epoch. 
    /// @dev Moreover, due to the leadingBlockTolerance, the assigned epoch for a height could be ahead of the actual epoch. 
    /// @dev solution is to move to the next epoch and count votes there
    function attemptPostconfirmOrRollover(uint256 superBlockHeight) internal returns (bool) {
        console.log("[attemptPostconfirmOrRollover] attempting postconfirm or rollover at superblock height %s", superBlockHeight);
        uint256 superBlockEpoch = superBlockHeightAssignedEpoch[superBlockHeight];
        if (getLastPostconfirmedSuperBlockHeight() == 0) {
            console.log("[attemptPostconfirmOrRollover] genesis");
            // if there is no postconfirmed superblock we are at genesis
        } else {
            // ensure that the superBlock height is equal or above the lastPostconfirmedSuperBlockHeight
            uint256 previousSuperBlockEpoch = superBlockHeightAssignedEpoch[superBlockHeight-1];
            if (superBlockEpoch < previousSuperBlockEpoch  )  {
                address[] memory stakedAttesters = getStakedAttestersForAcceptingEpoch();
                // if there is at least one commitment at this superBlock height, we need to update once
                for (uint256 i = 0; i < stakedAttesters.length; i++) {
                    if (commitments[superBlockHeight][stakedAttesters[i]].height != 0) {
                        superBlockHeightAssignedEpoch[superBlockHeight] = previousSuperBlockEpoch;
                        break;
                    }
                }
                superBlockEpoch = previousSuperBlockEpoch;
            }
        }

        // if the accepting epoch is far behind the superBlockEpoch (which is determined by commitments measured in L1 block time), then the protocol was not live for a while
        // We keep rolling over the epoch (i.e. update stakes) until we catch up with the present epoch
        while (getAcceptingEpoch() < superBlockEpoch) {
            // TODO only permit rollover after some liveness criteria for the postconfirmer, as this is related to the reward model (rollovers should be rewarded)
            rollOverEpoch();
            console.log("[attemptPostconfirmOrRollover] rolled over epoch to %s", getAcceptingEpoch());
        }

        // TODO only permit postconfirmation after some liveness criteria for the postconfirmer, as this is related to the reward model (postconfirmation should be rewarded)

        uint256 supermajority = (2 * getTotalStake(superBlockEpoch)) / 3 + 1;
        address[] memory attesters = getStakedAttestersForAcceptingEpoch();

        // iterate over the attester set
        // TODO: randomize the order in which we check the attesters, which helps against spam of commitments. 
        // TODO: it may be more elegant to go through the commitments rather than the attesters..
        bool successfulPostconfirmation = false;
        for (uint256 i = 0; i < attesters.length; i++) {
            address attester = attesters[i];
            SuperBlockCommitment memory superBlockCommitment = commitments[superBlockHeight][attester];
            // check if the commitment has committed to the correct superBlock height
            // TODO: possibly this is not needed and we can remove the height from the commitment?
            if (superBlockCommitment.height != superBlockHeight) continue;

            // check the total stake on the commitment
            uint256 totalStakeOnCommitment = commitmentStake[superBlockCommitment.height][superBlockCommitment.commitment];

            if (totalStakeOnCommitment >= supermajority) {
                // Check if enough time has passed since commitment was first seen
                // if not enough time has passed, then no postconfirmation at this height can yet happen
                uint256 firstSeen = getCommitmentFirstSeenAt(superBlockCommitment);
                // we should jump out of the for loop entirely
                if (block.timestamp < firstSeen + minCommitmentAgeForPostconfirmation) break;

                _postconfirmSuperBlockCommitment(superBlockCommitment, msg.sender);
                successfulPostconfirmation = true;
                console.log("[attemptPostconfirmOrRollover] successful postconfirmation at height %s", superBlockHeight);

                // TODO: for rewards we have to run through all the attesters, as we need to acknowledge that they get rewards. 

                // TODO: if the attester is the current postconfirmer, we need to record that the postconfirmer has shown liveness. 
                // TODO: this liveness needs to be discoverable by isWithinPostconfirmerPrivilegeDuration()

                return true;
            }
        }
        // if there was no supermajority for any commitment at that height it means that the attesters were not sufficiently live
        // we rollover the epoch to give the next attesters a chance
        if (!successfulPostconfirmation && getPresentEpoch() > getAcceptingEpoch()) {
            rollOverEpoch();
            console.log("[attemptPostconfirmOrRollover] rolled over to epoch", getAcceptingEpoch());
            return true; // we have to retry the postconfirmation at the next epoch again
        }
        console.log("[attemptPostconfirmOrRollover] no successful postconfirmation");
        return false;
    }

    // ----------------------------------------------------------------
    // -------- 6. Stake, Rewards & Slashing Mechanisms --------------
    // ----------------------------------------------------------------

    /// @notice Gets the stake for a given tuple (custodian, attester) at a given epoch
    function getStake(
        uint256 epoch,
        address custodian,
        address attester
    ) public view returns (uint256) {
        return
            stakingContract.getStake(
                address(this),
                epoch,
                custodian,
                attester
            );
    }

    /// @notice Gets the stake for a given tuple (custodian, attester) at the accepting epoch
    function getStakeForAcceptingEpoch(
        address custodian,
        address attester
    ) public view returns (uint256) {
        return getStake(getAcceptingEpoch(), custodian, attester);
    }

    /// @notice Gets the stake for a given attester at a given epoch
    // TODO: memorize this (<-- ? as in create a mapping?)
    function getAttesterStake(
        uint256 epoch,
        address attester
    ) public view returns (uint256) {
        address[] memory custodians = stakingContract.getRegisteredCustodians(
            address(this)
        );
        uint256 totalStake = 0;
        for (uint256 i = 0; i < custodians.length; i++) {
            // for now, each custodian has weight of 1
            totalStake += getStake(epoch, custodians[i], attester);
        }
        return totalStake;
    }

    /// @notice Gets the stake for a given attester at the accepting epoch
    function getAttesterStakeForAcceptingEpoch(
        address attester
    ) public view returns (uint256) {
        return getAttesterStake(getAcceptingEpoch(), attester);
    }

    /// @notice Gets the stake for a given custodian for a given epoch
    function getCustodianStake(
        uint256 epoch,
        address custodian
    ) public view returns (uint256) {
        return
            stakingContract.getCustodianStake(
                address(this), // domain
                epoch,
                custodian
            );
    }

    function getTotalStake(
        uint256 epoch
    ) public view returns (uint256) {
        // we can either use the attesterStake or the custodianStake
        // the sums of attesterStake and custodianStake should equal the same value
        address[] memory custodians = stakingContract.getRegisteredCustodians(
            address(this)
        );
        uint256 totalStake = 0;
        for (uint256 i = 0; i < custodians.length; i++) {
            // for now, each custodian has weight of 1
            totalStake += getCustodianStake(epoch, custodians[i]);
        }
        return totalStake;
    }

    // gets the total stake for the current epoch for a given custodian
    function getCustodianStakeForAcceptingEpoch(
        address custodian
    ) public view returns (uint256) {
        return getCustodianStake(getAcceptingEpoch(), custodian);
    }

    function getTotalStakeForAcceptingEpoch()
        public
        view
        returns (uint256)
    {
        return getTotalStake(getAcceptingEpoch());
    }

    function setRewardPerAttestationPoint(uint256 rewardPerPoint) public onlyRole(COMMITMENT_ADMIN) {
        rewardPerAttestationPoint = rewardPerPoint;
    }

    function setRewardPerPostconfirmationPoint(uint256 rewardPerPoint) public onlyRole(COMMITMENT_ADMIN) {
        rewardPerPostconfirmationPoint = rewardPerPoint;
    }

    /// @notice Gets the reward points for an attester in a given epoch
    function getAttesterRewardPoints(uint256 epoch, address attester) public view returns (uint256) {
        return attesterRewardPoints[epoch][attester];
    }

        /// @notice Gets the reward points for a postconfirmer in a given epoch
    function getPostconfirmerRewardPoints(uint256 epoch, address postconfirmer) public view returns (uint256) {
        return postconfirmerRewardPoints[epoch][postconfirmer];
    }

    /// @notice Gets the attesters who have stake in the current accepting epoch
    function getStakedAttestersForAcceptingEpoch() public view returns (address[] memory) {
        return stakingContract.getStakedAttestersForAcceptingEpoch(address(this)); 
    }

}
