#!/nix/store/lp3ginchcanhcj4dgw6yzdgv8bgdkm1v-bash-5.2p26/bin/bash

# fail if there are modified files
if [[ -n $(git status --porcelain --untracked-files=no) ]]; then
    echo "Failure: there are modified files"
    git status
    git diff
    exit 1
fi
