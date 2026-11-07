# Current migration acceptance queue

Updated 2026-11-07. The [frozen parity inventory](flutter-nsh-parity.md) remains
the reference for the complete 125-ID scope. This file records current source
disposition and outstanding acceptance work. It supersedes baseline MISSING/PARTIAL
labels for implementation tracking; it does not rewrite the original audit.

The [NSH roadmap](../nsh/ROADMAP.md) consolidates the latest unfinished work,
user-directed priorities and deferred scope after the page UI pass. Use it as the
continuation entry point; the historical evidence below is not a new execution order.

`N:`, `A:`, `C:` and `M:` refer to this repository's NSH, API, Core and Media source
roots. Source review alone does not establish native interaction or runtime success.
No row here declares acceptance on every desktop platform. The coordinator owns
independent integration, review and final acceptance; sub-agent completion reports
are inputs, not acceptance oracles.

## Current user-directed UI pass

On 2026-11-07 the user deferred Windows work and directed the coordinator to focus
on one chat page's layout, display and interactions before broader workflows.
Windows limitations remain documented, but do not block this UI pass. The user
explicitly requested actual functional interaction and then narrowed acceptance to
the page UI; no new send/receive or business-layer acceptance is being performed.
The coordinator operates the native macOS app; separate fresh agents implement
bounded UI fixes. Source-ready rows below do not mean that the page has passed UI
acceptance.

The native page initially failed acceptance: the history viewport collapsed and
the composer appeared immediately below the header despite existing messages.
Adding stretch and shrink constraints to the actual `OwnedView` renderer wrapper
made the three existing group messages visible and anchored the composer at the
bottom. The coordinator observed the rebuilt app, not just the implementation.

| Page check | Coordinator observation |
| --- | --- |
| Existing history and bottom composer | Passed after the first rebuilt UI fix; the previous viewport-collapse failure was reproduced beforehand. |
| Message menu and reply preview | Menu opens; reply preview expands above the composer and Cancel removes it. No message was sent. |
| Attachment menu | Opens above the bottom trigger; Escape dismisses it. A previously open menu in a screenshot was not evidence that it was stuck. |
| Emoji surface | Tabs display; an emoji inserts into the draft; Escape dismisses the surface. The temporary draft was cleared without sending. |
| Chat info and search panels | Open in the side region and return to the chat; the composer remains at the bottom. |
| Native window zoom | Enlarging and restoring the window retained visible messages and the bottom composer. This does not prove every compact breakpoint. |
| Multiline composer | Final rebuilt-page verification passed: three lines grow naturally; ten pasted lines reach the height cap with the last line and caret visible without an extra keypress. Manual scroll reaches the beginning without snapping back; subsequent typing reveals the caret. Undo removes that typing; replacing with three lines shrinks the viewport and clearing returns to one line and the microphone action. All preview text was cleared without sending. |
| Profile-to-chat dismissal | Rebuilt-page verification passed: Message selects the intended chat and removes the entire lookup/profile overlay. The original obstructing-overlay failure was reproduced beforehand. |
| Profile content bounds | Actual scrolling reaches the lower commands with the header fixed and a visible scrollbar. Content beyond the initial viewport was normal scrolling, not a layout defect; no code change or destructive command was needed. |
| Sidebar presentation | Actual collapse and expansion retain visible conversation/history/composer regions. Icons settle into the compact rail after its transition; the original expanded state was restored. |

No page-wide or application-wide UI completion is claimed by these observations.

The multiline implementation was rejected in native interaction until direct paste
passed. NSH 0.4.0's private pending-scroll flag does not itself invalidate its
selection/layout effect; delaying an empty scroll transaction did not solve the
observed failure. The final input follows this repository's existing reader's
intrinsic editor layout and gives the application-owned bounded Scroll the only
vertical range. It uses actual caret geometry and the public scroll ref, without
altering selection, focus or document history. Temporary diagnostics, duplicate
measurement text and the ineffective empty transactions were removed. A separate
read-only agent reviewed these source boundaries; the coordinator independently
performed the native interactions above. Hardware IME, all compact window sizes,
all themes and the full application remain outside this page pass.

The final page-only integration passed `cargo fmt --all -- --check`,
`cargo clippy -p mixin_desktop_ui --all-targets -- -D warnings`, the existing 23
UI-crate library tests and touched-file whitespace checks. The native Debug bundle
was rebuilt and its signature verified; it remains open on the signed-in t2 chat
with an empty composer. These checks supplement the native interactions above;
they do not replace them. Core/API/Media and the frozen 125-ID scope were not
changed by this UI pass.

- **SOURCE-READY**: the reachable source has been reviewed; remaining native/visual
  checks are stated in the evidence. This is not an all-platform completion label.
- **PARTIAL-PLATFORMS**: macOS implementation exists; concrete Windows/Linux native
  adapters or packaging still need implementation and native validation.
- **PLATFORM-VALIDATION-PENDING**: target-specific adapter source exists, but its
  native system/runtime behavior has not been checked.
- **FUNCTIONAL-GAP**: a source-backed workflow difference remains.
- **LIVE-PENDING**: implementation exists but the inventory requires an actual
  end-to-end runtime operation that has not yet been exercised.
- **DEFERRED-BY-USER** and **REFERENCE-EXCEPTION** are scoped out explicitly.

## Coordinator evidence for the current integration

The coordinator inspected the closing implementation sources: shared proxy injection
and WebSocket tunnels, transfer token/request state and subscription disposal, bounded
conversation navigation, transcript export/audio/visibility, recording session and
telemetry, notification redaction/actor/mention mapping, profile/deleted-user handling,
member eligibility paging, sticker metadata recovery/drag ordering and location tiles.

- `cargo test --workspace --all-targets`: **425 passed, zero failed or ignored**.
  This includes all SDK/Core/API/Media/NSH tests and affected consumers.
  Log: `/tmp/mixin-nsh-closing-tests.log`.
- Closing integration passed formatting and warnings-denied all-target Clippy on
  macOS. Final native Ubuntu ARM checks passed **421 tests, zero failed or ignored**,
  formatting and full-workspace Clippy. The four-test difference is Media
  platform-specific coverage. Logs: `/tmp/mixin-nsh-linux-native-final-tests.log`
  and `/tmp/mixin-nsh-linux-native-final-clippy.log`. Windows remains separate.
- Actual isolated mixtest restore/profile/Blaze/message-window checks and bounded
  shutdown passed. The Signal void-response regression reproduced before the SDK
  fix; the first fixed real startup registered prekeys successfully.
- Six persisted ordinary/transcript attachment exports were independently repeated
  against original fixture bytes. PNG clipboard payload pixels matched; temporary
  export directories were removed. Actual OS clipboard readback remains unverified.
- The actual language/catalog modules resolve **652 source pairs, 637 English keys,
  617 static pairs and 34 named templates in nine locales**. The original nine ARBs
  remain unchanged. Translation quality and expanded visual layout are unverified.

The coordinator's native existing-chat URI check first failed with
`Home provides recall editing`. NSH virtual cells inherit the nearest actual
component's context, while the original Home/Chat were plain helpers inside dynamic
bindings. `N:ui/owned.rs` now gives Home, Chat, pinned and transcript readers actual
component owners, retaining their feature context and disposal together. The same
packaged-app check subsequently passed saved restore, live Blaze, existing-chat URI,
second-launch handoff and bounded SIGTERM without presentation panic. This proves
the observed native startup regression was fixed, not visual gesture/layout parity.
The debug linker reported a compact-unwind size warning; packaging/signature checks
still succeeded.

The latest integration, including all ten closing fixes and the access-file split,
was rebuilt, packaged and ad hoc verified in both Debug and Release on 2026-11-07. Both packages independently passed native isolated restore, live
Blaze, existing-chat URI, second-launch handoff and bounded SIGTERM shutdown. Logs:
`/tmp/mixin-nsh-closing-native-debug-runtime.log` and
`/tmp/mixin-nsh-closing-native-release-runtime.log`. The optimized Release
link completed without the Debug compact-unwind warning. Coordinator checks
independently typechecked the exact Windows bot,
shell, notification and IPC sources against their real Windows APIs. GTK shell and
WebKit adapter metadata checks use real GTK/WebKit Rust APIs but shim only NSH's
native registration interfaces. Full Windows/Linux SDK builds did not pass on the
Mac host: Windows lacks the MSVC C headers; Linux lacks the cross GTK/pkg-config
sysroot. These are toolchain limitations, not passing target builds.

A fresh independent motion source review identified compact inspector layout and
exit-state disagreement. The coordinator made its wrapper fill the available width
and kept the chat hidden until the retained panel finishes leaving. Commands stop
immediately on close; presentation no longer gives two compact panes the same row.
Visual timing and physical focus behavior remain unverified.

The coordinator also reviewed the actual asynchronous NSH native-event contract.
Windows/Linux quit now hands the final bounds back to the bounded settings writer
before returning the native loop, with an independent two-second exit deadline if
a prior write stalls. Accepted writes remain tracked by the existing bounded
runtime shutdown. A missing Windows tray falls back to minimizing for Hide; system
close uses normal shutdown instead of leaving an unreachable hidden window.

Live message mutations are confined to the user-authorized marked mixtest fixtures.
Existing Flutter local-data import is a separate task. No worktree, commit, push or
personal data migration was performed. Earlier protocol checks did not use Computer
Use; the current user-directed UI pass above uses actual native interaction.

The coordinator also executed a 64 MiB actual-source codec probe over real
localhost TCP. Input creation, encryption/decryption and SHA-256 verification use
fixed chunks; the received byte count was 67,108,864 and staging was empty. The
standalone codec process peaked at 4,030,464 bytes RSS (about 3.84 MiB) according to
macOS `/usr/bin/time -l`. This is not a measurement of the complete desktop app or
a production throughput guarantee. Evidence:
`/tmp/mixin-nsh-coordinator-transfer-64m-evidence.json` and
`/tmp/mixin-nsh-coordinator-transfer-64m.log`.

## Closing source-audit fixes

A fresh independent source audit identified ten concrete parity or lifecycle
defects. The coordinator reviewed their owning sources after implementation:
password ZIP preparation, retained preview additions, multi-selection copy,
caption selection, completed-image recovery, stale-search activation guards,
self-profile command filtering, retryable sign-out preflight, REST-authentication
retirement and explicit Windows OS dependency classification. Private feature
modules own these changes; public handles and account authority remain shared.
The five independent access handles now live in private feature files under
`A:access/`, while their existing entry paths, signatures and execution bodies
remain unchanged. All affected NSH/headless/example consumers are recompiled.

The latest complete workspace run passed 425 tests, with zero failures or ignored
tests, plus formatting and all-target Clippy with warnings denied. Lifecycle tests
execute real temporary SQLite, local SDK HTTP 401 and real MCP listeners. They
check failed-preflight retry, credential rollback, stale-session guards, terminal
health delivery, listener release and replacement-server ownership.

The coordinator also independently executed the existing authorized ordinary PNG
fixture through the public API: missing and corrupt DONE/READ files redownloaded
with the original SHA-256; a valid file remained unchanged. Backup removal and
bounded shutdown passed, with no SQL repair or new messages. Evidence:
`/tmp/mixin-nsh-photo-recovery-runtime-evidence.json` and
`/tmp/mixin-nsh-photo-recovery-coordinator-runtime.log`. This is not a transcript
recovery or gallery-gesture runtime result.

Actual Media AES archives retained the original entry names and exact bytes.
Rust and Flutter's resolved Dart archive 4.5.2 read each other's ASCII-password
archives; independent native bsdtar read Rust ASCII/Unicode and Dart ASCII
archives. The old Dart library truncates UTF-16 code units for Unicode passwords;
Rust uses standard UTF-8. A separate explicit byte adapter verified Dart's AES
reader against Rust UTF-8 output, but the legacy normal Unicode-password path is
not claimed compatible. Log: `/tmp/mixin-nsh-archive-interop-runtime.log`.

The Windows dependency regression used a real x64 PE fixture and an isolated
System32 collision, verified copied bytes and distinguished VC redistributables
from reviewed OS DLLs. It reproduced the old omission and passed the new policy.
It does not establish a native Windows package build. Log:
`/tmp/mixin-nsh-package-dependency-regression.log`.

## Native macOS browser protocol evidence

The coordinator independently reran the external actual-WebKit fixture after
reviewing its source and isolated runner. Only NSH's component registration/context
interfaces are shimmed; WKWebView, WKFrameInfo, data stores and callbacks are real.
Local dual-port pages prove main-frame provenance, rejection of same/cross-origin
iframe privileged messages, current replies, old-navigation replies, account cookie
isolation, actual purge completion and cookie absence, and native content detach.

The first raw bridge stress check reproduced **34 native signature events where
only 32 were permitted**. The Mac adapter now owns its 32-slot pending set, rejects
duplicate IDs, verifies native frame URL against the current web view, clears on
navigation/close and releases completed old-SPA slots. The coordinator rerun also
proved that a stale generation cannot consume a reused current request ID.

Log: `/tmp/mixin-nsh-coordinator-mac-bot-runtime.log`; structured result:
`/tmp/mixin-nsh-mac-bot-runtime-evidence.json`. All created synthetic store IDs were
removed with the system API after the probe process exited; the recorded-profile
ledger is empty. No real credentials, remote bot, scheme registration, user browser
profile or Computer Use was involved. This is a protocol/data-store runtime check,
not visual or physical-gesture acceptance.

## Closing device-transfer findings

Coordinator-run local TCP tests now use two synthetic fully migrated SQLite/FTS
stores and the production sender/receiver. Four added regressions passed with
actual file bytes, imported metadata, timestamps and FTS results as oracles. They
exposed and fixed root-scoped transcript omission and sender-absolute paths. A
shared file UUID across ordinary/transcript rows now requires identical bytes;
conflicting copies terminate rather than producing a successful restore.

The coordinator then found and integrated a reference difference: Flutter streams
file encryption/decryption, while the prior Rust packet path retained whole-file
and ciphertext buffers. The new private codec uses 64 KiB chunks, authenticated
RAII staging and unchanged v3 framing. Thirty transfer tests passed in the complete
coordinator-run workspace suite, including a fixed NIST vector, old-codec wire
compatibility, corruption/truncation/cancellation cleanup, receiver conflicts and
real SQLite/TCP results. A backpressured control write now obeys the same operation
cancellation lifetime. Known persistence errors fail restore without dumping rows.
Completed ordinary media also stores portable paths; unfinished source URLs remain
unchanged. Failed transfers may retain earlier metadata; no full transaction
rollback is claimed.
This local data-plane validation is separate from mobile control-plane and actual
phone interoperability.

## Complete current queue

| ID | Current disposition | Owning source, implementation and remaining evidence |
| --- | --- | --- |
| L01 | SOURCE-READY | N:login.rs; A:login.rs; C:runtime/login.rs. Owner disposal cancels authorization, retry replaces prior login, timeout refreshes QR, only readiness/completion is logged in the NSH path. |
| L02 | SOURCE-READY | N:lib.rs App restore; C:runtime/desktop.rs and lifecycle.rs. Startup typed Unauthorized invalidates the matching user/session/key Auth snapshot before presenting sign-in; persistence failure cannot leave cached authority reusable. Other failures retain Retry. |
| L03 | SOURCE-READY | N:home.rs/profile subscription and sidebar; A:account.rs::profile_changes. Shared account profile event is the source of visible account metadata. |
| L04 | SOURCE-READY | N:home.rs; C:runtime/lifecycle.rs consumes SDK REST authentication errors and Blaze Unauthorized under the account lifetime. It retires authority, cancels downloads, invalidates only the matching Auth, publishes terminal health and shuts down MCP/runtime. API health streams retain terminal state across shutdown races. Applied proxy changes reconnect the transport. |
| L05 | SOURCE-READY | N:settings/profile.rs confirmation; C:runtime/lifecycle.rs and desktop.rs. Fallible credential/Auth preflight retains the active runtime and MCP on failure, allowing retry. Auth write failure restores the credential; failed restoration retains the memory verifier and locked cover. Teardown starts after preflight; cleanup errors cannot leave Home attached to a retired account. |
| L06 | SOURCE-READY | N:main.rs native entry; runtime.rs tracked commit/read owners and bounded shutdown; shell instance guard retained through native exit. Runtime outcomes are coordinator-owned evidence. |
| L07 | PLATFORM-VALIDATION-PENDING | N:security/*; C:runtime/screen_lock/*. macOS Keychain and real isolated Linux Secret Service passed native credential checks and full public-API PIN setup/wrong-code/correct-unlock/removal. Stored records are hashed; provider read failures retain the protected state. Windows Credential Manager native validation remains pending. |
| L08 | PLATFORM-VALIDATION-PENDING | N:security/mod.rs owns inactivity timers, locked presentation and biometric cancellation. Linux native credentials/PIN lifecycle passed; physical inactivity gestures and biometric prompts remain unverified. NonmacOS biometrics are explicitly unavailable in the migrated reference scope. Windows native checks remain pending. |
| L09 | DEFERRED-BY-USER | Old local account/history/settings import is explicitly a separate future task. No importer, migration run or personal-data access was performed. |
| N01 | SOURCE-READY | `N:sidebar.rs:28-69,73-152,329-360`; `sidebar/preference.rs:19-87`. 176/64 widths, <=384 drawer and <496 auto compact match `F:ui/home/desktop_shell_layout.dart:47-80`; durable manual preference is separate from automatic collapse, with rollback/error. |
| N02 | SOURCE-READY | `N:home.rs:290-321,576-620`; `N:sidebar.rs:115-164`; `N:chat.rs` nested inspector/back presentation. Main and side route switches use available 620-point region; drawer trigger remains reachable from list, chat/settings back expose list, session draft owner survives route disposal. |
| N03 | SOURCE-READY | `N:sidebar.rs:282-326`; `N:home.rs:198-258`; `C:db/mixin/conversation.rs` category predicates and unseen projections. All/contact/group/bot/stranger categories query the Rust owner and use account event streams. The common 500-row browsing gap is N05. |
| N04 | SOURCE-READY | `N:conversation_list.rs:47-259`; `N:appearance/badges.rs`; `N:ui.rs::conversation_avatar`; `N:presentation/datetime.rs`. Draft precedes last-message preview, participant puzzle/avatar fallback and sender/delivery/unread/mention/mute/pin/identity badges use current DTO data. |
| N05 | SOURCE-READY | N:conversation_list/window.rs; C:db/mixin/conversation.rs. A 100..300-row bidirectional window uses the same filtered business position query; real SQLite fixtures reach rows 500/600 among 660 conversations without raising the DAO cap. |
| N06 | SOURCE-READY | `N:home.rs` unread toggle/clear; `N:search/query.rs:35-45,79-120`; account unseen subscriptions. Unread filtering applies to conversations/message scope and suppresses unrelated user matches, with explicit clear-filter state. |
| N07 | SOURCE-READY | `N:conversation_list/actions.rs:38-82,175-211`; Core set_pinned facade. Menu/native command rereads the current row before choosing durable pin state. Search-context reachability is V04. |
| N08 | SOURCE-READY | `N:conversation_list/actions.rs:85-123,216-324`; direct/group targets and one-hour/eight-hour/week/year/off durations; cancel has no command, current mute is reread before toggle. |
| N09 | SOURCE-READY | `N:conversation_list/actions.rs:126-170`; confirmed object-scoped delete, accepted account-owned command and selected-chat cleanup; `N:navigation.rs:90-126` also detects deleted selected rows. |
| N10 | SOURCE-READY | `N:create/contacts.rs` direct-selection branch; `N:navigation.rs::open_user`. Paged eligible choices open/create the selected user's conversation without sending. |
| N11 | SOURCE-READY | `N:create/lookup.rs:17-37,44-99`; Core search validation/persistence. Lookup is disabled while busy/IME and opens the actual user profile, with not-found/error state and return. |
| N12 | SOURCE-READY | `N:create/contacts.rs:78-112,286-375`; Rust create_group. Contacts-first, 1023 invitees plus self, puzzle/count preview, 40-character name and busy guards; created ID is retained if later projection loading fails. |
| N13 | SOURCE-READY | `N:create/circle.rs` name/selection/commit branches. Choice keys deduplicate direct owner and contact entries; bounded pages, chunked edits and created-circle retention reconcile partial success without creating another circle. |
| N14 | SOURCE-READY | `N:sidebar/circles.rs:75-139,142-432`; context rename/delete and pointer drag with explicit commit, bounded edge autoscroll and cancellation on authoritative order change. Deleting selected circle restores chats. Physical drag remains unverified. |
| N15 | SOURCE-READY | `N:sidebar/circle_editor.rs:16-29,54-103,130-192`; shared batched edit command. Persisted memberships are reread before apply; selected changes survive pages, batches are <=200 and partial results reconcile. |
| N16 | SOURCE-READY | `N:sidebar/circles.rs:439-563,594-609`; `N:conversation/circles.rs:57-124`. Current membership query/event stream drives each circle, and click adds/removes only that source chat with owner/group semantics. |
| S01 | SOURCE-READY | N:search/results.rs reuses the existing conversation object menu only for Conversation matches; existing query versioning and message navigation are preserved. |
| S02 | SOURCE-READY | `N:search.rs` section tabs/pager; `N:search/query.rs:59-175`. Overview's three-per-section preview expands into 60-row pages, global message cursor history is retained and no fixed total result cap is substituted for pagination. |
| S03 | SOURCE-READY | `N:search/remote.rs:20-77,137-155,193-215`; `C:runtime/user.rs::search_mao_user/search_user`. `.mao` completion, numeric action and original server lookup/persistence are source-equivalent, with stale-request guards. |
| S04 | SOURCE-READY | `N:search/remote.rs:50-55,158-178`; HTTP/HTTPS banner invokes the shared LinkNavigation. The actual Flutter search URL banner likewise accepts HTTP/HTTPS only (`F:ui/home/conversation/search_list.dart:139-141,174-180`). |
| S05 | SOURCE-READY | N:search/palette.rs; C:db/mixin/quick_search.rs and runtime/quick_search.rs. Unified ranking precedes global pagination. Query/page transitions synchronously disable retained results; highlight/select and ticket+query+page completion guards prevent opening an old result during debounce or loading. Empty-query recents and owner-scoped live refresh remain. |
| S06 | SOURCE-READY | `N:inspector/search.rs` sender/text/post filters, bounded three-page cursor window, two-phase splice, deferred/dirty work and explicit retry. Actual `F:...search_message_page.dart` has sender/category, not date filters; no date-picker feature is missing. |
| T01 | SOURCE-READY | N:chat/timeline.rs, timeline/query.rs; 199-row maximum, bidirectional adjacent pages, surviving-anchor refresh and two-phase splice through presentation/message_window.rs. |
| T02 | SOURCE-READY | N:chat/timeline.rs, navigation.rs; explicit target window, missing-target state, native scroll/highlight, return origin/latest controls. |
| T03 | SOURCE-READY | N:chat.rs, chat/unread.rs; active lifecycle/latest-visible gating, initial unread divider, real row/viewport checks after index reuse. |
| T04 | SOURCE-READY | N:chat/unread.rs; unread mention query, pending visible target acknowledgement, clear transaction and failure retry; Core message-mention owner. |
| T05 | SOURCE-READY | N:chat.rs, chat/message_appearance.rs, appearance.rs, presentation/datetime.rs; grouped sender/dates/status assets and durable avatar/identity/font preferences. |
| T06 | SOURCE-READY | N:chat/message_text.rs and message_text/reader.rs map literal text and image captions, safe links, email, bot identities, resolved mentions and cross-span Unicode search emphasis. Captions share the same readonly selection surface and batched mention resolution; see A03. |
| T07 | SOURCE-READY | N:chat/gallery.rs permission-checked forwarding and presentation-only rotations preserve source bytes. C:runtime/attachment_recovery.rs revalidates DONE/READ images through ordinary and transcript download owners. Missing or confirmed-corrupt files can recover; valid, oversized or unsupported originals are preserved. Coordinator live ordinary-PNG recovery passed; transcript live recovery remains unverified. |
| T08 | SOURCE-READY | N:chat/message_media.rs, message_detail.rs; bounded video poster/duration/transfer overlay and native video controls; preview pauses on close and owner disposal. Native playback/fullscreen interaction remains a coordinator/platform check. |
| T09 | SOURCE-READY | N:playback/queue.rs scopes bounded queues to conversation, pins or transcript root. Only explicit natural Finished events continue a page; stop/new request invalidates it. Same-root audio queries have real SQLite tests. |
| T10 | SOURCE-READY | N:chat/message_transcript.rs exposes restricted root+child attachment save/copy. C:runtime/attachment_export.rs resolves persisted identities; six live ordinary/transcript exports were independently byte-checked. |
| T11 | SOURCE-READY | N:sticker.rs uses actual sticker ID/URL guarded failure recovery and real viewability. C:db/mixin/sticker.rs conditional metadata UPSERT prevents same-URL refresh loops; last_use updates remain persisted without renderer invalidation. |
| T12 | SOURCE-READY | N:chat/message_quote.rs, mentions.rs, navigation.rs; shared author/member/media quote + reply bar, source jump/return, safe unavailable types; current quoted-sticker projection is hydrated by the owning DAO and refreshed by account events. |
| T13 | SOURCE-READY | N:chat/message_content.rs bounded 400-point POST preview; message_post.rs full selectable Markdown reader and original-conversation link routing. |
| T14 | SOURCE-READY | N:chat/message_content.rs contact card resolves shared-user identity and opens conversation/profile actions through existing account navigation. |
| T15 | SOURCE-READY | N:chat/message_location.rs preserves the reference 260x180 Google tile card, pin, visible title/address/coordinates and external action. Only intersecting tiles are created. Public fixture returned PNG; 168 external pure projection cases passed. Additional regional provider attribution remains unverified. |
| T16 | SOURCE-READY | N:chat/message_detail.rs::card and message_links.rs; APP/card actions preserve original conversation/app context, input: command, generic-vs-verified bot host and card data. |
| T17 | SOURCE-READY | N:chat/message_transcript.rs has restricted copy/save actions, actual viewport pause and same-root continuous audio. Root independently checked the bounded SQL scope and explicit playback continuation. Authorized encrypted transcript roundtrips and exports have persisted byte-level evidence. |
| T18 | SOURCE-READY | N:chat/message_snapshot.rs, snapshot.rs; correct snapshot/safe dispatch, amount/asset/date/identity/memo/details; render path never initiates payment. |
| T19 | SOURCE-READY | N:chat/message_system.rs::inscription, snapshot.rs; projected collection/item/hash/artwork/sequence plus bounded cancellable Core text-content read. Real online inscription fixture remains a separate evidence limitation. |
| T20 | SOURCE-READY | N:chat/message_system.rs + chat.rs; actor/action-specific system and PIN states, recall/waiting/unknown handling before body, actual encryption notice and no raw ciphertext. |
| A01 | SOURCE-READY | N:chat/message_actions.rs, message_selection.rs; bounded business policy, pinned/transcript restrictions and item identity snapshots; mutators revalidate in Core. |
| A02 | SOURCE-READY | N:chat/message_quote.rs, chat.rs; reply persists original quote ID on text send, cancel clears, accepted send clears only matching pending quote. |
| A03 | SOURCE-READY | N:chat/message_text/reader.rs uses the public readonly editor for TEXT and image captions, with root+message+session identity. Ordinary/transcript Copy prioritizes selected display text, then full wire text/caption. Six schema/UTF16 mapping and ten real CoreText/Core/Taffy layout cases passed. Physical selection/link gestures and OS clipboard readback remain unverified. |
| A04 | SOURCE-READY | N:chat/message_forward.rs + recipients.rs; friends/bots without chats resolve on selection, Core atomic forward operation; gallery-specific missing entry recorded under T07/R07. |
| A05 | SOURCE-READY | N:chat/message_selection.rs and private message_selection/copy.rs. ID-based selection survives bounded windows; combine keeps the reference 2..99 limit and Core atomic transaction. Multi-copy queries authoritative DTOs in batches of 100, sorts time/ID and includes sender/local time/body. Query failure preserves selection; clipboard command dispatch clears only the unchanged selection snapshot. OS clipboard acknowledgement is unavailable. |
| A06 | SOURCE-READY | N:chat/reedit.rs, message_actions.rs, chat.rs; own single TEXT successful recall cached six minutes, conversation-matched append intent, lock cleanup; Core recall permission/time checks. |
| A07 | SOURCE-READY | N:chat/message_actions.rs; C:runtime/message/pinning.rs; current role/status/type check, persisted pin event/job and notifications. |
| A08 | SOURCE-READY | N:chat/pinned.rs, inspector/pinned.rs; default-visible account/conversation dismissal, direct source jump, bounded list/count/anchor, unpin-all confirmation and business permission/transaction; pinned media scope reused. |
| A09 | SOURCE-READY | N:chat/message_actions.rs, message_detail.rs::add_image_sticker; existing sticker add and media validation/preparation with failure display. |
| C01 | SOURCE-READY | N:composer/input.rs and chat.rs; literal editor schema, UTF16/mention atom mapping, Enter/ShiftEnter, composition checks. Installed NSH controller.rs::key returns before plugin hooks while composing. |
| C02 | SOURCE-READY | N:drafts.rs + chat.rs; account-authoritative draft command, bounded coalesced wake-up, current-text guard on accepted send, failures retained/retry rather than second durable store. |
| C03 | SOURCE-READY | N:runtime.rs::commit, chat.rs, composer/files.rs; per-conversation pending guard, accepted work outlives view, file receipt identity retained, partial preparations pruned only by accepted count. |
| C04 | SOURCE-READY | N:composer/mentions.rs + input.rs + chat/mentions.rs; @ candidate/search, current-request check, keyboard selection, wire identity insertion and batched name resolution. |
| C05 | SOURCE-READY | N:composer/attachments.rs, files.rs and private files/preparation.rs retain one preview owner across picker/drop/paste additions, with stable source IDs, deduplication, edits and prepared leases. Explicit files/media/archive modes share real preparation; Archive has an ephemeral masked optional password. Cancel sends nothing; picker/busy gates preserve the accepted send snapshot. |
| C06 | SOURCE-READY | N:composer/files.rs; A:media.rs; M:attachment.rs and archive.rs. Compression/original choices, image/video preview, dimensions/duration and caption feed accepted receipts. ZIP retains displayed filenames and leases, uses AES-256 for a password and plain ZIP for an empty password. Coordinator actual Rust/Dart/bsdtar cross-read checks passed; Unicode compatibility is qualified in the closing evidence section. |
| C07 | SOURCE-READY | N:composer/image_editor.rs and geometry/surface/preview modules; disposable source-preserving crop/ratio/rotate/brush/eraser/undo/redo and shared-media raster export. |
| C08 | SOURCE-READY | N:chat.rs file-drop/on_paste_files and composer/attachments.rs/files.rs. Clipboard files, picker selections and drops append to the existing preview without remounting its editing owner. Busy sends reject additions visibly; ordinary text remains editor input. |
| C09 | LIVE-PENDING | N:composer/voice.rs and voice/surface.rs keep one session owner with composer-anchored recording/preview, discard confirmation and stop-then-send. M:recorder/progress.rs derives bounded telemetry from actual accepted frames. Physical microphone capture/send and visual placement remain unverified. |
| C10 | SOURCE-READY | N:composer/attachments.rs/forms.rs; shared-contact selector, POST editor, quoted send/busy/error/accepted close. |
| E01 | SOURCE-READY | N:composer/emoji.rs + assets/emoji.json; categories/search, bounded recent/skin preference, current-caret insert via editor command. |
| E02 | SOURCE-READY | N:composer/stickers.rs subscribes once per open surface before querying; stale asset failures queue the existing deduplicated refresh job and metadata updates refresh the displayed source. |
| E03 | SOURCE-READY | N:composer/sticker_store/album_order.rs adds pointer drag, edge scrolling and one command on valid drop; arrows remain available. Source order updates, Escape, resize or disposal cancel the gesture. Shared asset recovery and independent detail subscriptions are connected. |
| E04 | SOURCE-READY | N:composer/stickers.rs menus/import and Core sticker file validation; personal add/remove refresh and visible failure. |
| E05 | SOURCE-READY | N:composer/gifs.rs; C:runtime/gif.rs + message/remote_image.rs; trending/search/page/error, existing distributor Giphy client configuration, proxy-aware bounded GET and same-ID cancel/retry/upload. The Flutter reference also supplies its client key from build configuration. |
| G01 | SOURCE-READY | N:conversation/info.rs uses the projected conversation row name and the existing localized group creation timestamp. |
| G02 | SOURCE-READY | N:conversation/members.rs reuses identity_badge for existing verified/bot/membership fields; no per-member query. |
| G03 | SOURCE-READY | C:db/mixin/user.rs filters eligible friends, self/system/deactivated/existing participants before LIMIT. N:conversation/members.rs keeps 100 DTOs plus selected IDs across pages. The 650-contact real SQLite fixture proves all 600 eligible contacts remain reachable. |
| G04 | SOURCE-READY | N:conversation/info.rs retains independent initial name/announcement values, sends only changed fields, disables unchanged submit and uses the reference 40-character name limit. |
| G05 | SOURCE-READY | `N:conversation/info.rs:353-399`; `N:links/view.rs` group invite confirmation. QR/copy/reset and server-refreshed detail are connected; old invitation invalidation uses the existing business command. |
| G06 | SOURCE-READY | `N:conversation/info.rs:185-351`. Presets/off, quantity plus seconds/minutes/hours/days/weeks and the reference 59/59/23/6/4 UI bounds are present. Checked conversion, role/busy gate and persisted active selection avoid optimistic state. The four-week UI bound is not asserted as a protocol limit. |
| G07 | SOURCE-READY | `N:conversation/info.rs:30-91`; `N:conversation_list/actions.rs:126-170`; Core clear/exit/delete owners. Clear history, membership exit and local deletion remain separate confirmations/commands and deleted selection is observed. |
| G08 | SOURCE-READY | `N:conversation/shared.rs`. Lazy retained media/post/file tabs, actual three/four-column media grid, bounded 180-row bidirectional window, native edges, phase splice and request guards; content operations are separate from Locate. Shared MediaPreview reuses gallery/reader/transfer/file operations. |
| G09 | SOURCE-READY | `N:conversation/user.rs:326-423`; `C:runtime/user.rs:152-226`. Current shared apps are loaded through the business owner and each ID opens its real profile; app home remains an explicit subsequent action. Error retry exists for this child page. |
| G10 | SOURCE-READY | N:conversation/user.rs gives common groups its own query error, Retry revision and non-error empty state. |
| U01 | SOURCE-READY | N:conversation/user.rs; C:runtime/user.rs and UserProfileItem. Local-first profile, one initial remote refresh, account events, deactivated state and identity badges are present. Own profile omits Message using the existing own_id while retaining Share contact. Core continues to reject self-conversations. |
| U02 | SOURCE-READY | N:conversation/user.rs relationship/name commands use existing Core add/remove contact operations and version guards. Account events refresh metadata; the UI has no second durable contact store. |
| U03 | SOURCE-READY | N:conversation/user.rs and C:runtime/user.rs implement confirmation, block/unblock and report-and-block through the SDK. Composer permissions are rechecked by shared business commands. |
| U04 | SOURCE-READY | N:conversation/user.rs shares the actual user through recipient selection and copies its mixin URI via existing link navigation; no automatic send occurs. |
| U05 | SOURCE-READY | N:conversation/user.rs and chat header query the actual bot creator/home URI, retaining original source conversation and the verified bot routing boundary. Windows/Linux native browser sources are integrated; native behavior remains D09 acceptance work. |
| P01 | SOURCE-READY | N:settings.rs master/detail, Home responsive route/back handling and settings category state. Release transfer entry was fixed during audit. |
| P02 | SOURCE-READY | N:settings/profile.rs + avatar picker/crop; account update facades. Draft/validation/failure and actual encoded avatar path are present; no live profile mutation was performed here. |
| P03 | SOURCE-READY | N:settings/appearance.rs; App appearance setting subscription. Immediate application, persisted business key, system mode and failure rollback are present. |
| P04 | SOURCE-READY | N:appearance.rs + settings/appearance.rs and message/composer consumers. Typed presentation preferences subscribe to persisted settings; avatar/identity/font changes affect actual message rendering. |
| P05 | SOURCE-READY | N:i18n.rs/language/catalog/aliases, preserved ARBs and supplemental catalog. Nine locales, system resolution, named formatting and save failure rollback are implemented; native-speaker/expansion behavior is unverified. |
| P06 | PLATFORM-VALIDATION-PENDING | N:settings/notifications.rs and platform/notifications*.rs expose target-accurate availability and permissions. Windows uses the actual installed package identity; Linux has no permission prompt. The private real D-Bus service fixture passed payload, click, replacement, withdrawal, daemon restart and bounded recovery contracts. Windows/Linux desktop behavior is unverified. |
| P07 | SOURCE-READY | N:settings/storage.rs independent three toggles; A:desktop.rs streams; C:runtime.rs attachment request decision uses typed SettingDao settings. |
| P08 | SOURCE-READY | N:settings/storage.rs overview/category detail/confirmed clear; C:runtime.rs filesystem usage/clear paths match the reference storage deletion behavior. No additional durable UI store. |
| P09 | SOURCE-READY | C:network.rs supplies watched HTTP clients to SDK, login and attachments; native WebSocket uses bounded HTTP CONNECT or SOCKS5 with original-target TLS validation. Local transport fixtures and compiled consumers passed. Ambient HTTP environment-proxy behavior remains as in the reference. |
| P10 | SOURCE-READY | N:settings/about.rs preserves own version, legal/social/help/log actions and the reference platform update destinations. Windows opens the existing Microsoft Store listing; Linux opens the official messenger page. No automatic NSH updater or new published release is claimed. |
| P11 | PLATFORM-VALIDATION-PENDING | Same settings/secure owner as L07/L08. Native Mac Keychain and isolated real Linux Secret Service/PIN contracts passed. Strict missing-vs-provider-error handling and business rollback tests passed. Windows secure-store runtime and physical biometric behavior remain pending. |
| P12 | REFERENCE-EXCEPTION | F:ui/setting/setting_page.dart gates destructive account entry to primary iOS. No new desktop account deletion/PIN/phone flow is required. |
| P13 | REFERENCE-EXCEPTION | F:ui/setting/backup_page.dart remains unlinked desktop placeholder with no-op switches. Actual device transfer is D10. |
| D01 | PLATFORM-VALIDATION-PENDING | C:runtime/notifications.rs batches mention/actor/pin mapping and redacts UNKNOWN/FAILED bodies before public events. Mac, Windows WinRT/COM and Linux D-Bus native adapters are integrated. Coordinator reran the real private Linux D-Bus fixture and Windows exact-source API checks. Native desktop delivery/withdrawal is unverified. |
| D02 | PLATFORM-VALIDATION-PENDING | N:notifications/session.rs routes typed account/conversation/message targets after revealing the same main window. Windows cold COM activation precedes second-launch IPC; warm activation retains generation ownership. Linux default actions route the same target. Exact-source API/real IPC and private D-Bus checks passed; actual desktop activation is unverified. |
| D03 | SOURCE-READY | N:shell.rs dispatch reveals the existing main window after disabled/lock checks before executing native route or creation commands; no second window is constructed. |
| D04 | PLATFORM-VALIDATION-PENDING | N:shell.rs, shell/window_state.rs, platform/shell_*.rs and shell/instance/* own process handoff, native lifecycle and durable bounds. Native Linux AppRun passed isolated X11 existing-chat URI/second-instance/SIGTERM, final SQLite geometry and account shutdown; destroy now disposes the NSH engine before loop exit. Windows exact APIs/IPC contracts passed; real Windows desktop and physical Wayland behavior remain unverified. |
| D05 | PLATFORM-VALIDATION-PENDING | Mac early AppleEvent routing and Alternate registration remain. Windows MSIX protocol metadata and Linux %U desktop entry are supplied by native packagers; native argv is forwarded through the same guarded instance protocol. Manifest/entry contracts passed coordinator checks; installed OS URI activation is unverified. |
| D06 | SOURCE-READY | N:links/parse.rs/router; Core resolve_conversation_link. Source account and conversation/user/message validation, invitation confirmation, request versions and missing-target handling are implemented. |
| D07 | SOURCE-READY | N:links/send/preview/recipient and chat/message_links.rs. Explicit send preview where required, input: card command retains original conversation, generic/bot web context supplied, coarse business send permissions rechecked. |
| D08 | SOURCE-READY | N:links/view.rs + snapshot.rs. Payment/multisig codes are read-only with asset/amount/participants/threshold; unsupported actions show phone QR/handoff and do not issue financial writes. |
| D09 | PLATFORM-VALIDATION-PENDING | Mac WebKit, Windows WebView2 and Linux GTK/WebKit adapters share account/session ownership. The coordinator ran actual Mac and Linux native-browser fixtures: native frame/URI/generation, bounded replies, reload/restart, profile isolation and real purge. Linux current-context retention was fixed after a real timeout. Windows native runtime, full bot/Core-signing UI interaction and physical gestures remain unverified. |
| D10 | SOURCE-READY | C:core/device_transfer/* retains request/operation snapshots, stale-token guards and cancellation across the full worker. The v3 file codec streams bounded AES/HMAC chunks into authenticated RAII staging. Coordinator-run complete workspace tests include 30 transfer contracts with real local TCP/SQLite, scoped/ordinary media relocation, old-codec compatibility, corrupt data, blocked control cancellation and conflict/failure handling. Actual mobile-peer control-plane interoperability remains unverified; the reference desktop menu is debug-only. |
| D11 | REFERENCE-EXCEPTION | No reachable Flutter MCP settings entry in the audited reference. Existing Rust headless/local MCP capability remains preserved. |
| D12 | REFERENCE-EXCEPTION | No Flutter desktop call/VoIP source entry/service identified; recall is unrelated. No new call feature is justified by parity. |
| B01 | SOURCE-READY | Shared applied proxy configuration now controls SDK/login/Blaze. Root repeated actual mixtest restore, profile, live Blaze and bounded shutdown. Local HTTP/SOCKS transport fixtures passed through the owning paths. |
| B02 | SOURCE-READY | SDK push_signal_keys now accepts successful void bodies without weakening strict count/error parsing. Root reproduced the prior startup warning, then verified successful registration. Authorized encrypted text/PNG/Opus/transcript fixtures retain byte-level and READ evidence. The second real restore read a count of 3000 without upload or warning; the account-scoped uploaded flag was independently read as true from SQLite. |
| B03 | SOURCE-READY | Core jobs/sync/SQLite/FTS owners and account-scoped command transactions are retained. No duplicate UI durable message/conversation DB or DAO import was found in inspected UI surfaces. |
| B04 | SOURCE-READY | Shared proxy-aware attachment clients, same accepted-ID upload cancel/retry, progress and READ/DONE were independently exercised. Ordinary/transcript exports preserve exact fixture bytes and PNG pixels. Latest public-API ordinary DONE/READ PNG missing/corrupt recovery passed SHA verification without SQL repair; valid originals were unchanged and backup removed. |
| B05 | SOURCE-READY | Core expiration/recall/pin/mention rules; recently added transaction-backed pin/unpin and current-role guards remain authoritative. UI commands use facade and event queries. |
| B06 | SOURCE-READY | Transfer observation now shares a retained snapshot source; explicit shutdown/drop releases subscriptions. Owner-scoped reads, accepted commits, lag recovery, stale request and operation checks were source-reviewed; deterministic business tests pass. |
| B07 | SOURCE-READY | Application semantic theme/controls and owner-scoped motion remain shared. Voice uses a short bottom transition; drag follows the pointer immediately. Reduced motion removes spatial movement. The actual nine-locale catalog harness resolves 652 pairs and 34 templates; native appearance/a11y/motion remain visually unverified. |
| B08 | SOURCE-READY | A:error.rs central ClientResult/ClientError, account-scoped DTOs/commands/streams; public presentation types remain NSH-free and owning DAOs remain in Core. Consumer builds remain coordinator-owned checks. |
| B09 | PLATFORM-VALIDATION-PENDING | Mac Keychain and real private Linux Secret Service passed credential contracts and full public-API PIN lifecycle; unavailable Linux bus remains an error. Process/account authority, hashed records and QR secrecy remain shared. Windows secure-provider runtime and physical biometrics remain unverified. No personal credentials or pairing URIs were displayed. |
| B10 | PLATFORM-VALIDATION-PENDING | Native Mac Debug/Release ad hoc bundles and actual Ubuntu ARM Debug AppDir/tar.gz passed builds and independently executed runtime checks. Linux includes the real extension, system dependency inventory, notices and print-only exact-path AppArmor generator; kernel attachment and cleanup were independently verified. Windows real-PE classifier/API checks passed, but native Windows SDK/package/runtime and distribution signing/notarization remain unverified. |

## Actual native Linux closing evidence

The coordinator found the existing stopped Ubuntu 24.04 ARM VM and used a dedicated
owned HGFS share for source, Rust 1.97.0 and build output. Native GTK 4.14.5,
WebKitGTK 2.52.6, ALSA, D-Bus and protobuf development dependencies were installed.
This replaced the earlier Mac-host cross-metadata limitation with a real Linux SDK
build. The final workspace passed 421 tests and warnings-denied all-target Clippy.

Actual native execution exposed and closed these platform defects:

- The Linux entry now converts the SDK's GLib exit code to the process exit type.
- SIGTERM/SIGINT enter GTK's main context and use the existing close/commit path.
- Native Quit destroys the main window before ending the application. NSH 0.4
  releases the engine from the window's destroy callback; deferring past the native
  command avoids reentrant host disposal. This preserves owner cancellation before
  draining the business runtime.
- The C extension now retains exactly the current JSCContext wrapper on its page.
  WebKit's transfer-full getter/cache does not retain that wrapper after callers
  release it. Document still uses weak references, avoiding a function/context
  ownership cycle. The actual old signature timeout was reproduced and the same
  native fixture subsequently passed.
- GTK owns titlebar teardown; the browser no longer replaces a realized titlebar.
- Ubuntu's restricted user namespaces need an exact application AppArmor policy
  for WebKit's sandbox. The AppDir includes a print-only policy generator. Nothing
  is installed automatically. The coordinator loaded and unloaded owned temporary
  exact-path policies; global restrictions stayed enabled. A real-kernel fixture
  proved complex literal paths match while a neighboring path does not.

The coordinator independently compiled and executed the unchanged production Linux
browser adapter and C extension with real GTK, JavaScriptCore and WebKit. Only NSH
registration/context delivery is recorded by the external fixture. Main-frame-only
native access, native URI/generation, duplicate IDs, the 32-request budget, stale
SPA/document replies, same-URL reload, actual web-process restart, account cookie
isolation, retained cookies on close and real purge completion passed. All native
windows and owned profiles were removed. A teardown crash in the fixture was traced
by GDB to retaining CookieManager after its DataManager owner; correcting the
fixture's release order resolved it. Production uses that manager only while its
NetworkSession owner remains alive; this was not reported as a product defect.
Evidence: `/tmp/mixin-nsh-linux-bot-native-evidence.json`.

Fresh t2 Linux authorization was performed through the public API and actual mixtest
phone approval. The existing machine-binding guard was preserved. Profile refresh,
Blaze and bounded queries passed. The authorized primary peer kept its existing
desktop session. Two marked Signal texts, a PNG and a short synthetic Ogg/Opus clip
then passed real upload/download and exact-byte verification; all four messages
reached persisted READ. Evidence:
`/tmp/mixin-nsh-linux-login-evidence.json` and
`/tmp/mixin-nsh-linux-native-roundtrip-evidence.json`.

The actual ARM64 AppDir and tar.gz were inspected for executable hashes, native
extension, dependency inventory, desktop metadata and required notices. Its AppRun
then passed saved-account restore, live Blaze, existing-chat URI, second-launch
handoff, persisted final geometry and completed account shutdown. A private real
GNOME Secret Service also passed public-API PIN setup, lock, wrong-PIN rejection,
correct unlock and credential removal. Separate native provider checks proved
binary set/get/update/delete and unavailable-bus errors. This did not borrow the
user/GDM display or keyring, disable WebKit's sandbox or apply global policies.
Evidence: `/tmp/mixin-nsh-linux-native-full-app-evidence.json` and
`/tmp/mixin-nsh-linux-secure-store-evidence.json`.

The t2 test account was subsequently restored to macOS through a new real login.
All Linux test account/primary credentials, temporary keyrings and runtime data were
removed, and owned kernel policies were unloaded. The dedicated VM share was
removed and the previously stopped VM was returned to its stopped state. A final
Mac Release runtime check passed after t2 restoration:
`/tmp/mixin-nsh-macos-post-linux-native-evidence.json`. The private native test display
is not physical Wayland, desktop notification activation or visual acceptance.
The synthetic voice check is not physical microphone capture.

## Outstanding concrete work

1. A03: the public readonly selection surface is implemented and its intrinsic
   width/height/scroll structure passed coordinator-run CoreText layout probes.
   Physical selection/link gestures and appearance remain unverified.
2. Windows/Linux: secure stores, notifications/activation, window lifecycle, typed
   second-launch IPC, URI metadata, bot hosts and package sources now exist. Native
   Windows compilation and desktop/package acceptance remain pending. Real Linux
   SDK, protocol, credential, package and isolated X11 runtime checks passed as
   recorded above. Physical desktop delivery/activation and Wayland are not inferred
   from those checks. Windows `192.168.31.212` previously returned `Host is down`
   with incomplete ARP resolution; the final bounded SSH check timed out. No
   trusted MAC/WOL record was available. Two other existing SSH targets were checked:
   `192.168.31.210` refused SSH and `codex-yangbin` identified itself as `MacTv.local`.
   No additional usable Windows host was found. The environment-information
   question remains pending.
3. D10 actual peer transfer, C09 physical microphone capture/send, native notification
   activation, physical bot interactions/Core signing from an actual bot, OS
   clipboard readback, biometrics and visual motion
   are not inferred from builds. No unrequested UI smoke-test suite is required.
4. The reference map tile endpoint supplies no known dynamic regional provider
   attribution metadata. The visible Google Maps source link is present; complete
   regional attribution has not been established by this migration.

The migration is **incomplete** while these applicable requirements remain.
