#!/usr/bin/env bash
# bot_pr.sh: commit generated files on a fresh bot/ branch and open an auto-merging pull request.
#
# Every change to main goes through a pull request, the bot's included. Workflows that
# regenerate files (contributors.yml now, board.yml next) call this script instead of pushing
# to main.
#
# Usage:
#   tools/bot_pr.sh <branch-prefix> <title> <body-file> -- <path>...
#
#   branch-prefix  lower-case letters, digits and dashes, for example "contributors"
#   title          one line; the pull request title and the commit subject
#   body-file      Markdown file; the pull request body and the commit body
#   path...        the files the job regenerated, relative to the repository root
#
# What it does:
#   1. When none of the paths changed, it prints so and exits 0 without a branch or a PR.
#   2. It commits the paths on a new branch bot/<prefix>-<UTC date> (bot/<prefix>-<date>-<time>
#      when that branch already exists) as jarrow-bot <jarrow-bot@noreply.invalid>. The message
#      ends with the trailers "Jarrow-Generated: <prefix>" and
#      "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>".
#   3. It pushes the branch over SSH with the BOT_DEPLOY_KEY deploy key. A push made with a
#      deploy key triggers workflows, so ci.yml runs on the push to bot/** and the head commit
#      gets the required ci-ok check. A push with the workflow token would trigger nothing.
#   4. It opens the pull request with GH_TOKEN (the job's GITHUB_TOKEN), enables auto-merge
#      (squash, pinned to the pushed commit, with the same commit message so the trailers
#      reach main), closes older open bot/<prefix>-* pull requests as superseded, and prints
#      the pull request URL on the last line of its output.
#
# Environment:
#   GH_TOKEN            required; needs pull-requests: write, and contents: write for auto-merge
#   BOT_DEPLOY_KEY      required; the private half of the jarrow-bot deploy key (write access)
#   GITHUB_REPOSITORY   required; OWNER/NAME
#   RUNNER_TEMP         where the key file lives while the script runs (default: a mktemp dir)
#   BOT_PR_BASE         base branch (default: main)
#   BOT_PR_REMOTE       push URL (default: git@github.com:$GITHUB_REPOSITORY.git); the tests
#                       point it at a local bare repository
#
# Secrets: the key is written to a 0600 file under RUNNER_TEMP, reaches git only through
# GIT_SSH_COMMAND, is never echoed, and is deleted by an EXIT trap. GitHub's SSH host keys come
# from "gh api meta" over HTTPS, so the first connection is verified, not trusted on first use.
#
# Exit status: 0 PR opened (or nothing to do), 1 a step failed, 2 usage error.
set -euo pipefail

BOT_NAME="jarrow-bot"
BOT_EMAIL="jarrow-bot@noreply.invalid"
TRAILER="Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"

usage() {
  echo "usage: tools/bot_pr.sh <branch-prefix> <title> <body-file> -- <path>..." >&2
  exit 2
}

fail() {
  echo "::error::bot_pr: $*" >&2
  exit 1
}

[ "$#" -ge 5 ] || usage
prefix=$1
title=$2
body_file=$3
[ "$4" = "--" ] || usage
shift 4
paths=("$@")

if ! [[ "$prefix" =~ ^[a-z0-9][a-z0-9-]{0,39}$ ]]; then
  echo "bot_pr: the branch prefix must be lower-case letters, digits and dashes (at most 40)" >&2
  exit 2
fi
case "$title" in
  "" | *$'\n'* | *$'\r'*)
    echo "bot_pr: the title must be one non-empty line" >&2
    exit 2
    ;;
esac
[ -f "$body_file" ] && [ -r "$body_file" ] || {
  echo "bot_pr: body file not found: $body_file" >&2
  exit 2
}
for p in "${paths[@]}"; do
  case "$p" in
    "" | /* | ../* | */../* | -*)
      echo "bot_pr: paths must be relative to the repository root: $p" >&2
      exit 2
      ;;
  esac
done

body_file=$(cd "$(dirname "$body_file")" && pwd)/$(basename "$body_file")
cd "$(git rev-parse --show-toplevel)"

# 1. Anything to commit?
if [ -z "$(git status --porcelain --untracked-files=all -- "${paths[@]}")" ]; then
  echo "bot_pr: nothing changed in ${paths[*]}; no pull request."
  exit 0
fi

[ -n "${GITHUB_REPOSITORY:-}" ] || fail "GITHUB_REPOSITORY is not set"
[ -n "${GH_TOKEN:-}" ] || fail "GH_TOKEN is not set, so the pull request cannot be opened"
[ -n "${BOT_DEPLOY_KEY:-}" ] || fail "BOT_DEPLOY_KEY is missing, so the branch cannot be pushed"
repo=$GITHUB_REPOSITORY
base=${BOT_PR_BASE:-main}
remote=${BOT_PR_REMOTE:-git@github.com:${repo}.git}

work=$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/bot-pr.XXXXXX")
key="$work/key"
hosts="$work/known_hosts"
cleanup() { rm -rf "$work"; }
trap cleanup EXIT

# 3a. The deploy key, for git only.
(umask 077 && printf '%s\n' "$BOT_DEPLOY_KEY" >"$key")
chmod 600 "$key"
unset BOT_DEPLOY_KEY
gh api meta --jq '.ssh_keys[] | "github.com " + .' >"$hosts" || fail "could not read GitHub's SSH host keys (gh api meta)"
[ -s "$hosts" ] || fail "gh api meta returned no SSH host keys"
export GIT_SSH_COMMAND="ssh -i '$key' -o IdentitiesOnly=yes -o StrictHostKeyChecking=yes -o UserKnownHostsFile='$hosts'"

# 2. The branch and the commit.
day=$(date -u +%Y-%m-%d)
branch="bot/${prefix}-${day}"
if git ls-remote --exit-code --heads "$remote" "refs/heads/$branch" >/dev/null 2>&1; then
  branch="bot/${prefix}-${day}-$(date -u +%H%M%S)"
fi

export GIT_AUTHOR_NAME="$BOT_NAME" GIT_AUTHOR_EMAIL="$BOT_EMAIL"
export GIT_COMMITTER_NAME="$BOT_NAME" GIT_COMMITTER_EMAIL="$BOT_EMAIL"

git switch -q -c "$branch"
for p in "${paths[@]}"; do
  if [ -e "$p" ] || git ls-files --error-unmatch -- "$p" >/dev/null 2>&1; then
    git add -A -- "$p"
  fi
done
if git diff --cached --quiet; then
  echo "bot_pr: nothing to commit after staging ${paths[*]}; no pull request."
  exit 0
fi

# The body exactly once, ending in one newline, then the trailers as the final paragraph.
body=$(cat "$body_file")
commit_body="$work/commit-body.md"
printf '%s\n\nJarrow-Generated: %s\n%s\n' "$body" "$prefix" "$TRAILER" >"$commit_body"
message="$work/commit-message.md"
{
  printf '%s\n\n' "$title"
  cat "$commit_body"
} >"$message"
git commit -q -F "$message"
sha=$(git rev-parse HEAD)

git push -q "$remote" "HEAD:refs/heads/$branch" || fail "could not push $branch with the deploy key"
echo "bot_pr: pushed $branch at ${sha:0:12} as $BOT_NAME"

# 4. The pull request.
pr_body="$work/pr-body.md"
{
  printf '%s\n\n---\n\nOpened by `tools/bot_pr.sh`' "$body"
  if [ -n "${GITHUB_RUN_ID:-}" ]; then
    printf ' in [%s run %s](%s/%s/actions/runs/%s)' "${GITHUB_WORKFLOW:-workflow}" "$GITHUB_RUN_ID" \
      "${GITHUB_SERVER_URL:-https://github.com}" "$repo" "$GITHUB_RUN_ID"
  fi
  printf '. CI ran on the push to `%s`; auto-merge squashes it when `ci-ok` passes.\n' "$branch"
} >"$pr_body"

errors="$work/gh-errors.txt"
if ! url=$(gh pr create --repo "$repo" --base "$base" --head "$branch" --title "$title" --body-file "$pr_body" 2>"$errors"); then
  cat "$errors" >&2
  if grep -qi "not permitted to create or approve pull requests" "$errors"; then
    echo "::error::bot_pr: GitHub Actions may not open pull requests in this repository. The owner turns on Settings > Actions > General > Workflow permissions > \"Allow GitHub Actions to create and approve pull requests\". The branch $branch is pushed; open its pull request by hand meanwhile." >&2
    exit 1
  fi
  fail "gh pr create failed for $branch"
fi
url=$(printf '%s\n' "$url" | tail -n 1)
number=${url##*/}
case "$number" in
  '' | *[!0-9]*) fail "gh pr create did not print a pull request URL" ;;
esac

if ! gh pr merge "$number" --repo "$repo" --auto --squash --match-head-commit "$sha" \
  --subject "$title (#$number)" --body-file "$commit_body" 2>"$errors"; then
  cat "$errors" >&2
  fail "pull request #$number is open, but auto-merge could not be enabled"
fi

# Older bot PRs for the same prefix regenerated the same files from an older main.
older=$(gh pr list --repo "$repo" --state open --limit 100 --json number,headRefName,isCrossRepository,author \
  --jq ".[] | select(.headRefName | startswith(\"bot/${prefix}-\")) | select(.headRefName != \"${branch}\") | select(.isCrossRepository | not) | select(.author.is_bot) | \"\(.number) \(.headRefName)\"" \
  2>"$errors") || {
  cat "$errors" >&2
  echo "::warning::bot_pr: could not list older bot pull requests; close any superseded ones by hand"
  older=""
}
while read -r old_number old_branch; do
  [ -n "${old_number:-}" ] || continue
  if gh pr close "$old_number" --repo "$repo" --comment "Superseded by #$number, regenerated from a newer main." >/dev/null 2>"$errors"; then
    git push -q "$remote" --delete "$old_branch" >/dev/null 2>&1 || true
    echo "bot_pr: closed #$old_number ($old_branch), superseded by #$number"
  else
    cat "$errors" >&2
    echo "::warning::bot_pr: could not close superseded pull request #$old_number"
  fi
done <<<"$older"

if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
  printf 'Opened %s with auto-merge (squash) from `%s`.\n' "$url" "$branch" >>"$GITHUB_STEP_SUMMARY"
fi
echo "$url"
