# TypeScript/Tauri Working Agreement

## Scope

- These rules apply to the `typescript/` React, CSS, Vite, and Tauri client.
- The repository-root `AGENTS.md` continues to apply. Flutter widgets under `../flutter/` remain the source of truth for requested UI and interaction parity.

## Development runtime and hot reload

- Start the complete desktop client from this directory with `npm run tauri dev`. `npm run dev` starts only the browser Vite frontend and does not provide native Tauri APIs.
- When an existing `npm run tauri dev` session is running, do not restart or launch a second app instance for ordinary TypeScript, TSX, or CSS changes. Save the files and wait for Vite HMR to apply the update in the existing window.
- Read the current HMR revision before editing, then use the event-driven wait command after saving. Do not use shell polling loops or fixed sleeps to guess when HMR has settled:

```sh
npm run debug:hmr-revision
# edit TypeScript, TSX, or CSS
npm run debug:wait-hmr -- <previous-revision>
```

- `debug:wait-hmr` uses one event-driven long-poll request to the debug bridge. It returns after the WebView receives `vite:afterUpdate` or finishes a full reload, returns immediately if the revision already advanced, and reports `vite:error`. Only inspect the DOM or capture a screenshot after it succeeds.
- Rust or Tauri configuration changes trigger a native rebuild and app restart through the Tauri watcher. Avoid touching Rust files, running redundant Cargo builds, or restarting the app when a frontend-only change is sufficient.
- Do not use a separate `CARGO_TARGET_DIR` merely to bypass the active dev watcher's build lock. Wait for the watcher to finish and use its rebuilt app unless isolated compilation is concretely required.

## Runtime UI debugging

- Prefer the debug eval bridge for inspecting the live Tauri WebView from the command line:

```sh
npm run debug:eval -- 'document.title'
npm run debug:eval -- '({ width: innerWidth, height: innerHeight })'
```

- Expressions may be async and must return a JSON-serializable value:

```sh
npm run debug:eval -- '(async () => ({ readyState: document.readyState }))()'
```

- Use debug eval to inspect real DOM structure, computed styles, element bounds, scrolling metrics, focus, and runtime state after HMR. Prefer targeted selectors and return only the fields needed for the diagnosis.
- The bridge exists only in debug builds. It binds to `127.0.0.1` on a random port and authenticates with a per-process random token stored in the current user's temporary directory with owner-only permissions. The CLI discovers this metadata automatically.
- Debug eval executes arbitrary JavaScript with the live application's WebView privileges. Default to read-only expressions. Do not send messages, mutate account data, invoke consequential Tauri commands, expose credentials, or return message content or other private data unless the user explicitly authorizes that exact action.
- If the CLI reports missing or stale debug metadata, first confirm whether the existing Tauri watcher is still rebuilding or restarting. Restart `npm run tauri dev` only when the current session is genuinely inactive or the user asks for it.

## Validation

- Run frontend commands from this directory.
- Use `npm run lint`, `npm run typecheck`, and the narrowest relevant checks while iterating. Use `npm run build` when a production frontend bundle is relevant.
- For visual and interaction changes, combine static validation with live inspection of the existing Tauri window after HMR. Static checks alone do not prove Flutter visual parity.
