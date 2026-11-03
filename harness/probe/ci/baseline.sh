#!/usr/bin/env bash
# Runs the harness baseline for each game version, or records why it did not run.
# Called by the "baseline" job of .github/workflows/harness.yml; docs/probes.md describes it.
#
#   baseline.sh --harness <path to jarrow-harness, or empty> --probes <dir> --out <dir> \
#               --versions "<game version> ..."
#
# <probes>/probes-<game version>/ holds that version's probe jars (one artifact per version).
# For each version it writes <out>/<game version>/baseline-status.json:
#   {"schema": 1, "game_version": "...", "status": "ran" | "not-run" | "failed",
#    "reason": "...", "detail": "...", "exit_code": <n or null>}
# and appends a Markdown table to $GITHUB_STEP_SUMMARY (or prints it when that is unset).
#
# Exit status: 0 when every version ran or was not run for a recorded reason; 1 when the harness
# ran and failed for any version; 2 on a usage error.
#
# The EULA gate: the harness may start a Minecraft server only when MINECRAFT_EULA_ACCEPTED (or
# MINECRAFT_EULA) is true, yes, 1 or accepted, ignoring case (docs/harness.md). This script
# checks the same rule so it never asks the harness to start a server with the gate closed. It
# never prints the variable's value.
set -euo pipefail

harness=""
probes=""
out=""
versions=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --harness) harness="${2-}"; shift 2 ;;
    --probes) probes="${2-}"; shift 2 ;;
    --out) out="${2-}"; shift 2 ;;
    --versions) versions="${2-}"; shift 2 ;;
    *) echo "baseline.sh: unknown argument: $1" >&2; exit 2 ;;
  esac
done
if [ -z "$probes" ] || [ -z "$out" ] || [ -z "$versions" ]; then
  echo "usage: baseline.sh --harness <bin or empty> --probes <dir> --out <dir> --versions \"<v> ...\"" >&2
  exit 2
fi

gate_open() {
  local value
  for value in "${MINECRAFT_EULA_ACCEPTED-}" "${MINECRAFT_EULA-}"; do
    case "$(printf '%s' "$value" | tr '[:upper:]' '[:lower:]')" in
      true | yes | 1 | accepted) return 0 ;;
    esac
  done
  return 1
}

if gate_open; then gate="open"; else gate="closed"; fi

# Does `jarrow-harness baseline --help` exist, and does it take the flags this script passes?
interface=""
help_text=""
if [ -z "$harness" ]; then
  interface="harness_not_built"
elif [ ! -x "$harness" ]; then
  interface="harness_not_built"
elif ! help_text=$("$harness" baseline --help 2>&1); then
  interface="no_baseline_command"
else
  for flag in --game-version --out; do
    if ! printf '%s\n' "$help_text" | grep -Eq -- "${flag}([^a-z-]|\$)"; then
      interface="harness_interface_mismatch"
    fi
  done
fi
takes_probe_dir=false
if [ -n "$help_text" ] && printf '%s\n' "$help_text" | grep -Eq -- '--probe-dir([^a-z-]|$)'; then
  takes_probe_dir=true
fi

write_status() {
  # write_status <dir> <version> <status> <reason> <detail> <exit code or null>
  mkdir -p "$1"
  printf '{\n  "detail": "%s",\n  "exit_code": %s,\n  "game_version": "%s",\n  "reason": "%s",\n  "schema": 1,\n  "status": "%s"\n}\n' \
    "$5" "$6" "$2" "$4" "$3" > "$1/baseline-status.json"
}

summary="${GITHUB_STEP_SUMMARY:-/dev/stdout}"
{
  echo "### Harness baseline"
  echo ""
  echo "- EULA gate: **${gate}**"
  if [ -z "$harness" ]; then
    echo "- jarrow-harness: not built (crates/harness not on main yet)"
  else
    echo "- jarrow-harness: \`$harness\`"
  fi
  echo ""
  echo "| Game version | Status | Reason |"
  echo "| --- | --- | --- |"
} >> "$summary"

failed=0
for version in $versions; do
  dir="$out/$version"
  probe_dir="$probes/probes-$version"
  status="not-run"
  reason=""
  detail=""
  code="null"
  if [ -n "$interface" ]; then
    reason="$interface"
    case "$interface" in
      harness_not_built) detail="crates/harness not on main yet" ;;
      no_baseline_command) detail="jarrow-harness has no baseline command yet" ;;
      harness_interface_mismatch)
        detail="jarrow-harness baseline --help does not list --game-version and --out"
        echo "::warning title=harness baseline::jarrow-harness baseline does not take --game-version and --out; baseline not run" ;;
    esac
  elif [ "$gate" != "open" ]; then
    reason="eula_not_accepted"
    detail="not run: EULA not accepted (MINECRAFT_EULA_ACCEPTED is not true, yes, 1 or accepted)"
  elif ! compgen -G "$probe_dir/*.jar" > /dev/null; then
    status="failed"
    reason="probes_missing"
    detail="no probe jars in $probe_dir"
    failed=1
  else
    args=(baseline --game-version "$version" --out "$dir")
    if [ "$takes_probe_dir" = true ]; then
      args+=(--probe-dir "$probe_dir")
    fi
    mkdir -p "$dir"
    echo "Running: $harness ${args[*]}"
    set +e
    JARROW_PROBE_DIR="$probe_dir" "$harness" "${args[@]}"
    code=$?
    set -e
    if [ "$code" -eq 0 ]; then
      status="ran"
      reason="ok"
      detail="dumps and noise report in $dir"
    else
      status="failed"
      reason="harness_failed"
      detail="jarrow-harness baseline exited with $code"
      failed=1
      echo "::error title=harness baseline::jarrow-harness baseline failed for $version (exit $code)"
    fi
  fi
  write_status "$dir" "$version" "$status" "$reason" "$detail" "$code"
  echo "| $version | $status | $detail |" >> "$summary"
  echo "$version: $status ($reason)"
done

if [ "$gate" != "open" ] && [ -z "$interface" ]; then
  {
    echo ""
    echo "The baseline was not run because the owner's Minecraft EULA acceptance is not set. This is not a failure of the workflow, and nothing is green without a harness result (docs/harness.md, \"The EULA gate\")."
  } >> "$summary"
fi
exit "$failed"
