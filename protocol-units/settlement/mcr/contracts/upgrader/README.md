# Upgrader

The Upgrader is a script designed to automate the upgrade process for settlement protocol units, streamlining the execution of several key steps required for contract upgrades. Below is a breakdown of its main components and the steps it performs.

## Process Overview

### Deploy

#### 1. Run the `safeDeploy.sh` Script

This bash script is responsible for preparing and executing the deployment of a new contract, using a Safe multisig as the deployer. It accepts a `-c` flag to specify the contract name, a `-u` flag to specify the fork URL, and a `-k` flag to provide an Etherscan API key.

**Steps:**

- Ensures the contract name and URL are provided.
- Runs the deployment script to deploy the implementation contract.
- Generates transaction data and saves it for later use.
- Executes the TypeScript `safeDeploy.ts` script to complete the deployment process using the Safe API.

**Example of Execution:**

```bash
bash safeDeploy.sh -c ExampleContract -u https://example.url -k etherscan_api_key
```

#### 2. Run the propose.sh Script

After deployment, this script is responsible for proposing an upgrade to a proxy contract. It generates the necessary transaction data and proposes the upgrade using the Safe multisig. The script accepts a -c flag for the contract name and a -u flag for the fork URL.

### Steps

Ensures the contract name and URL are provided.
Generates the upgrade transaction data using the deployer script.
Converts the contract name to lowercase (for consistency).
Runs the TypeScript propose.ts script, passing the necessary data to the Safe API for proposing the transaction.
Example of Execution:

```bash
bash propose.sh -c ExampleContract -u https://example.url
```

### Upgrade

#### 1. Run the acceptKms.sh Script

This script is used to accept a transaction using a KMS (Key Management Service). It accepts a -c flag for the contract name, a -t flag for the transaction hash, and a -k flag for the key ID used in KMS.

Steps

- Ensures the contract name, transaction hash, and key ID are provided.
- Converts the contract name to lowercase.
- Runs the TypeScript acceptKms.ts script to accept the transaction using the KMS agent.

Example of Execution:

```bash
bash acceptKms.sh -c ExampleContract -t 0x123... -k key_id
```

#### 2. Run the accept.sh Script

Once the transaction has been proposed, this script is responsible for accepting the upgrade proposal using the Safe API. It accepts a -c flag for the contract name and a -u flag for the fork URL.

Steps

- Ensures the contract name and URL are provided.
- Converts the contract name to lowercase.
- Runs the TypeScript accept.ts script to accept the upgrade proposal using the Safe API.

Example of Execution:

```bash
bash accept.sh -c ExampleContract -u https://example.url
```

## Script Breakdown

Bash Script: safeDeploy.sh
This script handles the deployment of a contract using a Safe multisig as the deployer. It generates transaction data for the deployment and then calls the TypeScript safeDeploy.ts script to finalize the deployment.

Flags:

```bash
-c: Contract name (required)
-u: Fork URL (required)
-k: Etherscan API key (required)
```

Bash Script: propose.sh
This script generates the transaction data required for upgrading a contract and proposes the upgrade using the Safe multisig.

Flags:

```bash
-c: Contract name (required)
-u: Fork URL (required)
Bash Script: acceptKms.sh
This script accepts a transaction using a KMS, handling multisig approval.
```

Flags:

```bash
-c: Contract name (required)
-t: Transaction hash (required)
-k: KMS key ID (required)
```

Bash Script: accept.sh
This script accepts the upgrade proposal through the Safe API.

Flags:

```bash
-c: Contract name (required)
-u: Fork URL (required)
```
