# Mixin Messenger Repository Working Agreement

## Scope routing

- These rules apply to the whole repository.
- For work under `swiftui/`, also read `swiftui/AGENTS.md`. It owns the SwiftUI architecture, macOS state management, navigation, UniFFI lifecycle, source organization, parity workflow, and Xcode validation.
- For SwiftUI/macOS feature work that also changes `mixin_desktop_api`, `mixin_desktop_core`, or `mixin-bot-sdk`, apply the relevant bridge and parity rules from `swiftui/AGENTS.md` to those shared changes.
- For work under `flutter/`, also read `flutter/AGENTS.md`. It owns Flutter hooks, Provider state management, FRB lifecycle, widget testing, and Flutter validation.
- For work that affects both UI targets, preserve both bridge contracts and run the validation required by each affected target.

## Shared architecture boundary

- Rust owns protocol and API access, Blaze, Signal, sync, jobs, SQLite, credentials, durable settings, durable business state, and reusable business filtering.
- Each UI target owns presentation, navigation, view-local state, UI-tree composition, lifecycle, and platform integrations such as notification delivery and click handling.
- `DesktopRuntime` owns process-scoped Rust services. `AccountRuntime` remains authoritative for account-scoped business state.
- `mixin_desktop_api` is the platform-neutral public facade over `mixin_desktop_core`. Keep it free of Flutter, SwiftUI, FRB, UniFFI, and platform presentation types.
- UI bridge crates depend on `mixin_desktop_api`. Do not bypass the public facade to expose core DAOs or internal services.
- Expose coarse account-scoped handles, DTOs, commands, and event streams. Prefer screen-ready queries and batched commands over per-row or per-field FFI calls.
- UI targets must not maintain a second durable profile, conversation, message, or settings source of truth.
- Prefer events over polling. Preserve cancellation, disposal, shutdown, lag recovery, and request-version guards for long-lived asynchronous work.
- Keep UI controllers and models limited to presentation state, bounded paging/message windows, orchestration, and presentation mapping.

## Public API, bridges, and generated code

- Stabilize the public Rust API shape before adding bridge annotations or regenerating bindings.
- Use `mixin_desktop_api::ClientResult<T>` and `ClientError` as the shared public error contract. Convert internal errors once at the public boundary.
- Prefer bridge-safe primitives, records, enums, byte arrays, opaque handles, explicit timestamp units, and string filesystem paths.
- Do not expose SQL rows, DAOs, Tokio runtime handles, Rust path types, or internal synchronization primitives.
- Keep bridge-specific error lowering exhaustive and typed. Preserve unauthorized, not found, cancelled, and invalid argument cases.
- After changing an FRB-visible API, run `flutter_rust_bridge_codegen generate` from `flutter/` and include generated Dart changes.
- After changing a UniFFI-visible API, regenerate Swift source, C headers, and the module map with the workflow in `swiftui/README.md`.
- Generated code is committed. Inspect generated diffs for accidental API expansion, stale symbols, naming regressions, and formatting noise.
- Do not add thin wrappers that only rename an existing operation. A wrapper must add conversion, batching, cancellation, lifecycle ownership, error lowering, or a meaningful test seam.

## Testing discipline

- Avoid adding UI-layer tests by default, including widget, snapshot, UI model/controller, and end-to-end UI tests. Add them only when explicitly requested or when a concrete, high-value regression cannot be covered at the Rust/business layer. Existing relevant tests may still be run.
- Do not use Computer Use for UI testing unless explicitly requested. UI smoke tests are not a completion requirement; prefer source review, formatting, static analysis, builds, and relevant existing tests. Report any unverified visual behavior without requesting Computer Use access.

- Write tests against observable contracts. Test names state the precondition, action, and expected outcome.
- Do not add low-value tests that only restate implementation details, verify trivial string assembly or mock wiring, or prove that a stub returns its configured value. A test must be capable of catching a meaningful product regression; when no such automated seam exists, prefer a targeted live validation over a contrived unit test.
- Each test proves one behavior or one table of cases with the same invariant.
- Non-UI regression fixes include a test that fails for the original bug at the lowest layer that owns the behavior. UI-only fixes follow the UI testing preference above.
- Rust unit tests cover pure rules, serialization compatibility, state transitions, and error classification.
- Rust database tests execute real SQLite queries and assert persisted rows, ordering, transaction rollback, and emitted events.
- Rust async tests use deterministic synchronization such as channels, barriers, paused Tokio time, or bounded timeouts. Do not use arbitrary sleeps.
- Do not add ignored tests that require personal credentials, hard-coded live accounts, or mutation of a real service.
- Fakes fail on unexpected calls and record meaningful arguments. Do not use permissive defaults that can make an unimplemented path pass.
- Tests must not use logs, screenshots, or successful construction as their only oracle.
- Keep tests hermetic and order-independent. Close subscriptions, tasks, timers, temporary files, platform handlers, and global overrides in teardown.
- Run a live runtime smoke test when the task concerns startup, protocol behavior, Blaze, Signal, real data flow, notifications, or files. Use non-UI runtime checks; visual interaction does not require an automated UI smoke test.

## Validation

- Run the narrowest relevant tests while iterating, then validate every touched layer before handoff.
- Rust baseline from the repository root:

```sh
cargo fmt --all -- --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
```

- Follow `swiftui/AGENTS.md` for SwiftUI, UniFFI, Xcode, and macOS live validation.
- Follow `flutter/AGENTS.md` for Dart formatting, Flutter analysis, tests, FRB generation, and Flutter builds.
- A shared public API change must compile every affected bridge and UI target even when only one target currently consumes the new method.
- Run `git diff --check` and inspect the final diff for generated noise and unrelated changes.
- Distinguish repository failures from local Xcode, CocoaPods, signing, Flutter cache, and other toolchain failures.

## Communication and commits

- Reply in Chinese and lead with a short conclusion. Keep code, comments, commit messages, PR titles, and technical documentation in English.
- Lead reviews with discrete, actionable findings backed by source and destination paths.
- Show an ASCII UI sketch when proposing or handing off a UI/UX change.
- Keep changes minimal and logic explicit. Do not add entities, abstraction layers, or fallback behavior without a concrete need.
- Do not add `codex/` or `[codex]` prefixes to branch names, commits, or PR titles.
- Commit only when requested. Before committing, inspect the intended diff; afterwards verify `git status --short` and `git show --stat --oneline HEAD`.
