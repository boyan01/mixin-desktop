# NSH migration roadmap

Updated 2026-11-07. This is the current handoff for the Flutter-to-Rust/NSH
migration. The user stopped the ongoing implementation run before requesting
this checkpoint commit. This document does not restart that run.

## Scope and evidence

The reference is the Flutter desktop app in
`/Users/yangbin/workspace/mixin/flutter-app`. Preserve its layout, available
operations and return paths. Motion is designed for NSH rather than copied from
Flutter. Keep the shared Rust business layer independent of NSH.

Use these documents for different purposes:

- [Frozen 125-feature inventory](../docs/flutter-nsh-parity.md): original scope,
  not the current implementation status. Its original MISSING/PARTIAL labels
  must not be copied into a new backlog without checking the current source.
- [Migration acceptance ledger](../docs/flutter-nsh-acceptance.md): source,
  business/runtime and platform evidence, including earlier isolated Linux work.
- [Page acceptance ledger](../docs/ui-page-acceptance.md): coordinator-operated
  macOS UI observations, exact regressions and remaining conditional states.
- This roadmap: current priorities and completion criteria. Historical sections
  in the ledgers may describe an earlier state; later integration results take
  precedence.

An implemented page is not automatically an accepted page. Missing test data or
permissions are not automatically missing implementation. Record source/build,
native UI and real-service results separately.

## Current checkpoint

The native Rust application, API facade and platform adapters are in the
workspace. The latest macOS UI pass covered navigation, settings, conversation
menus, search, chat display/composer, attachments, gallery, audio controls and
many profile/ChatInfo interactions. See the page ledger for the tested states.

Recent integrated changes include narrow Settings navigation, hidden-editor
focus handoff, mention-name previews, compact link feedback, Inspector selected
states, ChatInfo biographies and Info-origin Search return paths. Shared-app
previews and their existing Apps/Profile controller now have a direct Info entry.
Nickname editing has a separate `rename_contact` UPDATE command; it no longer
uses Add contact. Native nickname checks exercised drafts and cancellation only;
the wire contract was checked against a local HTTP fixture, without saving a real
user's nickname.

The app was rebuilt and operated on macOS with a mixtest account. That evidence
does not establish full migration acceptance, every viewport or another platform.

Checkpoint validation on 2026-11-07 passed workspace formatting, all-target tests
(427 passed, zero failed or ignored), warnings-denied all-target Clippy and staged
whitespace checks. Native UI evidence comes from the earlier coordinator-operated
page pass; preparing this roadmap did not run new account mutations or UI flows.

## P0 — Finish the main page UI pass

### R01. Establish isolated group and circle fixtures

**Status: authorized, not created.** The user approved independent test groups and
circles using only mixtest t1/t2, retained without automatic deletion. Existing
groups and real contacts are outside this fixture mutation scope.

- Create one `NSH UI acceptance` group with t2 as owner and t1 as its only other
  participant. Verify the actual creator, participant set and owner role.
- The installed mixtest CLI has no create-group command. A temporary helper was
  prepared outside the repository; its manifest remained `planned` when work
  stopped. No group-creation request was executed. Reinspect its source, phase
  and account identities before using it; do not assume a temporary artifact
  remains available or current.
- Create the test circle through NSH from the new group's Info page. Verify that
  the new circle includes the current chat immediately, appears selected and
  filters to the intended group from the sidebar.

Do not relink/release t2, open its live desktop database from a second runtime,
include a third participant or reuse the existing four-member group for writes.

**Done when:** the isolated group and circle exist with checked membership, and
their actual NSH display is recorded in the page ledger.

### R02. Accept group-management and populated-circle pages

**Status: implemented; native coverage incomplete.**

Use R01 to check group Info, owner/member labels, member search and menus, edit
group drafts, invite QR, eligible-member selection, cancellation and return
navigation. Inspect the available actions without changing membership or roles
merely to make a screenshot look complete.

Check populated circle membership, its sidebar entry, rename/edit-chat drafts,
selection counts and cancellation. Create-and-add partial failure retains the
created circle ID in source; its retry and busy/leave states still need suitable
evidence. Physical circle reordering needs more than one circle and remains a
separate conditional check. Do not delete the retained fixtures automatically.

**Done when:** each relevant page has coordinator-observed initial, editing and
cancel/return results; unsupported roles and unexercised mutations remain explicit.

### R03. Close minimum-window and keyboard/IME gaps

**Status: narrow navigation/focus verified; minimum-size coverage incomplete.**

The native minimum is 384 × 480pt. Prior edge-drag and Zoom attempts did not reach
that size reliably. Source inspection found no demonstrated resize-disable or
repeated-frame-restore bug. Do not change the SDK, add a debug geometry interface
or call the viewport accepted just because a maximum size exists in source.

Priority native checks at the actual minimum:

1. New circle chat selection: fixed header/footer, count, scrolling, Back retaining
   the local name/selection, then Cancel without Create.
2. User lookup → profile: bounded width, body scrolling to lower actions, Back
   retaining the query, then Cancel.
3. Direct Info → custom disappearing messages: quantity/unit bounds, Select
   open/cancel and Back without applying a duration.
4. Shared-app preview/list, nickname form, composer and Inspector transitions:
   no clipped controls, hidden input focus or lost draft.

Also check hardware IME composition, pure-keyboard popup activation, long
translations and the affected chat text sizes. A paste batched with Info Back
during its 140ms exit did not append; post-exit focus restoration passed. Establish
normal user-input behavior before deciding whether that transition needs a fix.

**Done when:** actual viewport and input-mode evidence covers these contracts;
ordinary wide-window results are not used as a substitute.

### R04. Finish the remaining ChatInfo entry differences

**Status: functions mostly exist; direct reachability differs from Flutter.**

Shared apps, Search and nickname editing now have direct Info entries. The next
source-confirmed differences are:

- Share contact, groups in common, developer and relationship/report actions are
  reachable through Profile but lack Flutter's direct Info paths. Preserve the
  right user and Back target when reusing those existing surfaces.
- The Flutter Info-avatar Alt action copies a conversation URI. NSH currently has
  no equivalent action there. Prefer an accessible explicit command over a
  modifier-only action if adding the path.
- Review bot-specific conditions/copy rather than treating a bot as an ordinary
  user in every relationship row.

Do not duplicate controllers or introduce a second durable user/profile store.
Group photo/name/announcement are already combined in Edit group; that is not
three missing business capabilities.

**Done when:** remaining direct-entry differences are resolved or individually
documented with an explicit product decision and native return-path evidence.

### R05. Accept conditional content and finish UI copy

**Status: implementations exist; fixtures or presentation checks remain.**

- Existing pins, unread separators/mentions, sent-quote return history and
  system/pin/recall previews need corresponding message states.
- Transcript reader, native video controls/fullscreen, populated sticker albums,
  configured GIF results and larger paged lists need appropriate content.
- Exited-group Info, mute deadlines, nonempty announcements, deactivated/self/
  FRIEND profile variants, live biography changes and shared-app updates are not
  established by the currently accepted ordinary-user states.
- Privacy/lock covers, restore failures, microphone pre-permission/preview,
  notifications, About and transfer pages retain the limits in the page ledger.
  Do not configure security, grant microphone access or start a real transfer as
  an incidental page check.
- Refresh the localization audit after the latest changes. For example, the new
  circle-create progress and partial-failure phrases are not yet in
  `assets/l10n/supplemental.json`. Recheck all supported locales, placeholders,
  expansion and date/time presentation; the earlier copy audit is a dated result.
- Inspect relevant motion and reduced-motion states. Existing Presence/button
  transitions should be retained unless a concrete interaction requires a change.

**Done when:** the main page queue has no unexplained REPAIR/UNVISITED state and
each conditional item has actual evidence or a clearly recorded fixture boundary.

## P1 — Separate runtime and release acceptance

These remain part of the broader migration, but the user explicitly moved the
current work to page UI. Do not restart a general messaging/business-chain test
campaign as a prerequisite for every UI change.

| Area | Remaining evidence |
| --- | --- |
| Media and transfer | Physical microphone capture/send, native video/fullscreen, actual peer device transfer, transcript-specific recovery and the other exact cases still listed in the migration ledger. |
| Notifications and shell | Actual desktop delivery, withdrawal and activation; installed URI activation; clipboard readback; window lifecycle and platform-specific gestures. |
| Security | Physical inactivity/biometric interaction and failure/cancellation presentation. Existing secure-store/API tests do not prove OS prompts. |
| Embedded apps | Real bot interactions and Core-signing requests with the actual bot/document context. Existing native browser protocol/profile-isolation fixtures are not full bot acceptance. |
| Maps | Complete regional provider attribution; a visible Google Maps source link alone does not establish all regional requirements. |
| Distribution | Release signing/notarization, update delivery and installed-package behavior. Local ad hoc signing is not distribution readiness. |

Linux has recorded isolated native build, X11, browser, credential, IPC and package
evidence in the migration ledger. Physical desktop/Wayland and current-page UI
acceptance remain incomplete. Do not erase that evidence or call every Linux
requirement verified. Windows is deferred by the user; its native build, runtime,
WebView2, credentials, activation and packaging still need target-native checks
when work resumes on Windows.

## Deferred scope

- Further image-editor work is paused by the user. Existing implementation and
  pointer-draft observations remain; additional editor polish and coverage are
  not the current priority.
- Importing old Flutter accounts, messages, attachments and settings is a separate
  future task. Do not access personal local data to close this roadmap.
- Mobile-only destructive account/phone/PIN flows, an unlinked legacy backup
  placeholder, and unsupported desktop call/VoIP work are not missing features
  merely because an unreachable reference file exists. See the ledger exceptions.

## Completion and continuation rules

Keep implementations inside their existing feature modules. Use
`mixin_desktop_api` for business work, owner-scoped tasks and events, stable request
guards, semantic controls and the current theme/motion system. The coordinator
owns interfaces, task boundaries, integration and actual acceptance; a sub-agent
report or successful build alone cannot close an item.

Before the next handoff, run the applicable checks from the repository root:

```sh
cargo fmt --all -- --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
git diff --check
```

Compile every affected consumer after public API changes. Record new page/runtime
evidence in the appropriate ledger, then update this roadmap. Do not create a
worktree, commit or push automatically; those actions require the user's request.
