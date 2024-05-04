#!/nix/store/lp3ginchcanhcj4dgw6yzdgv8bgdkm1v-bash-5.2p26/bin/bash

# Copyright © Aptos Foundation
# Parts of the project are originally copyright © Meta Platforms, Inc.
# SPDX-License-Identifier: Apache-2.0

set -e

# Check for modified or untracked files after CI has run
diff="$(git diff)"
echo "${diff}"
[[ -z "${diff}" ]]

changed_files="$(git status --porcelain)"
echo "${changed_files}"
[[ -z "${changed_files}" ]]
