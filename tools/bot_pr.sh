#!/usr/bin/env bash
# bot_pr.sh: commit generated files on a fresh bot/ branch and open an auto-merging pull request.
#
# Every change to main goes through a pull request, the bot's included. Workflows that
# regenerate files (contributors.yml, and board.yml's ledger) call this script instead of
# pushing to main.
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
#      when that branch already exists). Author and committer are
#      github-actions[bot] <41898282+github-actions[bot]@users.noreply.github.com>, so the
#      commit, and the squash commit on main, count for GitHub Actions as a contributor. The
#      message ends with the trailers "Jarrow-Generated: <prefix>" and
#      "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>".
#   3. It pushes the branch over HTTPS with GH_TOKEN, the job's GITHUB_TOKEN. The token reaches
#      git only through a credential helper set in the environment for this run, scoped to the
#      GitHub server; it is never written to a file, a remote URL or a command line.
#   4. It opens the pull request with GH_TOKEN, enables auto-merge (squash, pinned to the pushed
#      commit, with the same commit message so the trailers reach main), and closes older open
#      bot/<prefix>-* pull requests as superseded.
#   5. A push or a pull request made with GITHUB_TOKEN starts no workflow, so it starts ci.yml
#      and harness.yml on the branch with "gh workflow run", the documented exception. The ci
#      run reports ci-ok on the head commit, which the main ruleset requires; then the merge
#      queue runs its merge_group checks as for any pull request.
#   6. It prints the pull request URL on the last line of its output.
#
# Environment:
#   GH_TOKEN            required; the job's GITHUB_TOKEN with contents: write (push, auto-merge),
#                       pull-requests: write (open, close) and actions: write (workflow_dispatch)
#   GITHUB_REPOSITORY   required; OWNER/NAME
#   GITHUB_SERVER_URL   default https://github.com
#   RUNNER_TEMP         scratch files while the script runs (default: a mktemp dir)
#   BOT_PR_BASE         base branch (default: main)
#   BOT_PR_REMOTE       push URL (default: $GITHUB_SERVER_URL/$GITHUB_REPOSITORY.git); the tests
#                       point it at a local bare repository
#
# The BOT_DEPLOY_KEY deploy key is not used here. It is kept only for a push to a protected
# branch, which GITHUB_TOKEN cannot make (docs/community-automation.md).
#
# Exit status: 0 PR opened (or nothing to do), 1 a step failed, 2 usage error.
set -euo pipefail

BOT_NAME="github-actions[bot]"
BOT_EMAIL="41898282+github-actions[bot]@users.noreply.github.com"
TRAILER="Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
# Workflows started on the bot branch. ci.yml reports the required ci-ok check.
DISPATCH=(ci.yml harness.yml)
# git runs this as `sh -c '<helper> "$@"' <helper> get|store|erase`. It answers "get" with the
# token from the environment, so the token itself never appears in git's configuration.
CREDENTIAL_HELPER='!f() { test "$1" = get || exit 0; printf "username=x-access-token\npassword=%s\n" "$GH_TOKEN"; }; f'

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
[ -n "${GH_TOKEN:-}" ] || fail "GH_TOKEN is not set, so the branch cannot be pushed and the pull request cannot be opened"
repo=$GITHUB_REPOSITORY
base=${BOT_PR_BASE:-main}
server=${GITHUB_SERVER_URL:-https://github.com}
server=${server%/}
remote=${BOT_PR_REMOTE:-${server}/${repo}.git}

work=$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/bot-pr.XXXXXX")
cleanup() { rm -rf "$work"; }
trap cleanup EXIT

# 3a. GH_TOKEN for git, through the environment only: an empty helper first drops any helper
# from other configuration, then ours answers for the GitHub server and nothing else.
export GIT_CONFIG_COUNT=2
export GIT_CONFIG_KEY_0="credential.${server}.helper" GIT_CONFIG_VALUE_0=""
export GIT_CONFIG_KEY_1="credential.${server}.helper" GIT_CONFIG_VALUE_1="$CREDENTIAL_HELPER"
export GIT_TERMINAL_PROMPT=0

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

git push -q "$remote" "HEAD:refs/heads/$branch" || fail "could not push $branch with GH_TOKEN (the job needs contents: write)"
echo "bot_pr: pushed $branch at ${sha:0:12} as $BOT_NAME"

# 4. The pull request.
dispatched=""
for workflow in "${DISPATCH[@]}"; do
  dispatched="${dispatched:+$dispatched and }\`$workflow\`"
done
pr_body="$work/pr-body.md"
{
  printf '%s\n\n---\n\nOpened by `tools/bot_pr.sh`' "$body"
  if [ -n "${GITHUB_RUN_ID:-}" ]; then
    printf ' in [%s run %s](%s/%s/actions/runs/%s)' "${GITHUB_WORKFLOW:-workflow}" "$GITHUB_RUN_ID" \
      "$server" "$repo" "$GITHUB_RUN_ID"
  fi
  printf ' as `%s`. A push made with the workflow token starts no workflow, so the script starts %s on `%s` with workflow_dispatch. Auto-merge puts the pull request in the merge queue when `ci-ok` passes.\n' \
    "$BOT_NAME" "$dispatched" "$branch"
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

# 5. CI on the branch. Without these runs the pull request never gets ci-ok and never merges.
for workflow in "${DISPATCH[@]}"; do
  if ! gh workflow run "$workflow" --repo "$repo" --ref "$branch" 2>"$errors"; then
    cat "$errors" >&2
    fail "pull request #$number is open, but $workflow could not be started on $branch (the job needs actions: write)"
  fi
  echo "bot_pr: started $workflow on $branch"
done

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
  printf 'Opened %s as `%s` with auto-merge (squash) from `%s`, and started %s on it.\n' \
    "$url" "$BOT_NAME" "$branch" "${DISPATCH[*]}" >>"$GITHUB_STEP_SUMMARY"
fi
echo "$url"
