#!/bin/bash
set -e

# Turn on bash safety options: fail on error, variable unset and error in piped process
set -eou pipefail

# Initialize parameters
debug=false
container_name=movement-full-node
repository=ghcr.io/movementlabsxyz

# Example: add "d" as a boolean toggle
while getopts "dn:r:" opt; do
  case "$opt" in
    d)
      debug=true
      ;;
    n)
      container_name=$OPTARG
      ;;
    r)
      repository=$OPTARG
      ;;
    *)
      echo "Usage: $0 [-d] -n <container-name> -r <repository>"
      exit 1
      ;;
  esac
done

if [ "$debug" = true ]; then
    echo "Debug mode is on"
    echo "CONTAINER_NAME: $container_name"
    echo "REPOSITORY: $repository"
fi

# Get dockerfile path
git_root=$(git rev-parse --show-toplevel)
dockerfile_path=${git_root}/docker/build/${container_name}/Dockerfile

if [ "$debug" = true ]; then
    echo "GIT_ROOT: $git_root"
    echo "DOCKERFILE: $dockerfile_path"
fi

# Get git info
commit_hash=$(git rev-parse HEAD | cut -c1-7)
branch_name=$(git rev-parse --abbrev-ref HEAD)
sanitized_branch_name=${branch_name//\//.}
is_tag=false

if git describe --exact-match --tags HEAD >/dev/null 2>&1; then
    is_tag="true"
fi

if [ "$debug" = true ]; then
    echo "COMMIT_HASH: $commit_hash"
    echo "BRANCH_NAME: $branch_name"
    echo "SANITIZED_BRANCH_NAME: $sanitized_branch_name"
    echo "IS_TAG: $is_tag"
fi

# Get the machine hardware name
arch=$(uname -m)

# Determine the platform name suffix based on the architecture
case "$arch" in
    x86_64)
        platform_shorthand="amd64"
        ;;
    aarch64)
        platform_shorthand="arm64"
        ;;
    arm64)
        platform_shorthand="arm64"
        ;;
    *)
        echo "Unsupported architecture: $arch"
        exit 1
        ;;
esac

if [ "$debug" = true ]; then
    echo "ARCH: $arch"
    echo "PLATFORM_SHORTHAND: $platform_shorthand"
fi

# Get application version
application_version=$(grep -m1 '^version\s*=' Cargo.toml | sed 's/^version\s*=\s*"\(.*\)"/\1/')

if [ "$debug" = true ]; then
    echo "APPLICATION_VERSION: $application_version"
fi

# Generate image tags
container_tags=()
container_tag_commit="${repository}/${container_name}:${commit_hash}-${platform_shorthand}"
container_tag_version="${repository}/${container_name}:${application_version}-${platform_shorthand}"
container_tag_branch="${repository}/${container_name}:${application_version}-${sanitized_branch_name}-${platform_shorthand}"



container_tags+=("$container_tag_commit")
container_tags+=("$container_tag_branch")
if [ "$is_tag" = true ]; then
    # If it's a tag, use the application version
    container_tags+=("$container_tag_version")
fi

if [ "$debug" = true ]; then
    for tag in "${container_tags[@]}"; do
        echo "CONTAINER_TAG: $tag"
    done
fi

docker buildx build \
  --platform "linux/$platform_shorthand" \
  --file "$dockerfile_path" \
  $(for tag in "${container_tags[@]}"; do echo -n " -t $tag "; done) \
  --push \
  "$git_root"