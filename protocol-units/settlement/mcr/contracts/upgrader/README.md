# Upgrader

The Upgrader is a script designed to automate the upgrade process for settlement protocol units, streamlining the execution of several key steps required for contract upgrades. Below is a breakdown of its main components and the steps it performs.

## Process Overview

1. **Run the `upgrade.sh` script**:  
   This bash script is responsible for preparing and executing the upgrade for the specified protocol unit contract. It accepts a `-c` flag to specify the contract name, ensures the contract name is provided, generates transaction data, and proceeds with executing the upgrade process.

2. **Deploys Implmentation and Generates Transaction Data**:
   As part of the upgrade process, it dynamically runs a `forge` command, using the provided contract name to locate the correct deployer script (e.g., `MCRDeployer.s.sol`). The script deploys an implementation contract for the proxy to point out to and generates the transaction data necessary by storing it in `script/helpers/upgrade/{contract}.json`, dta that will be served to upgrade the contract through the Safe Multisig.

3. **Run the TypeScript ProposeTransaction Script**:  
   After generating the transaction data, the bash script runs the TypeScript `upgrader/index.ts` script. It takes the `script/helpers/upgrade/{contract}.json` data and enforces it to be used through the Safe API by calling ProposeTransaction. It takes the `PRIVATE_KEY` of one participant of the multisig, the KMS agent.

4. **Propose Transaction to the Safe API**:  
   Finally, the transaction is up for a vote by Movement Labs signers who are able to accept or cancel the upgrade proposal transaction. This does not immediately upgrade the contracts since a timelock enforces a 2 days waiting time for any upgrade to be executed.

## Script Breakdown

### Bash Script (`upgrade.sh`)

The following steps are executed in the `upgrade.sh` script:

1. Parse the provided contract name using the `-c` flag.
2. Ensure that the contract name is supplied; otherwise, exit with an error.
3. Generate the transaction data needed for upgrading the contract using `forge script`, pointing to the appropriate deployer file for the contract.
4. Convert the contract name to lowercase (required for consistency).
5. Execute the TypeScript upgrader script (`index.ts`), passing the lowercase contract name.
6. The upgrader script sends the `proposeTransaction` request to the Safe API.

### Example of Bash Script Execution

```bash
./upgrade.sh -c ExampleContract
