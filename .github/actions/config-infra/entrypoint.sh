#!/bin/bash
set -e

# Clone and configure the repository
git clone https://github.com/${REPOSITORY}.git repo
cd repo
git config user.name "GitHub Action"
git config user.email "action@github.com"

# Create a new branch
git checkout -b "${NEW_BRANCH}"

# Modify the file
echo "${FILE_CONTENT}" >> "${FILE_PATH}"
git add "${FILE_PATH}"
git commit -m "Update ${FILE_PATH} via GitHub Action"

# Push the new branch
git push origin "${NEW_BRANCH}"

# Create a pull request
gh pr create --base main --head "${NEW_BRANCH}" --title "${PR_TITLE}" --body "${PR_BODY}" --repo "${REPOSITORY}"