# Browser runtime

The core and public API support `wasm32-unknown-unknown`. Run one core instance in
one dedicated Worker per origin. The Worker owns SQLite, OPFS file handles,
account tasks and WebSockets. Do not run the core on the window thread or create
an instance per component. A second Worker cannot acquire the same SAH-pool locks.

## Boundaries

- `src/db/storage`: the query operations used by existing DAOs, explicit row
  mappings and bound values. Native uses SQLx; the browser uses the official
  SQLite WASM SAH-pool VFS. SQL schemas, migrations and business rules are shared.
  Transactions own their connection through commit or rollback. A failed commit
  rolls back before reuse. NULL arguments retain their positions; integers cross
  JS as decimal strings and BigInt. The SDK has no database dependency.
- `src/fs`: async acquisition and directory operations, then binary chunk I/O on
  OPFS SyncAccessHandles. SQLite files have a separate VFS namespace; account
  cleanup and database backups use `db/storage/files` after connections close.
- `src/task`, `src/time`, `src/runtime/runner`: platform scheduling and timers.
  Native keeps Tokio; the browser uses its event loop. Account business state
  remains owned by `AccountRuntime`. Shutdown waits for the account loop; dropping
  a task handle detaches and explicit abort cancels.
- `src/core/message/ws`: native handshake headers and protocol keepalive stay in
  the native transport. Browser binary frames use WebSocket callbacks.

`platform.js` is imported by wasm-bindgen and copied into its generated snippets.
The embedding build must resolve `@sqlite.org/sqlite-wasm` and its WASM asset.
Initialize and use the **same copied module instance** when configuring the host
connector. A bundler may expose that module via an alias. Do not import a second
copy of `platform.js`, which would have independent database and connector state.

## Network integration still required

Before starting an account, the embedding app must call
`setBlazeConnector(({endpoint, authorization, protocol}) => socket)` in the Worker.
It must return a newly created, CONNECTING WebSocket using an authentication
mechanism actually supported by the deployment. Browser WebSockets cannot set the
native Authorization header. No authorization query parameter is assumed here.
Missing configuration fails account startup explicitly. Never log the token.

HTTP uses browser Fetch and is subject to CORS. Browser proxy configuration,
background operation after a tab is suspended, native media capture, MCP and LAN
device transfer are not provided by this backend. Native-only facade methods are
excluded from WASM. There is no web UI or production login bridge in this slice.
Real server authentication and end-to-end messaging still need deployment-specific
verification; passing the local contracts does not establish server compatibility.

## Reproduce the non-UI browser checks

Prerequisites: Rust's `wasm32-unknown-unknown` target and `llvm-tools` component,
`protoc`, Node.js, Google Chrome, and `wasm-bindgen-cli` matching Cargo.lock.

```sh
rustup target add wasm32-unknown-unknown
rustup component add llvm-tools
./scripts/test-web-wasm.sh
```

Set `PROTOC` if it is not on PATH. If an earlier macOS build used Apple's archiver,
run `cargo clean -p ring --target wasm32-unknown-unknown` once before retrying.
The generated `pkg/` and `node_modules/` directories are build artifacts.

The harness runs headless Workers against a temporary browser profile. It checks
runtime/settings startup, request signing, real schemas and message persistence,
NULL/blob/full-width integers, transaction exclusivity and rollback (including a
failed commit), Signal group encryption, OPFS attachment encryption/decryption,
file moves, direct OPFS File uploads to a local HTTP endpoint, database backup and
removal, closed handles, timers and task abort.
A fresh Worker then checks persistent messages and files. It uses no real account,
credentials or remote mutations.
