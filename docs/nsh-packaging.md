# Official NSH packaging migration

Status: preparation implemented; release migration remains blocked. Continue using
`python3 nsh/tool/package_macos.py --release` for a usable macOS release. The
candidate below is produced directly by NSH, without a packaging wrapper. It must
not replace the running application.

## Versions and scope

Investigated on 2026-11-13, starting from `b2da8fb` (including `assets/mixin.caf`).
Installed CLI: `nsh 0.5.0 (10582e5)`; full CLI revision
`10582e57cbe67de7cdee07acd7fd1c2cbaac7729`. Project `Cargo.lock` resolves NSH to
`31e107f98348ac65de1b5c1b13db9e7250cd4222`. Generated runners patch NSH to that
project source. The installed binary's source symlink is not authoritative: the
local `.nsh/source` checkout currently points to `643ca53`.

Latest official HEAD checked independently:
`608bf7849228e0b84a5c425123bf81e8c66a4fd6`. Its runner still calls
`run_rust(app_root())`, has no process preflight/after-run hook, and its packager
still copies registered files under `Resources/nsh/assets`. No framework changes
or dependency upgrades were made. Raw official files were fetched at that exact
revision, rather than relying on potentially cached `main` pages.

## Implemented preparation

- `nsh.json` declares the macOS icon, existing six sandbox entitlements, both usage
  descriptions, automatic graphics switching, `mixin` scheme and 1280 by 750
  default window size through supported configuration.
- The shared app id remains `one.mixin.messenger.nsh`, preserving other targets.
  The candidate command supplies `--id one.mixin.messenger.desktop`, as the
  existing macOS packager does. This flag does not replace the root's own app id.
- `Registry::assets` registers emoji and media notices and the existing sound
  without duplicating their source files. The media notice points at its canonical
  media-crate source. Registration only describes files: it starts no runtime,
  opens no account and changes no process environment during CLI describe.
- The release script, notification sound implementation, account startup, native
  component registrations and shutdown safeguards remain intact.

The official command from the repository root is:

```sh
nsh package nsh --target macos --id one.mixin.messenger.desktop -o /tmp/mixin-package-candidate
```

This currently builds an investigational bundle, not an equivalent release.

## Lifecycle mapping and rejected shortcuts

| Existing responsibility | Official entry / timing | Result |
| --- | --- | --- |
| Resolve Flutter-compatible data directory, then export it before threads | No preflight hook in generated main | Still owned by `src/main.rs` |
| Install Mixin logging before opening services | Runner `setup()` installs its logger first; `DesktopClient::open_in` calls Mixin logging initialization | Runtime fails with `attempted to set a logger after the logging system was already initialized`; logger ownership must be explicit |
| Acquire process instance guard; forward arguments/URLs; exit forwarded process successfully | `native_module` describes the app; `native_components` returns `()`; root mounts after window creation | No supported early-return and guard lifetime contract |
| Install SIGTERM/SIGINT handling and retain its abort-on-drop handle | Could install later in native registration, after a window already exists | Feasible only after a safe process owner exists; not a complete migration alone |
| Select initial size | `window.defaultSize`, compiled into root by CLI | Implemented |
| Keep the engine alive when the window closes | `AppOptions.keep_alive_on_close`; runner uses default `false` | Not exposed by runner config; explicit close interception/hide could reproduce behavior with app changes |
| Drain commits and account shutdown after native loop returns (10 seconds maximum) | `on_app_quit` runs before engine disposal, bounded by **200 milliseconds** | Useful for short cleanup, cannot replace existing drain guarantee |
| Native image input, bot and shell components | `#[native_components(macos)]`, called by `AppView::new` | Already connected; retained |
| Notification delegate and permission status | NSH enables its delegate after native registration; Mixin installs its delegate on mount | Keep current ordering; moving Mixin delegate into registration would let NSH overwrite it |
| Named notification sound | `Registry::assets` copies to nested NSH asset directory | File ships, but main bundle resource lookup remains blocked |

Relevant official source:
[generated runner](https://github.com/longbridge/nsh/blob/10582e57cbe67de7cdee07acd7fd1c2cbaac7729/crates/nsh-cli/src/rust_app.rs#L1531),
[matching runner setup](https://github.com/longbridge/nsh/blob/31e107f98348ac65de1b5c1b13db9e7250cd4222/crates/nsh-macos-app/src/lib.rs#L37),
[native registration ordering](https://github.com/longbridge/nsh/blob/31e107f98348ac65de1b5c1b13db9e7250cd4222/crates/nsh-platform-macos/src/app.rs#L349),
[text prewarming thread](https://github.com/longbridge/nsh/blob/31e107f98348ac65de1b5c1b13db9e7250cd4222/crates/nsh-platform-macos/src/surface.rs#L439),
[quit deadline](https://github.com/longbridge/nsh/blob/31e107f98348ac65de1b5c1b13db9e7250cd4222/crates/nsh-core/src/element/quit.rs#L29).

The important distinction is timing and ownership, not merely whether an API can
call an app function. `native_module` runs in the CLI describe executable too;
putting process initialization there would acquire the real instance or start
services while building. `native_components` runs after `Surface::new` has spawned
a CoreText prewarming thread. Moving the current unsafe `set_var` there would
invalidate its single-threaded safety argument. Registration also cannot return a
launch failure or a forwarded-success result to generated main. Calling
`process::exit` inside registration would bypass normal resource destruction.
A component owner is a surface lifetime, not a process lifetime; disposing it is
not a substitute for returning from the native event loop.

There is an app-side alternative to the environment requirement: make all bot,
notification-preview and shell consumers use a resolved directory handle instead
of reading the environment. That touches other feature owners and still does not
supply a clean early-return/after-run contract. It is not enough to justify
removing the existing entry point. The official single-instance option targets
Windows/Linux; macOS LaunchServices reuse does not cover direct executable starts
or the project's directory-scoped forwarding protocol.

## Remaining bundle differences

- The candidate carries registered files under
  `Contents/Resources/nsh/assets/@mixin_desktop_ui/`. Existing release paths are
  `Contents/Resources/Emoji-LICENSE.txt`, `Media-Third-Party-Notices.md`, and
  `mixin.caf`. Notices can be nested if their consumers accept that location, but
  `UNNotificationSound::soundNamed("mixin.caf")` needs an OS-supported sound
  location. Shipping the nested CAF alone does not solve notification audio.
- `schemes` emits URL name and scheme but not `CFBundleTypeRole=Viewer` or
  `LSHandlerRank=Alternate`. `CFBundleURLTypes` is reserved by `macos.infoPlist`,
  so it cannot be replaced there. No plist mutation after packaging was used.
- NSH declares minimum macOS 14. The existing packager derives the deployment
  target from the Mach-O load commands. These declarations must be compared
  against the actual produced binary, not changed speculatively.
- NSH signs the bundle. The release script additionally rejects non-system native
  library dependencies and explicitly verifies the final signature. Those checks
  can be CI verification commands; they do not inherently require a custom packager.

See [official package implementation](https://github.com/longbridge/nsh/blob/10582e57cbe67de7cdee07acd7fd1c2cbaac7729/crates/nsh-cli/src/package.rs#L1378)
and [reserved Apple keys](https://github.com/longbridge/nsh/blob/10582e57cbe67de7cdee07acd7fd1c2cbaac7729/crates/nsh-cli/src/apple.rs#L118).

## Minimal upstream proposal

These are proposed contracts, not existing APIs, and no local fork was created.

1. Add an optional desktop lifecycle export recognized by generated main. Call
   preflight **before** registry creation, logger installation, NSApplication,
   platform surfaces or worker threads. Return either an error, an exit status
   (forwarded launch), or a retained process guard plus AppOptions configuration.
   Keep this export out of describe, preview and headless test discovery. CLI
   capability/version negotiation must reject unsupported required lifecycle
   configuration instead of silently ignoring it.
2. Keep that guard on generated main's stack through `run_rust`; finish and drain
   after the native loop returns, before process exit or restart. Expose runner
   options (including keep-alive and initial size) without duplicating runner
   setup. Provide an explicit logger ownership/installation contract: an app may
   initialize its own logger through preflight (using its public `init_logging`)
   before runner setup, or opt out of runner logging. Current native registration
   runs too late to take ownership. Do not stretch the engine's 200 ms quit deadline into a business drain.
   Preserve the primary guard until shutdown finishes and propagate startup errors.
3. Add validated macOS bundle resource mappings relative to `Contents/Resources`,
   applied before signing. Reject absolute/traversing destinations, symlink escapes
   and collisions with generated resources. Map canonical notice sources and
   `assets/mixin.caf` directly to their current paths. Do not abuse `../` asset URIs.
4. Add role/rank fields to URL declarations (or permit a validated URL-types
   override). Preserve existing scheme validation. Independent signature and
   dependency checks may remain ordinary CI checks after `nsh package`.

A narrower upstream alternative is explicit Cargo binary selection, leaving this
app's current `main` in charge of lifecycle while NSH owns bundle generation.
Lifecycle exports are preferable if upstream wants all apps to use its standard
runner. Either option still requires resource mappings and URL metadata fidelity.
No need to maintain a project packaging wrapper under an official command name.

## Verification

Commands executed from the repository root (all package output is temporary):

```sh
cargo check -p mixin_desktop_ui --lib --locked --offline
cargo clippy -p mixin_desktop_ui --lib --locked --offline -- -D warnings
cargo fmt --all -- --check
rustfmt --check --edition 2024 nsh/src/package_assets.rs
git diff --check
CARGO_NET_OFFLINE=true CARGO_TARGET_DIR=/Users/yangbin/workspace/github/mixin-desktop/nsh/target nsh package /tmp/mixin-nsh-lifecycle-audit/source/nsh --target macos --id one.mixin.messenger.desktop -v -o /tmp/mixin-nsh-lifecycle-audit/out
codesign --verify --strict '/tmp/mixin-nsh-lifecycle-audit/out/Mixin Messenger.app'
codesign -d --entitlements :- '/tmp/mixin-nsh-lifecycle-audit/out/Mixin Messenger.app'
otool -L '/tmp/mixin-nsh-lifecycle-audit/out/Mixin Messenger.app/Contents/MacOS/MixinMessenger'
otool -l '/tmp/mixin-nsh-lifecycle-audit/out/Mixin Messenger.app/Contents/MacOS/MixinMessenger'
```

The temporary source snapshot includes current tracked and untracked sources,
including concurrent copy-fix changes as they existed at snapshot time. Only the
isolated snapshot's id was subsequently changed for runtime testing. Reused the
previous investigation's 12 GB Cargo cache deliberately; did not change the real
workspace's Cargo configuration or lock. An initial online command was stopped
and retried offline after checking that the generated patches resolve to `31e107f`.
The already-started release Cargo child finished building and the offline command
reused its result. This did not upgrade project dependencies.

| Check | Observed result |
| --- | --- |
| UI library check, locked/offline | Passed |
| UI library Clippy, warnings denied, locked/offline | Passed |
| New asset module rustfmt | Passed |
| Workspace rustfmt | Passed on the final recheck; initial transient failure in concurrent `nsh/examples/copy_regression.rs` was left to its owner |
| Diff whitespace | Passed |
| Official release package command | Passed, exit 0; final incremental runner 26.8 seconds, package 763 ms |
| Signature verification | Passed |
| Entitlements | All six match `Mixin.entitlements` exactly |
| App icon | SHA-256 matches existing bundle |
| Emoji notice, media notice, CAF | All present in nested official asset directory; all SHA-256 values match canonical sources |
| Main-bundle CAF lookup | Foundation `Bundle.url(forResource: "mixin", withExtension: "caf")` returns nil |
| Native dependencies | Only `/System/Library/` and `/usr/lib/` dependencies |
| Minimum OS | Info.plist 14.0; Mach-O `minos` 11.0 |
| URL registration | Name/scheme correct; role/rank missing |

Warnings were the unused generated `nsh-bundler` patch and the description
executable's large `__eh_frame` compact-unwind linker warning. These were also
seen in the earlier investigation; no framework patches were introduced.

For the non-UI runtime probe, generated a second bundle with both configuration
and package id `one.mixin.messenger.package-audit-20261113`. Before launch, the
probe asserted that its isolated container Documents directory did not exist.
It passed that new directory through `MIXIN_DESKTOP_DATA_DIR`; NSH storage also
uses the separate root id. `NSH_APP_DIR` pointed at this copy's resource directory
to enable terminal logs and disable login-item integration. This means login-item
and normal Finder launch behavior were not verified. No HOME override was used.

Two direct executable launches (PIDs 24392 and 24399) remained alive concurrently.
Both logged the logger initialization failure above and never opened an account;
only `log/log_0.log` was created in the isolated Documents directory. SIGTERM was
sent only to these two recorded PIDs; both exited with `-15`. This establishes
startup/logger failure, absence of main's process forwarding and absence of its
signal hook. It does **not** establish a working signed-out UI, account shutdown,
notification delivery, URL forwarding or window behavior. No GUI inspection was
performed. Sandbox launch logs also contain an extension warning for the temporary
app location; the explicit business startup failure is separately identifiable.

No authorization request, login, account/message read or send was performed. The
formal app was neither launched, stopped, replaced nor re-signed. Temporary probe
processes have exited. The isolated container is retained as test evidence.

Detailed artifacts are under `/tmp/mixin-nsh-lifecycle-audit/`:
`verification.json`, `runtime.json`, `runtime-0.log`, `runtime-1.log`,
`package-offline.log`, `package-isolated.log`, `check.log`, `clippy.log`, and `fmt.log`. The CLI version,
matching source and latest official source were inspected in addition to the
commands above. Windows and Linux packaging/runtime remain unverified.
