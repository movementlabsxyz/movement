# MRC
- **RFC**: [RFC MCR](https://github.com/movementlabsxyz/rfcs/pulls)

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
The stake is fixed for an epoch, so only commitments for a specific block height are considered, allowing for a straightforward proof.

Let $C$ represent all possible commitments, and $C'$ be an ordered subset of $C$. MCR returns $c_i \in C'$, the earliest commitment matching the supermajority of stake, defined as:

$$
\delta(C') = \frac{2}{3} \times \sum_{c \in C'} s(v(c)),
$$

where $v: C \to V$ maps a commitment to its validator and $s: V \to \mathbb{N}$ maps a validator to their stake. Define $\sigma'(C', i) = \sum_{j = 0}^{i} s(v(c_j))$, the cumulative stake up to the $i$-th commitment. $\sigma'$ is non-decreasing as $\sigma(C', i) = \sigma(C', i - 1) + s(v(c_i))$.

If $\sigma(C', i) \geq \delta(C')$, then $c_i$ is the earliest commitment where the supermajority is met, since any earlier commitment $c_j$ for $j < i$ would violate the non-decreasing nature of $\sigma'$.
