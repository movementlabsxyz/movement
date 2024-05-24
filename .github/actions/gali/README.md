# `gali`
This repository uses an infrastructure management system call `gali`.`gali` is developed internally at Movement Labs.

To create an appropriately formatted PR in Movement Labs' infrastructure repository, comment `gali` with any additional text you want on a PR. You will then be direct to the infrastructure repository deploy you dynamic environment, make updates to infrastructure, etc.

The intent of `gali` is not to immediately trigger infrastructure changes. It instead creates an environment which can be used to simply deploy and edit infrastructure.

## Usage
- Comment `gali` and any additional text you may want on a single line in a PR to create a new environment.
- Comment `gali` and any additional text you may want on a single line in a PR to update an existing environment.
- Follow the provided link to an [`atlantis`](https://www.runatlantis.io/) powered PR in the infrastructure repository to deploy stacks specific to your revisions.