# MCR - L1 contract

This directory contains the implementation of the MRC settlement smart contract. To test the contract, run:

```bash
forge test
```

There is a long-running test covering over 50 epochs. It will likely take a few seconds to run.

# Implementation 
## Description
For a given block height, MCR selects the earliest block commitment that matches the supermajority of stake for a given epoch by:
1. Fixing the stake parameters for the epoch; all stake changes apply to the next epoch.
2. Tracking commitments for each block height until one exceeds the supermajority of stake.

## Proof of Correctness

> To proof: For a given block height, MCR selects the earliest block commitment that matches the supermajority of stake-

The stake is fixed for an epoch, so only commitments for a specific block height are considered, allowing for a straightforward proof.

**Commitment**. Let $v: C \to V$ map a commitment to its validator, where $C$ represent all possible commitments and $V$ is the set of validators. Since commitments are ordered by L1 in the L1-blocks, let $C'$ be an ordered subset of $C$ with $k$ elements (i.e. up to the $k$-th commitment). 

**Stake**. Let $s: V \to \mathbb{N}$ map a validator to their stake and $S(C',i) = \sum_{j = 1}^{i} s(v(c_j))$ the cumulative stake up to the $i$-th commitment. $S$ is non-decreasing as $S(C',i) = S(C',i - 1) + s(v(c_i))$.

We require that 

$$
S(C',i) > \frac{2}{3} TotalStake = \frac{2}{3} \times \sum_{u \in V} s(u),
$$

If $S(C', i)$ satisfies the condition, and $S(C',i-1)$ does not, then $c_i$ is returned by MCR. Due to the non-decreasing nature of $S$ with $i$, $c_i$ is the earliest commitment that can be returned.