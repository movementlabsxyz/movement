#!/bin/bash
set -e

echo "$TARGET_REPO"

GALI_FILE_PATH=gali/${SOURCE_REPO}

ESCAPED_SOURCE_REPO="${SOURCE_REPO//\//_}"

# Clone and configure the repository
git clone https://x-access-token:$GITHUB_TOKEN@github.com/${TARGET_REPO} repo
cd repo
mkdir -p $(dirname "${GALI_FILE_PATH}")
git config user.name "GitHub Action"
git config user.email "action@github.com"


echo "Branch does not exist, creating new branch"
git checkout main
git checkout -b "${GALI_ID}"
git pull origin "${GALI_ID}" || true # ignore failure if branch does not exist
echo "::set-output name=branch_message::$(echo 'Branch does not exist, creating new branch')"


cat <<EOF > "${GALI_FILE_PATH}"
# non-namespaced values, never use these when loaded as an environment variable
GALI_ID=${GALI_ID}
GALI_SHA=${GALI_SHA}
GALI_SOURCE_REPO=${SOURCE_REPO}

# namespaced values, use these when loaded as an environment variable
${ESCAPED_SOURCE_REPO}_GALI_ID=${GALI_ID}
${ESCAPED_SOURCE_REPO}_GALI_SHA=${GALI_SHA}
${ESCAPED_SOURCE_REPO}_GALI_SOURCE_REPO=${GALI_SOURCE_REPO}
EOF

# Modify the file
git add "${GALI_FILE_PATH}"
git commit -m "gali: update ${GALI_ID} via GitHub Action"

# Push the new branch
git push https://x-access-token:$GITHUB_TOKEN@github.com/${TARGET_REPO} "${GALI_ID}"

# Create a pull request
gh pr create --base main --head "${GALI_ID}" --title "${GALI_ID}" --body "${COMMENT}" --repo "${TARGET_REPO}"

# set link to pr
echo "::set-output name=pr_link::$(gh pr view -w --json number --repo ${TARGET_REPO})"

EXISTING_PR_URL=$(gh pr list --base "main" --head "$GALI_ID" --repo "$TARGET_REPO" --json url --jq '.[0].url')

echo "EXISTING_PR_URL: $EXISTING_PR_URL"
if [ -z "$EXISTING_PR_URL" ]; then
    # No existing PR, create a new one
    gh pr create --base "main" --head "$GALI_ID" --title "$GALI_ID" --body "$COMMENT" --repo "$TARGET_REPO"
    # Fetch the URL of the newly created PR
    EXISTING_PR_URL=$(gh pr list --base "main" --head "$GALI_ID" --repo "$TARGET_REPO" --json url --jq '.[0].url')
fi
echo "EXISTING_PR_URL: $EXISTING_PR_URL"
echo "::set-output name=pr_link::$EXISTING_PR_URL"
