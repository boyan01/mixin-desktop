# Mixin Desktop Working Agreement

## Project truth sources

- Treat this repository as a Rust-backed reimplementation of the source `flutter-app` repository.
- Resolve the destination root with `git rev-parse --show-toplevel` and the source root with `git config --local --get mixinDesktop.flutterAppRoot`.
- If the source root is missing or invalid, ask the user for its checkout path and store it with `git config --local mixinDesktop.flutterAppRoot <path>`. Do not guess or add a hidden path fallback.
- Re-read the current source implementation before porting behavior. Do not rely on an old parity note or memory when the source is available.
- Keep `flutter-app` read-only unless the user explicitly asks to change it. Preserve any dirty files in both repositories.
- For broad parity work, trace from `main` and `runApp` through the real descendant widget tree. Do not audit isolated screenshots or components only.

## Architecture boundary

- Rust owns protocol/API access, Blaze, Signal, sync, jobs, SQLite, durable business state, and reusable business filtering.
- Flutter owns presentation, navigation, view-local state, session-tree composition, lifecycle, and platform integrations such as notification delivery and click handling.
- Keep `AccountRuntime` authoritative for account-scoped business state. Flutter must not maintain a second durable profile, conversation, or message truth source.
- Expose coarse account-scoped handles, DTOs, commands, and event streams through `flutter_rust_bridge`. Do not expose DAOs or internal services to Dart.
- Prefer events over polling. Preserve cancellation, disposal, and request-version guards for long-lived asynchronous work.
- Keep Flutter controllers limited to view state, bounded paging windows, orchestration, and presentation mapping.

## Porting from flutter-app

- Preserve the source flow, widget structure, interaction semantics, lifecycle behavior, and user-visible states before simplifying code.
- Reuse the project's source-parity widgets and theme primitives. Do not replace them with generic Material widgets merely because they are convenient.
- Render useful local/cache data immediately and refresh remote details in the background when the source follows cache-first behavior.
- Adapt source Riverpod/Drift/API dependencies to the current Provider/session-tree and Rust-core boundary; do not copy source DAOs, providers, SDK clients, or workers into Flutter.
- Keep unsupported actions visibly disabled until the Rust operation exists. Do not add mocks, silent no-ops, or hidden fallbacks.
- Accept compatibility exceptions only for known source behavior, such as documented nullable or empty values. Continue rejecting unknown enum or corrupted values.
- Log operation context, exception, and stack trace for failures that can otherwise produce a blank or stale UI.
- When behavior is genuinely unclear, record a concrete unresolved parity question. Do not use a checklist as a substitute for implementation.

## Bridge and generated code

- Stabilize the Rust API shape before regenerating bindings.
- After changing an FRB-visible Rust API, run `flutter_rust_bridge_codegen generate` from `flutter/` and include the generated Dart changes.
- Prefer bridge-safe primitives and DTOs. For example, expose filesystem paths as strings instead of leaking Rust-specific path types.
- Do not add thin wrapper functions that only rename an existing operation.

## Validation

- Run the narrowest relevant tests while iterating, then validate the touched layers before handoff.
- Rust baseline: `cargo fmt --all -- --check` and `cargo test --workspace --all-targets` from the repository root.
- Flutter baseline: `flutter analyze` and relevant `flutter test` targets from `flutter/`; run the full Flutter suite for broad UI/controller changes.
- Run `flutter build macos --debug` for bridge, plugin, native build, or broad parity changes.
- Run a live app/runtime smoke test when the task concerns startup, protocol behavior, real data flow, or visual parity. Do not claim visual equivalence without a visual smoke test.
- Run `git diff --check` and inspect the final diff for generated noise and unrelated changes.
- Distinguish repository failures from local toolchain failures such as a stale CocoaPods shim or unwritable Flutter cache.

## Testing discipline

- Write tests against an observable contract, not an implementation detail. A test name must state the precondition, action, and expected outcome; avoid names such as `test`, `works`, or `renders everything`.
- Each test should prove one behavior or one table of cases with the same invariant. Split unrelated branches and state transitions so a failure identifies the broken contract.
- Every regression fix must include a test that fails for the original bug. Prefer the lowest layer that owns the behavior, then add a higher-layer test only when wiring or user interaction is part of the risk.
- Rust unit tests should cover pure rules, serialization compatibility, state transitions, and error classification. Database tests should execute real SQLite queries and assert persisted rows, ordering, transaction rollback, and emitted events rather than only checking that setup succeeds.
- Rust async tests must use deterministic synchronization such as channels, barriers, paused Tokio time, or bounded timeouts. Do not use arbitrary sleeps to make races pass.
- Do not add `#[ignore]` tests that require personal credentials, hard-coded live accounts, or mutation of a real service. Put intentional live verification in an explicit operator workflow outside the default suite. Ignored tests must still be hermetic, assert a contract, and document why CI cannot run them.
- Tests must not use `println!`, `debugPrint`, screenshots, or successful construction as their only oracle. Assert returned values, durable state, calls with meaningful arguments, emitted events, and important negative behavior.
- Flutter controller tests should drive public methods and assert visible state plus Rust-handle commands. Cover loading, success, failure, retry/supersession, disposal, and stale-result guards where applicable.
- Flutter widget tests should render the smallest production-like tree, including required providers, localization, `Portal`, overlay support, and navigation shell. Interact through taps, text entry, keyboard, focus, or scrolling and assert user-visible results; do not call private state or assert incidental widget nesting.
- Use keys for semantic actions and `find.text(..., findRichText: true)` when production intentionally renders rich text. Do not weaken interaction tests with `warnIfMissed: false`; a missed hit target is a failure.
- Fakes must fail on unexpected calls and record meaningful arguments. Do not return broad default values through permissive mocks that can make an unimplemented path pass. Complete and close streams, completers, timers, temporary files, platform handlers, view sizes, and global overrides in teardown.
- Keep tests hermetic and order-independent. No test may depend on another test's database, filesystem path, clock, global singleton, platform channel, or process state.
- For UI geometry, assert only deliberate product contracts such as breakpoints, hit targets, overflow absence, and stable component sizes. Do not snapshot incidental pixels or duplicate implementation constants without a source-parity reason.
- Before handoff, run the narrow touched tests, then the layer baseline. A failing existing test is not noise: classify it as a product regression, a stale/invalid test, or an environment failure and either fix it in scope or report it explicitly.

## Communication and commits

- Reply in Chinese and lead with a short TL;DR or conclusion; keep code, comments, commit messages, and PR text in English.
- Lead reviews with discrete, actionable findings backed by both source and destination paths.
- Show an ASCII UI sketch when proposing or handing off a UI/UX change.
- Keep changes minimal and logic explicit. Do not add entities, abstraction layers, or fallback behavior without a concrete need.
- Do not add `codex/` or `[codex]` prefixes to branch names, commits, or PR titles.
- Commit only when requested. Before committing, inspect the intended diff; afterwards verify `git status --short` and `git show --stat --oneline HEAD`.
