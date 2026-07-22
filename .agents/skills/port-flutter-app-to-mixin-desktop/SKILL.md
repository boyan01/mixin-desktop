---
name: port-flutter-app-to-mixin-desktop
description: Port, implement, audit, or review Mixin Flutter UI and behavior from the source `flutter-app` repository into the Rust-backed `mixin-desktop` application. Use for flutter-app parity work, copying a page or widget flow, filling missing desktop behavior, adapting Riverpod/Drift logic to Provider and `flutter_rust_bridge`, checking source-vs-destination regressions, or deciding whether logic belongs in Flutter or `mixin_desktop_core`.
---

# Port flutter-app to mixin-desktop

Use the current `flutter-app` checkout as the behavioral and visual source of truth while preserving `mixin-desktop`'s Rust-core architecture.

Read [references/repo-map.md](references/repo-map.md) and resolve `DEST_ROOT` and `SOURCE_ROOT` before implementation. Read [references/parity-lessons.md](references/parity-lessons.md) when the task touches shell layout, login, conversation/message UI, cache behavior, notifications, or FRB state ownership.

## Establish the scope

- Inspect `git status --short` under both resolved roots. Treat source changes as user-owned and keep `flutter-app` read-only unless explicitly authorized.
- Re-read the current source files. Treat historical constants and mappings only as search anchors because both repositories evolve.
- For a page or flow, identify its source entrypoint, descendant widgets, providers/controllers, data calls, lifecycle hooks, platform branches, assets, localization, and tests.
- For a broad audit, start at `main` and `runApp`, then traverse the actual widget tree top-down. Maintain a parity checklist only for concrete work or genuinely unresolved questions.

## Classify ownership before copying

Classify each source dependency instead of copying it mechanically:

- Copy or adapt presentation widgets, layout, theme, assets, localization, keyboard behavior, focus, lifecycle, and navigation in Flutter.
- Keep view-local selection, animation, input, paging-window, and route state in Flutter.
- Implement API, database, protocol, Signal, sync, jobs, durable caches, and reusable business filtering in Rust.
- Keep platform delivery and interaction in Flutter/native shell; for notifications, Rust decides reusable eligibility while Flutter owns permission, privacy preview, OS display, click navigation, and dismissal.
- Mount account/session state in a tree-scoped owner. Keep `AccountRuntime` as the single durable business-state source.

Do not copy Drift DAOs, Riverpod data providers, SDK clients, or background workers into the destination Flutter layer. Do not expose Rust DAOs or internal services across FRB.

## Build the destination path

1. Stabilize the Rust behavior and coarse account-scoped API first. Prefer handles, focused DTOs, commands, and streams over chatty calls.
2. Add Rust tests for business rules, persistence, compatibility decoding, and event behavior.
3. Adapt the thin bridge in `flutter/rust/src/api/`. Avoid wrappers that only rename an existing core method and avoid Rust-specific bridge types such as `PathBuf`.
4. Run `flutter_rust_bridge_codegen generate` from `flutter/` immediately after the API shape is stable.
5. Port the source widget tree and flow. Reuse existing destination parity primitives before adding new widgets.
6. Replace source data providers with scoped destination controllers that orchestrate Rust handles and map DTOs into presentation state.
7. Preserve cache-first behavior: paint usable local state immediately, refresh remotely in the background, and keep cached UI visible when refresh fails.
8. Keep unsupported actions visibly disabled. Never invent mock success, silent no-ops, or hidden fallbacks.

## Compare semantics, not filenames

- Compare conditions, ordering, empty/loading/error states, keyboard and focus behavior, lifecycle suppression, platform branches, cache/refresh sequencing, and event propagation.
- Trace every declared UI state to a real transition. A rendered loading/error state is not parity if no controller or Rust event can reach it.
- Distinguish pending or empty results from transport/service failures, and preserve source retry thresholds and escalation behavior.
- Preserve source-specific message rendering rules instead of normalizing them into generic bubbles.
- Preserve cancellation, disposal, and supersession guards when replacing timers or provider listeners with awaited Rust operations or streams.
- Log operation context, exception, and stack trace where a failure could otherwise leave a blank or stale UI.
- Allow only compatibility values explicitly supported by the source. Keep unknown enum values and corrupted records observable as errors.
- When reviewing, report findings first. Cite the source path, destination path, actual behavioral difference, and smallest safe fix.

## Validate in layers

Run narrow checks during implementation, then the relevant final set:

```sh
# repository root
cargo fmt --all -- --check
cargo test --workspace --all-targets
git diff --check

# flutter/
flutter analyze
flutter test
flutter build macos --debug
```

Use targeted tests when the full set is disproportionate, but state exactly what ran and what did not. Run a live app/runtime smoke test for startup, protocol, real-data, or visual-parity claims. Never claim visual parity from static analysis alone.

If a check fails, separate code regressions from environment failures such as Flutter cache permissions, CocoaPods path/shim problems, or missing live fixtures.

## Finish cleanly

- Inspect the final diff for unrelated edits, accidental source-repository changes, stale generated code, and formatting noise.
- Summarize the source behavior preserved, the ownership adaptation, and validation evidence.
- Commit only when requested. After committing, verify both `git status --short` and `git show --stat --oneline HEAD`.
