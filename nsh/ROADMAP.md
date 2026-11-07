# NSH roadmap

Updated 2026-11-07. Feature-level status is in [docs/parity.md](../docs/parity.md).

## Current state

The native Rust/NSH app covers all 125 Flutter desktop feature IDs in source,
backed by `mixin_desktop_api`. On macOS the main pages (navigation, settings,
conversation menus, search, chat display and composer, attachments, gallery, audio,
profile and ChatInfo) have been operated natively with a mixtest account.
Workspace fmt, tests (427) and warnings-denied Clippy passed on 2026-11-07.
Windows and Linux are not accepted: Linux has isolated build/X11/credential/IPC
checks only, and Windows has no native verification.

## P0 — Main-flow gaps

### Minimum-window and input checks

The native minimum is 384 x 480 pt; edge-drag and Zoom have not reliably reached it.
At that size check:

1. New circle chat selection: fixed header/footer, scrolling, Back keeps name and
   selection, Cancel does not create.
2. User lookup to profile: bounded width, body scrolls to lower actions, Back keeps
   the query.
3. Info to custom disappearing messages: bounds, unit select, Back applies nothing.
4. Shared-app preview/list, nickname form, composer and Inspector transitions: no
   clipped controls, lost focus or lost draft.

Also check hardware IME composition, keyboard-only popup activation and long
translations. A paste batched with Info Back during its 140 ms exit did not append;
confirm normal user input before deciding whether that needs a fix.

### ChatInfo direct entries

Shared apps, Search and nickname editing are reachable from Info. Still only
reachable through Profile:

- Share contact, groups in common, developer, relationship and report actions.
  Keep the right user and Back target when reusing those surfaces.
- Copy conversation URI from the Info avatar (Flutter uses an Alt action; prefer an
  explicit command).
- Bot-specific relationship rows and copy.

Do not add a second profile store or duplicate controllers.

### Group and circle fixtures

Group Info, member search/menus, edit-group drafts, invite QR, populated circles,
circle create-and-add retry and circle reordering have no native coverage because
no isolated fixtures exist. Create a test group and circle with mixtest t1/t2 only
(the mixtest CLI cannot create groups; use a separate helper), then check initial,
editing and cancel/return states. Do not touch real groups or contacts.

### Localization refresh

New strings (for example circle-create progress and partial-failure messages) are
missing from `assets/l10n/supplemental.json`. Re-audit all locales for
placeholders, text expansion and date/time formats.

### Conditional content states

Needs matching message/account data: pinned messages, unread separators and
mentions, quote return history, system/pin/recall previews, transcript reader,
video fullscreen, populated sticker albums, GIF results, exited groups, mute
deadlines, announcements, deactivated/self/FRIEND profiles.

## P1 — Polish and platform acceptance

| Area | Remaining |
| --- | --- |
| Windows | Native build, WebView2, credentials, URI activation, tray, packaging. |
| Linux | Physical desktop and Wayland: notification delivery/activation, bot hosts, packages. |
| Media | Physical microphone capture/send, native video controls and fullscreen, peer-device transfer. |
| Notifications and shell | Desktop delivery, withdrawal, click activation; installed `mixin:` URI activation; clipboard readback; window lifecycle. |
| Security | Inactivity auto-lock and biometric prompts, failure/cancel presentation. |
| Embedded apps | Real bot interactions and Core signing with an actual bot/document. |
| Maps | Regional provider attribution beyond the Google Maps source link. |
| Motion | Review reduced-motion states; keep existing Presence/button transitions. |
| Distribution | Developer ID signing, notarization, update delivery, installed-package behavior (local builds use ad hoc signing). |

## Deferred

- Image editor polish beyond the current implementation.
- Calls/VoIP, and mobile-only account/phone/PIN/backup flows (unreachable in the
  Flutter reference).

## Follow-ups from the cleanup pass

- Manually verify native context menus: inside modals (transcript, pinned), menus whose
  row is recycled while open, Windows/Linux.
- Manually verify that the chat keeps its scroll position and draft after Settings, and that
  sticker/GIF/file/voice sends jump to the latest row (source change only, no native check yet).
- Attachment size/count limits; quotes for sticker/GIF/post sends (needs API change).
- IME composition state is not reactive (`rich_editor` has no composition event).
- Verify global search infinite scroll (one section, scroll to the end) natively.
- Refresh the key counts in `nsh/assets/l10n/README.md`.

## Before handoff

```sh
cargo fmt --all -- --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
git diff --check
```

Compile affected consumers after public API changes. Update `docs/parity.md` and this
file when status changes.
