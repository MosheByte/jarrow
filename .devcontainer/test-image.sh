#!/usr/bin/env bash
# Tests a built devcontainer image the way a Codespace uses it: as the "vscode" user, with
# devcontainer.json's containerEnv, it runs every postCreateCommand from devcontainer.json
# against a copy of this checkout, then prints the tool versions.
# .github/workflows/devcontainer.yml runs it before an image is pushed. Locally:
#   docker build -f .devcontainer/Dockerfile -t jarrow-devcontainer:local .
#   bash .devcontainer/test-image.sh jarrow-devcontainer:local
# Needs docker and jq on the host.
set -euo pipefail

image="${1:?usage: bash .devcontainer/test-image.sh IMAGE}"
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
config="$root/.devcontainer/devcontainer.json"

# One command per line: the values of an object-form postCreateCommand, or the string form.
commands="$(jq -r '
  .postCreateCommand
  | if type == "object" then .[]
    elif type == "string" then .
    else error("postCreateCommand must be a string or an object of strings")
    end' "$config")"
if [ -z "$commands" ]; then
  echo "test-image: devcontainer.json has no postCreateCommand" >&2
  exit 1
fi

env_args=()
while IFS= read -r pair; do
  [ -n "$pair" ] && env_args+=(--env "$pair")
done < <(jq -r '.containerEnv // {} | to_entries[] | "\(.key)=\(.value)"' "$config")

docker run --rm \
  --user vscode \
  ${env_args[@]+"${env_args[@]}"} \
  --env "POST_CREATE=$commands" \
  --volume "$root:/src:ro" \
  "$image" \
  bash -c '
    set -euo pipefail
    mkdir "$HOME/jarrow"
    tar -C /src --exclude=./target --exclude=./site/node_modules -cf - . | tar -C "$HOME/jarrow" -xf -
    cd "$HOME/jarrow"
    while IFS= read -r cmd; do
      [ -n "$cmd" ] || continue
      echo "+ $cmd"
      bash -euo pipefail -c "$cmd"
    done <<< "$POST_CREATE"
    echo "+ tool versions"
    rustc --version
    cargo fuzz --version
    python --version
    node --version
    java -version 2>&1
    gh --version
  '
echo "test-image: every postCreateCommand passed in $image"
