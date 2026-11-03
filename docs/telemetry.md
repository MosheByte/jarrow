# Telemetry

**Status: no telemetry is collected yet.** Nothing converts yet, so there is nothing to measure. This page is the full list of what will be collected once the converter exists (milestone M3), and it is the contract: an event or field that is not listed here is not sent.

## Principles

- **Anonymous.** No account id, no name, no email, no IP address stored, no cookie and no persistent identifier.
- **Mod ids only.** Mod ids are public data. File names, file hashes and JAR bytes are never sent.
- **Opt-out.** The site has a settings toggle that turns telemetry off. When it is off, no event leaves the page. The site also treats the browser's Global Privacy Control signal as "off".
- **The CLI sends nothing.** `jarrow` makes no telemetry calls at all.
- **No mod code, ever.** Tier B jobs never send mod bytes or decompiled source to any analytics or error service.

## Events

### `conversion_started`

Sent when a conversion begins.

| Field | Type | Example | Meaning |
| --- | --- | --- | --- |
| `mod_id` | string | `"examplemod"` | The `id` from `fabric.mod.json` |
| `tier` | `"A"` or `"B"` | `"A"` | The tier the classifier assigned |
| `target_game_version` | string | `"1.21.1"` | One of the pinned versions |
| `engine_version` | string | `"0.1.0"` | Version of the Jarrow engine that ran |

### `conversion_completed`

Sent when a conversion ends, whether it succeeded or not.

| Field | Type | Example | Meaning |
| --- | --- | --- | --- |
| `mod_id` | string | `"examplemod"` | The `id` from `fabric.mod.json` |
| `tier` | `"A"` or `"B"` | `"A"` | The tier the classifier assigned |
| `target_game_version` | string | `"1.21.1"` | One of the pinned versions |
| `engine_version` | string | `"0.1.0"` | Version of the Jarrow engine that ran |
| `duration_ms` | integer | `4210` | Wall time of the conversion in milliseconds |
| `success` | boolean | `true` | Whether the engine produced an output JAR. This is not a harness result |
| `rule_hits` | map of rule id to integer | `{"lifecycle-events-v1/server-tick-end": 2}` | How many times each rule fired |

Unsupported JARs are not converted, so they send no event.

## Where events go

- **PostHog** is connected (`infra/INVENTORY.md`, decision D-006). The events above go to a dedicated PostHog project, created together with the telemetry code in M3, with IP capture turned off and no persistent identifier. Session replay is off. The feature flag `tier_b_enabled` is read from PostHog too; reading a flag sends no personal data.
- **Fallback**, if PostHog stops being available: the same events are written to an `events` table in D1 by a site route.

Raw events in the D1 fallback are kept for 90 days and then reduced to daily counts.

## Errors

Errors from the WASM bundle, the Workers and Tier B jobs go to Sentry if it is connected, or to an `errors` table in D1 otherwise. Stack traces are scrubbed of class names from the user's mod before they are sent, so an error report never reveals a mod's code.

## Changing this list

A new event or field needs a pull request that updates this page first. The site code sends only what this page lists.
