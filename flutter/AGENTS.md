# Flutter Working Agreement

These rules extend the repository-level `AGENTS.md` for work under `flutter/`.

## State management

- Prefer `flutter_hooks` for widget-local state, lifecycle, controllers, focus, animations, subscriptions, and memoization. Use `HookWidget` or `HookBuilder` instead of `StatefulWidget` unless hooks make the lifecycle less clear or a framework API requires a `State` object.
- Call hooks unconditionally and in a stable order. Keep `useEffect` dependencies complete and stable, and always return cleanup for subscriptions, timers, listeners, and controllers created by the effect.
- Use `useState` only for transient presentation state. Derive computed values during build or with `useMemoized`; do not copy Provider or Rust state into a second local cache.
- Use `Provider` for dependency injection and tree-scoped shared state. Prefer `context.select` for narrow rebuilds, `context.watch` when the whole value affects rendering, and `context.read` for event handlers and one-shot commands.
- Create owned controllers with `ChangeNotifierProvider(create: ...)`. Use `.value` only for an existing instance whose lifetime is owned elsewhere. Mount account-scoped providers below the session/account boundary so signing out disposes them together.
- Prefer existing `ChangeNotifier`, `ValueNotifier`, and focused listenable controllers over adding another state-management framework. Do not introduce Riverpod, BLoC, GetX, or global service locators without explicit approval.
- Keep ephemeral state local to the smallest widget subtree. Promote state to a Provider controller only when multiple widgets coordinate through it or its lifetime must outlive a widget instance.

## Architecture and async work

- Keep Flutter responsible for presentation, navigation, view-local state, session-tree composition, lifecycle, and platform integration. Keep protocol/API access, SQLite, sync, jobs, durable business state, and reusable filtering in Rust.
- Treat `AccountRuntime` and its coarse handles as the authoritative account state. Flutter controllers may orchestrate commands, events, bounded paging windows, and presentation mapping, but must not maintain a second durable profile, conversation, or message source of truth.
- Prefer Rust event streams and listenables over polling. Preserve cancellation, disposal, and request-version guards so stale or superseded async results cannot update current UI.
- After every `await`, verify that the hook/widget/controller is still active before mutating state or using `BuildContext`. Log operation context, exception, and stack trace for failures that could leave blank or stale UI.
- Do not add thin wrappers around Provider, hooks, or FRB APIs when they only rename an existing operation.

## Widgets and source parity

- Re-read the current `flutter-app` implementation before porting a page or behavior. Preserve its widget flow, interaction semantics, keyboard/focus behavior, lifecycle, and visible loading, empty, error, and disabled states.
- Reuse the project's theme and source-parity widgets before adding generic Material replacements or new visual primitives.
- Keep `build` methods declarative and side-effect free. Start commands from callbacks or effects, and split widgets when doing so narrows rebuilds or gives a lifecycle owner; do not split only to reduce line count.
- Use `const` where practical, stable keys for identity-sensitive lists, and lazy builders for long or paged collections. Avoid broad Provider watches high in the tree.
- Keep unsupported actions visibly disabled until the Rust operation exists. Do not add mocks, silent no-ops, hidden fallback behavior, or compatibility handling for unknown/corrupted values.

## Testing and validation

- Regression fixes require a test that fails for the original bug. Test observable behavior through public methods and user interaction rather than private widget structure.
- Controller tests must cover the relevant loading, success, failure, retry/supersession, disposal, and stale-result paths. Fakes must reject unexpected calls and record meaningful arguments.
- Widget tests must mount the smallest production-like tree with required providers, localization, `Portal`, overlays, and navigation shell. Interact through taps, text entry, keyboard, focus, or scrolling and assert user-visible results.
- Close streams, timers, completers, platform handlers, and global overrides in teardown. Keep tests hermetic and order-independent.
- Run `dart format` on touched Dart files, then the narrow relevant tests. Before handoff, run `flutter analyze`, the relevant `flutter test` targets, and `git diff --check`; use the full Flutter suite and `flutter build macos --debug` for broad UI, bridge, plugin, or native changes.
- After changing an FRB-visible Rust API, stabilize the API first, run `flutter_rust_bridge_codegen generate` from `flutter/`, and include the generated bindings.
