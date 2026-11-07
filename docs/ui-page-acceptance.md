# Page-by-page UI acceptance

Updated 2026-11-07. This is the current coordinator-owned UI plan. The user asked
for page layout, display and local interaction work, with implementation delegated
to fresh sub-agents and actual native acceptance performed by the coordinator.
Business send/receive, sync and protocol acceptance are not part of this pass.
Windows is deferred by the user. The first target is the running macOS NSH app.

The implementation run is stopped at the user's request. See the consolidated
[NSH roadmap](../nsh/ROADMAP.md) for current remaining work, authorization and
priorities; the dated observations below remain the evidence ledger.

Current priority: the user paused further image-editor work and asked to finish
the main features and UI first. Prioritize the conversation rail/list, primary
chat header/message/composer, chat information and common search/create entry
points. Existing editor observations remain evidence, but do not expand that
feature while these primary surfaces have concrete gaps.

The frozen [125-feature inventory](flutter-nsh-parity.md) retains migration scope;
its source dispositions do not establish UI acceptance. Earlier chat-page results
are recorded in [the migration evidence](flutter-nsh-acceptance.md).

## Ownership and acceptance

- The coordinator chooses page order, defines shared boundaries, operates the app,
  records failures, integrates changes and repeats the exact interaction.
- Sub-agents implement bounded fixes in assigned files. Their reports and passing
  builds are inputs to acceptance, not the acceptance result.
- Retain existing UI foundations, semantic tokens and NSH native behavior. Keep
  business state in the existing Rust owner and public API.
- For each page, check initial and conditional display, content bounds, local
  scrolling, keyboard focus, popup placement, cancellation and return navigation.
  Do not infer all window sizes, themes or languages from one visible state.
- Avoid general send-flow acceptance, creating groups/circles, editing account or group data, clearing
  content, changing security settings or initiating transfer. Form drafts may be
  inspected and cancelled. Sticker/GIF result activation can send immediately.
- The user separately authorized marked test text/image/short-voice messages
  between mixtest accounts. Four such messages were prepared from t1 to t2 only
  to provide visible gallery/playback fixtures; this is not protocol acceptance.
- Keep the signed-in t2 session. Do not relink or sign out just to display a page.
- The user authorized temporary theme, language, text-size and avatar/ID changes;
  restore the original values after each inspection. New system permissions are not granted by this UI pass.
- No automatic worktree, commit or push. No new UI test suite is required.

Statuses: `UNVISITED` means no current native observation; `INSPECTED` means the
listed state was viewed but the page's full UI contract is not yet accepted;
`REPAIR` means a concrete native defect is assigned; `VERIFIED` requires recorded
coordinator interaction after integration. `CONDITIONAL` needs an appropriate UI
state or fixture without introducing an unrequested business operation.

## Ordered page queue

| Batch | Page family | Native entry | Status and current evidence |
| --- | --- | --- | --- |
| 0 | Primary chat header | Open an existing chat | VERIFIED (current direct/group): t1 shows 12346372770 beneath its name; group shows its group badge and four-member count. Clicking the identity region opens Info; clicking again closes it. Search also opens and closes with the same button. The existing single side-panel state is retained. Other identity badge variants and compact bounds remain conditional; no extra expiry query was introduced for this header. |
| 0 | Primary reply composer | Message menu > Reply | VERIFIED (rebuilt exact regression): group 111 > secondary menu > Reply focuses the existing input; immediate paste enters NSH UI draft without another click. Quote bin/111 remains visible and Send becomes available. Draft was cleared and Reply cancelled without sending. Ordinary typing does not subscribe the focus effect. |
| 0 | Primary conversation preview | Conversation column | VERIFIED (current article/voice/draft): group row shows bin plus a single-line article excerpt instead of generic Article; t1 keeps Voice message. Local reply draft takes precedence, and clearing it restores sender/excerpt. Existing metadata also maps system/pin/recall/status categories, but those latest-message states are not claimed as native acceptance. Mention-name display is recorded separately below. |
| 0 | Primary search header | Home search / Unread | VERIFIED: Contacts > Unread changes placeholder to Search unread; typing mixtest displays an X. Clicking X clears only query, retains Contacts/Unread, and focuses the field; immediate paste enters a new query. Clearing again and disabling Unread restores ordinary Search (Cmd+K); All restores both conversation rows. IME preedit remains conditional. |
| 0 | Bot chat header app entry | Existing bot profile > Message | VERIFIED (entry/window shell): Lucky Coin chat displays its bot badge/7000103214 and the rebuilt Open app button before Search/Info. Clicking shows loading and opens its named native window at redpacket.mixin.space. Native More menu and Close > return to bot chat work. Direct t1 and group have no Open app button. Third-party page returned Unknown error, try again, and a phone-open handoff appeared in the main window; both were closed without retry/authorization/payment/send. External content is not accepted by this entry check. |
| 0 | Quote and latest navigation | Reply preview / timeline arrow | VERIFIED (current existing data): group 111 > Reply > click bin/111 preview positions the original message and shows its themed highlight, retaining the reply draft. Reply cancelled. In t1, scrolling up reveals Latest; clicking returns to the final voice message and hides the arrow. Existing sent-quote return history, unread separator and unread-mention navigation remain conditional on corresponding existing messages. |
| 0 | Group mention picker | Group composer > @ | VERIFIED (rebuilt current pointer/arrows): @mix displays one complete 48pt row; @no_member_98765 removes the candidate region entirely. @ restores three other members within the existing bounded list. Down/Up changes the selected candidate; clicking t1 inserts the styled name and returns to editor focus. Earlier Backspace check removes trailing space then the whole mention. Drafts were cleared, no Enter/Send. Empty visibility also drives the existing picker behavior in source; no claim of a live Enter submission. Loading/error/IME cases remain conditional. |
| 0 | Conversation mention display | Conversation preview / local draft | VERIFIED (current local draft): choosing t1 displays @mixtest-t1 in both the composer and the conversation row. Settings > Back retains the same draft/name after the list remounts. Replacing it with Local UI draft keeps the plain text; clearing restores the sender/article excerpt and removes the draft label. Current bounded row previews share the existing batch mention replacement contract. Ordinary sent-message mentions and name-change events are not separately claimed as native coverage. No new API or durable user copy. |
| 0 | Main link loading and phone handoff | Home search > Open link | VERIFIED (current local/invalid-target states): unsupported HTTPS mixin.one link now shows a natural-height card with complete 240pt QR, Copy and right-aligned Close; one Escape restores Home search focus/query. Invalid messages shape and a missing local message show a compact Error card. Invalid app read lookup visibly shows spinner/Loading in a short card with right-aligned Close; Escape cancels, then clearing search restores the normal list without a later popup observed. Send content/Cancel/Presence were unchanged. Compact full-window bounds remain conditional. |
| 0 | Inspector toggle state | Chat header Search / Info | VERIFIED (wide current fixture): native RED had no retained fill/AX selection. Rebuilt Search retains accent fill after clicking its search field and exposes selected; switching to Info clears Search and selects Info. Circles within Info retains the Info fill/selection. Clicking the same Info button closes the nested panel and clears both selections, restoring composer focus. No new state or animation layer. |
| 0 | Main responsive focus | Home list > chat > inspector | VERIFIED (current narrow/wide transitions): native regression first changed hidden HomeSearch; rebuilt Search > t1 moves focus to composer and paste displays Local UI focus in the visible draft. Info releases hidden composer focus; attempted paste does not change that draft. After Info exit, composer focus returns and paste appends restored. Search opens with its own focus and NSH query returns the current fixture result. Wide HomeSearch focus > native Zoom to narrow transfers to composer; paste displays Resize UI focus while Home query stays empty. All drafts/queries cleared and wide chat restored. Input during the 140ms Inspector exit transition is not covered by the post-exit pass. |
| 1 | Settings frame | Account / Preferences | VERIFIED (current navigation): all eight categories open; Back returns to chat. Rebuilt Settings removes Home search/rows from AX and Tab order. Nine Tabs follow the visible rail, Profile and Security; Return opens Security without entering a hidden chat. Query mixtest survives Settings > Back and restores its t1 result. Search tab/page and native row viewport reset when remounted; Home query/drafts/selection/bounded paging remain. The rebuilt narrow Settings route now shows categories first; Appearance/Notifications open, detail Back returns to categories, category Back exits, and reentry starts at categories. Native Window > Zoom switches narrow/wide while retaining Appearance. Minimum-size forms remain conditional. |
| 1 | Edit profile | Settings > Edit profile | VERIFIED (current form draft): right content scrolls independently to Save/Language/sign-out while header/categories stay fixed in the current shorter window. Empty and whitespace-only Name disable Save; original mixtest-t2 draft restored. After editing Name, language popup first Down/Return selects English; Home/Return restores System. No profile Save/photo change/sign-out performed; biography limits remain conditional. Avatar draft evidence is recorded separately below. |
| 1 | Avatar crop draft | Edit profile > Change avatar | VERIFIED (local pointer draft): isolated PNG opens crop; zoom changes 1 to 4.38, image drag changes crop, Reset restores original position/zoom 1, and Escape cancels. Account avatar is unchanged; Save was never activated. Keyboard crop movement and compact bounds remain conditional. |
| 1 | Privacy and security | Settings > Privacy and security | INSPECTED: current disabled-lock state displays. PIN/biometric configuration is not changed. |
| 1 | Notifications | Settings > Notifications | INSPECTED: permission explanation, refresh, preview option and system-settings action display within current shorter viewport. Refresh permission status returns unchanged without opening permission UI. No new permission granted or preview preference changed. |
| 1 | Data and storage | Settings > Data and storage | VERIFIED (current display): download options and two conversation rows display; after media fixtures, t1/total are 139.4 KB and group is 0 B. Automatic-download switches were not changed. |
| 1 | Conversation storage | Data and storage > Conversation | VERIFIED (zero/positive-byte selection and cancellation): prior zero-byte categories were disabled/muted and could not be selected. Current t1 Photos 77.7 KB and Voice 61.7 KB can be checked; Video/Documents 0 B remain disabled. Selecting either positive category enables Clear, clearing both selections disables it. First Clear opens compact warning; Escape closes and retains selection. Overview > t1 reentry clears the local selection, keeps 139.4 KB, and disables Clear. Final destructive confirmation was not activated; no cache cleared. |
| 1 | Proxy list and form | Settings > Proxy > Add proxy | VERIFIED (draft): title/four fields/footer visible, Host auto-focused, Tab moves to port, 00 and 70000 keep Add disabled, 80 enables it; Escape cancels. No proxy saved/enabled. Smaller-window form scroll remains conditional. |
| 1 | Appearance | Settings > Appearance | VERIFIED (current controls): Light/English/font offset +4/avatar off/ID on update preview and chat display. Mouse-open theme menu Down/Return selects Light; Home/Return restores System. Actual Tab reaches the font Slider. Rebuilt Right > Right > End > Home > Right > Right produces 1 > 2 > 4 > -2 > 0 with focus retained. A rapid burst ending at 2 survives About > Appearance; 0 restored with keyboard and confirmed after leaving/reentering Settings. Final values System theme/System language/font 0/avatar on/ID off. Compact bounds and pure Tab popup activation remain conditional. |
| 1 | About | Settings > About | INSPECTED: icon/version and help/legal/social/log actions display; independent content scrolling reaches Open log folder while header/categories stay fixed. No external destination or log folder opened. |
| 1 | Backup and restore | Debug settings category | INSPECTED: instructions and both directions display. Direction buttons initiate business commands and were not activated. |
| 2 | Conversation list and rail | Home categories / circles | VERIFIED (current categories): expanded/compact rail retains accessible category names and selection; Contacts/Bots empty scope prompts are specific to their category, Groups shows the existing group, Strangers shows t1. Bots + Unread shows category-specific no-unread prompt; Clear filters removes Unread while retaining Bots, All restores both rows. Later Settings traversal established visible rail Tab focus. A separate anchored-window refresh defect and its remaining exact-trigger coverage are recorded below. |
| 2 | Conversation menu | Conversation secondary click | VERIFIED (current rows): group and t1 secondary click display Pin/Mute/Circles/Delete entries; Escape dismisses without changing rows. Circles entry opens the current conversation membership card. Native integration caught a row-owner context regression; moving the single existing Circles handle to Home and explicitly passing it through Commands/sidebar repaired both rows and menus. Diagnostic logs removed. No conversation mutation. |
| 2 | Circle editor | Existing circle menu | VERIFIED (membership empty state): Right-click > Circles opens a compact card with current group name, centered zero-circle explanation and Done; Done returns to unchanged rows. Escape cancellation passed on the prior shared modal. Existing-circle rename/manage and populated membership remain conditional. No circle created or membership changed. |
| 2 | Global search | Conversation-column search | VERIFIED (current fixture): Messages shows “未找到消息”, Contacts shows “没有找到联系人”, no invalid pager; focused search Escape clears query and restores normal rows. Populated large-result pagination remains conditional. |
| 2 | Quick search | Cmd+K | VERIFIED (current fixture): single-row header, compact empty/two-row surfaces, initial focus, visible arrow selection and Return opens the highlighted existing chat and closes. After keyboard selection, opening article reader then one Escape now closes correctly. Large result list remains conditional. |
| 2 | User lookup | New > Search user | VERIFIED (current form/result): compact natural-height form, initial focus, invalid query retains input and shows inline error; editing clears the error. Return starts visible busy feedback with disabled actions. t1 result opens a bounded 340pt profile; Back retains the query and restores input focus, then one field Escape closes. Earlier profile-to-chat dismissal regression remains covered. Small-window/IME cases remain conditional. |
| 2 | New direct chat | New > New conversation / Cmd+N | VERIFIED (empty state): Cmd+N opens compact contact selector, search has initial focus, no useless pager, Escape cancels. Populated state remains conditional on eligible contacts. |
| 2 | New group | New > New group / Shift+Cmd+N | VERIFIED (zero-contact selection): rebuilt selector is compact, centered empty state, search initially focused, no useless pager, Next disabled and field Escape cancels. Name/populated stages remain conditional; no group created. |
| 2 | New circle | New > New circle | VERIFIED (current two-chat fixture): compact name stage, initial focus, compact recent-chat selector, selection/count, Back retaining name and selection, empty Contacts tab, no useless pager, field Escape cancels. No circle created; multi-page/large data remains conditional. |
| 3 | ChatInfo shared-app preview / entry | Info of a user with cached shared apps | VERIFIED (current empty/four-app fixture): bin Info shows the four overlapping icons; its direct entry opens one visible header and the same four-app list. Lucky Coin Profile Back returns to that list; root Back returns to bin Info with its preview. t1 Info hides the empty preview. Its original Profile > Shared apps empty page still returns to Profile. No external app opened. |
| 3 | Nickname-only editing | Direct Info / Profile > Edit name | VERIFIED (native drafts plus HTTP contract): t1 Info Edit name preloads mixtest-t1 with field focus. Empty/whitespace disable Save; valid draft enables it. Back discards the draft and reentry restores mixtest-t1. Stranger Profile exposes distinct Edit name and Add contact; their separate titles/forms and Back to Profile work. UPDATE payload and server error are covered by hermetic HTTP tests. No real Save, rename or contact write was performed. |
| 3 | ChatInfo biography / search entry | Existing bot Info > Profile > Back | VERIFIED (current biography and return paths): final package shows Lucky Coin’s existing bilingual biography in Info. Info Search takes field focus; Back and same header Search return to Info and its selected button. Ordinary header Search Back closes to Chat. Switching from Info-origin Search to t1 clears the origin; t1 ordinary Search Back stays in t1 Chat. Info-origin t1 query NSH returns its marked fixture; clicking closes the panel, positions and highlights it. A subsequent ordinary Search Back closes to Chat, proving the result jump did not retain the origin. Queries cleared and Latest restored. Group/nonempty-announcement and live user-update variants remain conditional. |
| 3 | Chat info | Chat header > Info | VERIFIED (current direct/group display and navigation): identity/member count, available rows and group creation date display; ordinary group member has disabled Expiry and no manage/invite actions. Direct Expiry opens with existing 1-week choice and Back returns. Direct Mute opens native four-duration/Cancel sheet; Escape cancels and retains Mute. Rebuilt main row now shows 1 week in both direct and disabled group states. No expiry or mute setting changed. Other nested pages remain separate checks. |
| 3 | Group members / selection | Chat info > Members | INSPECTED: four members and owner label display; query mixtest filters to t1/t2, menu exposes Profile/Message, profile opens and returns. Rebuilt profile takes focus from background field; one Escape closes and restores the original filtered member search focus. No membership change. |
| 3 | Edit group / invite | Managed group info | CONDITIONAL: appropriate permissions required; inspect drafts/QR/back only. |
| 3 | Disappearing messages | Chat info > Disappearing messages | VERIFIED (custom draft): Set is disabled for empty/0/60/non-numeric ab/99, enabled for Seconds=59; Hours updates range to 1–23, 23 enables and 24 disables. After local Select open focus handoff, typing 59 then clicking Unit and issuing the first Down/Down/Return selects Hours; focus returns to Unit and Set is disabled for 59 hours. Back discards the draft. Original one-week selection remains; no preset/Set activated. Group entry correctly disabled for current t2 role. |
| 3 | Shared media/files/articles | Chat info > Media | VERIFIED (current fixture): bottom three tabs retained, prior group Media/Documents empty states centered, Article shows bounded preview and author/time; switching to Documents removes old card from screen and AX. Direct Media now shows B/A thumbnails; B opens the matching gallery/caption, Escape returns to the same media grid. Documents and Article show their own centered empty prompts and remove the thumbnails; Media restores both. Back returns to ChatInfo. Multi-page media remains conditional. |
| 3 | In-chat search | Chat header > Search | VERIFIED (current fixture): initial search focus and centered empty prompt; query 11 shows bin111 and own11, Article filter produces centered empty state. Sender mode displays its own placeholder and focus; choosing t1 retains query/category, clearing sender/category restores results. Clicking bin111 closes the panel and locates the message. Sender-query current-account exclusion remains the existing Flutter/core rule. |
| 3 | Pinned messages | Pinned banner / native chat menu | CONDITIONAL: existing pins or isolated UI state required; no pin/unpin mutation. |
| 3 | User and bot profiles | Avatar / member / search result | VERIFIED (current user/bot navigation): t1 ChatInfo Profile > Groups in common shows existing group; selecting it opens that group and closes the side panel. Chat-avatar bin Profile modal matches Flutter 340pt width with bounded body scroll and fixed Close/Back. Shared apps shows four readable rows; Lucky Coin bot Profile > Back restores the same four-app page, then Back returns to bin Profile. Loading is shown once. Add contact opens auto-focused nickname draft, empty Save disabled; Back discards it and one Escape closes the modal. The exact regression loop passed both alone and after nested app navigation. Contact writes, opening bot apps, report/block and larger lists remain conditional; none activated. |
| 4 | Attachment preparation | Composer > Files / Images | VERIFIED (local drafts): single document is compact; append built-in app_icon.png, select row and switch image/document mode; image preview and caption remain inside body scroll with fixed title/footer. Eight isolated text files show at most five rows and scroll to file 08. ZIP mode reveals optional password field. Cancel clears all drafts; removing the only file closes. No upload/send. |
| 4 | Image editing | Attachment preview > Edit | VERIFIED (current local pointer drafts; further work paused): prior rotation/16:9 ratio/crop-corner/Reset checks passed. Fixture A visibly flips horizontally and a second Flip restores it. Red brush line Undo/Redo disappears/restores; Eraser opens a gap while preserving the underlying image, Undo restores and Redo reapplies it. Custom hue slider changes to 138 and paints a green line. Brush Done retains both lines; a new Brush session's Cancel discards only its added third line. Crop Done returns the edited preview; later Flip > Cancel opens discard warning, Cancel retains editor, Discard preserves the previous send preview. Outer Cancel clears the draft. No send or original-file edit; custom-color keyboard path and compact bounds remain conditional. |
| 4 | Contact sharing | Composer > Contact | VERIFIED (zero contacts): compact card, centered empty state, placeholder and initial search focus; typing mixtest retains centered empty display; no invalid pager, one Escape closes. Populated/multi-page rows remain conditional; no contact row activated or sent. |
| 4 | Article editing | Composer > Article | VERIFIED (local draft): rebuilt editor has initial focus and direct typing enters it. Empty/three-space drafts disable Send; local Markdown enables Send. A prior 30-line draft proved caret/end visibility and independent editor scroll with fixed title/footer. Editing Escape now closes in one press; reopening starts empty, and another whitespace-only Escape closes. Cancel retains busy/composition/Presence guards. IME preedit and compact bounds remain conditional. No send. |
| 4 | Multi-select / forward picker | Message menu > Select / Forward | VERIFIED (current fixture): selecting two messages updates count; opening search side panel retains a single-line count with actions wrapping into two rows. Forward opens a natural-height two-row picker with initial focus and no invalid pager; Contacts shows a compact centered empty state. One field Escape closes Forward and preserves two selected messages. Background search field consumes one Escape to release editing; a subsequent non-editing Escape leaves selection. Follow-up standard Checkbox exposes localized 选择消息 and checked 0/1; native checks selected a second message (count 2), unchecked the first (count 1), then Cancel removed selection. Receiver rows were not activated; no forward/delete. Large result pages and link-recipient return remain conditional. |
| 5 | Image gallery | t1 marked image fixtures | VERIFIED (two-image fixture): rebuilt A/B navigation updates both image and caption, with correct endpoint disabling. 125%/195% visibly enlarge the image; drag moves both axes and wheel scroll reveals another image region. Fit restores the full image; Rotate shows a contained portrait orientation. Previous resets rotation/zoom and caption to A; Right selects B. Save As defaults to B's filename, Cancel returns, and one Escape closes. Save/Copy/Forward were not executed. Long captions, large image sets and compact bounds remain conditional. |
| 5 | Video/audio viewer and bar | t1 marked voice fixture | VERIFIED (12-second audio fixture): bubble Play/Stop-reset and 1x/1.5x/2x/1x match Flutter. Switching to the group exposes the rebuilt bar with visible track/thumb/time and speed menu. Pause retains position; dragging to 6000ms shows 0:06 / 0:12, Play advances to 6203ms, and Pause retains 6469ms. Bar speed changes 1x > 1.5x > 2x > 1x. Source title opens t1 and hides the bar while preserving 6469ms in the bubble; switching back restores it, and Close stops/hides it. Video and compact bounds remain conditional. |
| 5 | Article / Markdown reader | Existing article or local file | VERIFIED (existing long article): finite Markdown preview stays within 400pt bounds, long code is clipped inside preview, wheel scrolls chat, click opens full wrapping reader. With detail panel open, article reflows within narrower chat; ordinary text/status remain visible. Reader Escape succeeds after keyboard quick-search navigation. Large multi-article navigation remains conditional. |
| 5 | Transcript reader | Existing transcript | CONDITIONAL: sender/date/media/scroll/nested return, without forwarding. |
| 5 | Message QR / detail / location / asset | Corresponding message/menu | VERIFIED (short text QR): text111 menu opens compact 360pt card with header Close, readable QR/value and Copy; Escape returns to chat. Existing article still opens its full reader after this size specialization. Long QR text and other message-detail categories remain conditional. |
| 5 | Emoji panel | Composer > Emoji | VERIFIED (current fixture): search takes initial focus, noemojiresult98765 shows centered no-results prompt, clearing restores grid. Animals switches grid and selected outline/AX category; default skin selection and labels display. Recent shows the pre-existing grinning face. Grid scrolling retains controls; one Escape from editing closes picker. No skin preference/recent changed or emoji cell activated in this pass. |
| 5 | Sticker tabs/store/detail | Emoji panel > Tabs / Store | VERIFIED (empty data): Recent and Personal show centered no-sticker prompt; Personal import stays fixed at bottom. Store displays centered no-albums prompt, fixed header/Refresh and no orphaned Your albums section. Store album/detail and populated grids remain conditional. No thumbnail/import/Add/Remove/Refresh activated. |
| 5 | GIF search | Emoji panel > GIF | VERIFIED (unconfigured fixture): centered unavailable prompt, fixed attribution and no invalid pager. Native replay caught disabled-only editing; final readonly predicate prevents typing cat from changing the value, and one Escape closes the picker. Configured results/paging remain conditional; no GIF activated. |
| 5 | Voice controls | Composer microphone | CONDITIONAL: pre-permission and local preview UI only; no new microphone permission or physical capture. |
| 6 | Link / invite / send confirmation | Home search > Open link | VERIFIED (text-send preview/error): short text uses natural-height card with one footer action area and header Close. Recipient choice has initial search focus and no Send action. Query mixtest > select t1 returns to preview, enables Send and focuses the persistent Cancel; one Escape closes and restores Home search. Query > header Back > Escape also passed. A 30-line preview scrolls to line 30 while title/recipient/footer stay fixed. Invalid base64 shows a compact error card; Escape closes. Unsupported action's phone handoff was inspected before this specialization. No Send/join/payment activated; image/card/invite states remain conditional. |
| 6 | Bot native window | Bot chat header > Open app | VERIFIED (current shell only): Lucky Coin native window has More/Close; More exposes Refresh/Forward/Copy link/Open in browser/Close. Menu Cancel and window Close restore the bot chat. Page content returned an error; retry, signing, authorization and payment were not exercised. Other navigation, reuse, compact bounds and external content remain conditional. |
| 6 | Login / restore error / lock cover | Appropriate isolated state | CONDITIONAL: layouts and local transitions without retiring t2 or configuring security. |

## Integration record

The first two UI batches and the attachment layout follow-up have been rebuilt
and operated by the coordinator. Acceptance is restricted to the exact states
marked VERIFIED above; it does not establish every page or every data condition.
Shared Escape routing removes persistent Home/Voice global bindings without
changing the native modal RequestClose / Presence contract. Local field
cancellation remains guarded.
First integrated replay confirmed contact/circle/global-search fixes in the exact states listed above. Quick-search header and natural-height results are fixed, but successful keyboard selection leaves a stale editing responder: opening a reader afterwards makes Escape and Cmd+[ ineffective. Palette close now explicitly releases editing; Settings entry releases only the hidden Home field. A shared modal mount releases background editing once, after native focus capture and before delayed auto-focus; native restore remains authoritative. Second integrated replay confirmed these focus changes. Proxy invalid port 00 incorrectly enabled Add; the rebuilt predicate matches existing u16 > 0 submit validation. Native drafts confirmed 00/70000 disabled, 80 enabled and Escape cancellation.

The coordinator also observed article overflow when a side panel narrows chat. Shared bubble/context-menu shrink constraints are repaired; second replay confirmed narrow article reflow and ordinary text/status remaining visible. No public Rust API or business contract was changed.

The first UI batches did not send messages, create groups/circles, change security,
clear media or initiate device transfer. Later explicitly authorized display
fixtures are recorded below. Temporary appearance changes were explicitly
authorized and restored before rebuilding.

## Checks for the integrated UI batch

- `cargo fmt --all -- --check`: passed for the final UI sources.
- `CARGO_INCREMENTAL=0 cargo clippy -p mixin_desktop_ui --all-targets -- -D warnings`: passed after correcting an integration-only needless return.
- `CARGO_INCREMENTAL=0 cargo test -p mixin_desktop_ui --lib`: 23 existing tests passed again for the final UI sources (0 failed, 0 ignored).
- Native macOS package/signature verification passed for all three integrations; the debug linker reports the previously known compact-unwind size warning.
- `git diff --check`: passed. No new UI test suite was added.

## Shared motion and focus contract

Keep the existing engine-clock motion policy: modal entry 180 ms / exit 120 ms,
side-panel entry 180 ms / exit 140 ms, popup entry 100 ms / exit 80 ms,
control feedback 120 ms. Reduced motion removes translation/scale and limits
opacity timing to 80 ms. Do not animate content height to mask missing constraints.
Closing state stops interaction immediately; Presence retains only presentation
until exit completes. Native modal RequestClose and focus restoration remain the
shared contract. UI fixes must not bypass this by enabling immediate native close.

Second integration replay passed quick-search-to-reader Escape, member-search-to-
profile one-Escape and restored focus, Settings releasing hidden Home search,
mouse-open Select Down/Return selection and Home/Return restoration, proxy draft
validation, narrow article wrapping, and inactive shared-tab visibility. Pure Tab
Select activation and IME preedit are not established by these observations.

## Next page batch

The main chat header/reply composer, conversation previews, primary search and
ChatInfo expiry display have now been integrated and operated as recorded below.
Keep the user's revised priority on main features and UI. The bot-chat header
Open-app shortcut and mention-picker bounds are now integrated and operated.
Other existing main-page states should be evaluated before returning to editor
details. External bot content and unavailable unread/role fixtures retain their
separate coverage limits.
Narrow Settings navigation has a recorded native pass. Main narrow focus now has
a reproduced regression and rebuilt native pass recorded below. Minimum-size bounds, IME preedit
and fixture-dependent states remain explicit gaps; do not repeat accepted cases without a new change or concern. Treat unavailable
fixture-dependent roles/media/login states as explicit conditional UI coverage,
not business-chain failures. Keep implementation delegated and native replay owned
by the coordinator. Keep public Rust interfaces and verified core untouched.

## Current follow-up UI batch

Native observations above produced bounded tasks for conversation menus, in-chat
search, QR, selection/forwarding and contact sharing. Message context-menu entries
rendered correctly; only the conversation menu is blank. Search query/filter and
result navigation already work in the observed fixture and must be preserved.
The temporary bool-only diagnostics identified missing Circles context. They were removed before the integrated menu build. Native replay caught and repaired a second row-owner failure; source/build checks alone did not establish completion.


## 2026-11-07 follow-up integration result

The coordinator rebuilt and operated all repaired current-fixture states in this
batch: group/direct conversation menus, zero-circle membership, in-chat search,
short message QR, narrow selection toolbar, forward recipient picker, contact
sharing, emoji search/categories, empty sticker/store panels, unconfigured GIF,
and custom expiry validation/unit navigation. Each exact state is recorded in
the queue above; conditional populated media, roles and large datasets remain
unverified.

The integration found three failures that source reports did not establish:
missing Circles owner context (including a row regression in the first fix),
disabled native GIF input still accepting text, and Select keyboard navigation
being intercepted by an active quantity editor. All three were repaired and the
coordinator repeated the failing interaction on rebuilt native bundles. Home now
owns the single existing Circles handle and passes it through existing UI
commands; no Rust business/public API was changed. Temporary diagnostic lines
are absent from the sources.

Final formatting, strict UI-crate Clippy, macOS packaging/signature verification
and the 23 existing UI-crate library tests passed. The debug linker retains its
known compact-unwind size warning. The complete workspace suite was not rerun
for this UI-only batch. No new UI tests were added.

The final app is left running on the ordinary group chat with t2 signed in, no
open popup or draft. System theme/system language/font offset 0/avatar on/ID off
were restored. This pass sent no message, changed no expiry or membership,
added/imported no sticker, and created no circle/group. No commit or push.


## Settings, category and profile follow-up

The coordinator operated the rebuilt native UI for the following bounded changes:

- The shared language picker releases active profile editing before its popup
  opens. Name editing > Language > first Down/Return selected English;
  Home/Return restored System. The original profile name was restored without
  pressing Save. The unused old no-matches body key was removed after strict
  Clippy correctly reported its last call site had disappeared.
- Zero-byte storage categories cannot be selected, and Clear remains disabled.
  The predicate uses the current screen usage rows and existing selection; no
  storage business contract changed. Native clicking the disabled Photos row did
  not select it. Positive-byte clearing was not exercised.
- Contact/bot category empty copy identifies its actual scope. Bots > Unread >
  Clear filters retains Bots and removes only Unread; All restores both rows.
  Group/stranger populated categories and compact rail names were inspected.
- Message selection uses native Checkbox semantics and localized names. Native
  checked values and selection counts followed 1 > 2 > 1; Cancel removed the
  selection UI without changing messages.
- The chat-avatar user profile host alone uses Flutter's 340pt dialog width and a
  bounded scrolling body. The full article reader remains wide. In the shorter
  viewport, QR body scrolling reaches Copy while its header stays fixed; Escape
  closes both QR and article reader after the profile-edit regression sequence.

### Native profile focus regression

A repeatable CUA function, `profileEditBackEscapeLoop()`, starts at the bin
profile card, opens the Add-contact nickname draft, verifies initial input focus,
clears the name and asserts disabled Save, returns to Profile, presses Escape,
and checks whether the modal Close control remains. It performs no Save and
cleans up the failed modal with Close. The old build produced RED in 3.3 seconds;
the final build produced GREEN in 2.85 seconds. Repeating it after the complete
Shared-apps > bot-profile > Back > apps > Back sequence also produced GREEN.

A fresh-process contrast established that an untouched Profile closes with one
Escape, while EditName cancellation alone was sufficient to reproduce failure;
message selection was not required. A global Blur before switching the dynamic
child did not fix this. The resolved SDK renders/removes the old Input before
shipping host commands, so a reference to that departing input would not be a
reliable target. The final app fix instead focuses the existing, still-mounted
Back button after returning to Profile. It adds one local ref, with no global
keyboard handler, delay, registry, SDK modification or second business state.
The original modal RequestClose/Presence behavior remains authoritative.

The local history now retains the existing user ID and destination together, so
returning from an app's bot profile restores the source Apps page. Child lists
own their loading indication; the profile loading text is limited to the profile
and name form. No synthetic UI unit test was added: the failure depends on the
real macOS editor and modal routing, which the targeted live loop exercises.

Final `cargo fmt --all -- --check`, strict UI-crate Clippy, the 23 existing library
tests and macOS package/signature verification passed. The known debug linker
compact-unwind size warning remains. No full-workspace test result is claimed
for this UI-only follow-up. The final app is left running in the normal group
chat, with no modal or draft, t2 signed in and System language restored. No
message/contact/storage/security/transfer mutation, commit or push occurred.

## User lookup and link preview follow-up

Two fresh implementation agents owned `create/lookup.rs` and
`links/{view,send}.rs` respectively. The coordinator rebuilt the native bundle
and operated the exact form/preview states recorded in the queue.

The lookup form previously used nearly the full viewport despite having one
field, and did not focus that field initially. Its form now uses natural height
with a bounded scroll body; the successful profile uses the existing bounded
340pt profile presentation. Native replay covered not-found feedback, retained
input, clearing feedback on correction, Return submission, visible busy state,
t1 profile, Back retaining/focusing the query, and one-Escape cancellation.

The text-link preview previously used the same tall card for all payloads and
showed duplicate footer cancellation. Its short/error states now use natural
height; the payload alone scrolls at a bounded height. Native replay scrolled a
30-line preview to its final line while keeping the title, recipient chooser and
footer visible. The existing Home-search Open-link entry was used; no product
debug entry or external browser was added.

The coordinator also reproduced a stale editor after choosing a recipient:
query > choose t1 > preview > Escape failed to close. All chooser exits now focus
the one persistent Cancel button. The same loop passed on the rebuilt bundle;
query > header Back > Escape also passed. Selecting a recipient updates only the
local preview. Send was enabled but never activated. Invalid base64 displayed a
compact error card, and Escape restored Home search.

Formatting, strict UI-crate Clippy, all 23 existing UI library tests, native macOS
packaging/signature verification and `git diff --check` passed. Shared public
Rust APIs and business implementations were not changed. Full-workspace tests
were not rerun for these three presentation files.

An additional appearance inspection confirmed Light theme and pointer dragging
the font slider to +4 updates the preview. Both were restored to System/0;
language remained System, avatar on and ID off. Clicking the font thumb followed
by Left did not update the value or show focus; a separate bounded task is
investigating the existing Slider keyboard contract. Native Zoom expanded the
window, but edge dragging did not establish a smaller viewport. Compact bounds
remain unverified and must not be inferred from this larger-window inspection.

## Settings keyboard and article follow-up integration

Actual Tab traversal superseded the earlier inconclusive Tab observation. It
revealed hidden Home conversation rows behind Settings: Return on a visually
absent t1 row left Settings and opened its chat. Home now conditionally mounts
that presentation branch. Home query, draft, selection, bounded-window state and
subscriptions remain authoritative. Search-local tab/page/results and native row
viewport are recreated; no extra route, retained native host or scroll API was
introduced. Native replay confirmed visible rail > Profile > Security keyboard
order, Return activation, absent Home search/rows in AX, query/result retention
after Back, normal group opening and the group context menu after remount.

Tab also established a separate Slider failure that mouse clicking could not:
the first Right changed 0 to 1, then saving disabled the thumb and released focus
to the window. The existing Slider now stays enabled after preferences load.
The existing Appearance owner retains only the latest completed font target
while one write runs; older readback does not overwrite a newer preview/target.
No SDK, keyboard-handler or public business API was changed. Native replay kept
Slider focus through Right/Right/End/Home/Right/Right and values
1/2/4/-2/-1/0. A rapid burst ending at 2 survived a category roundtrip. Keyboard
restored 0, and a later Settings entry displayed 0 again.

The article editor now initially focuses its existing multiline input and routes
field Escape through the existing guarded cancellation. Native replay covered
direct typing, an independent empty three-space draft remaining unsendable,
Markdown enabling Send, one editing Escape closing, and an empty reopen. Send
was never activated. IME composition cancellation is guarded in code but is not
claimed as a completed native preedit test.

Final formatting, strict UI-crate Clippy, all 23 existing library tests, macOS
package/signature verification and `git diff --check` passed for this integrated
batch. No new UI tests or full-workspace test result is claimed. The known debug
linker compact-unwind warning remains. The app is running on the ordinary group
chat with t2, no popup/draft and original appearance preferences restored.

### Media display fixture preparation

A temporary primary-only helper and non-sensitive PNG/12-second synthetic Opus
assets were prepared outside the repository. The first read-only preflight
refused 12 old CREATE_MESSAGE jobs. A focused source check established these are
SessionAck jobs, not SENDING_MESSAGE: with primary_session_id=None the existing
runner returns before reading, sending or deleting them. The coordinator
independently checked that dispatch and early return. No Signal database copy,
queue deletion, identity reset or login replacement was needed.

The helper's revised preflight allows only those inactive SessionAck rows and
still requires zero other jobs, SENDING/PENDING messages, transcript uploads and
flood messages, matching saved identity and no other open primary database.
It passed. Under the user's existing explicit authorization, one marked text,
two marked 1200x800 PNGs and one 12-second synthetic Opus were sent t1 > t2 as UI
display fixtures. The one-shot manifest completed; accepted IDs are:

- Text: `b7c38da4-4be5-4fc8-a30f-b9b06f86339c`.
- Image A: `47f84740-6949-4338-b622-3c2c9b95090c`.
- Image B: `f1e2e22e-5713-4a76-89dd-11867b1cb0e7`.
- Voice: `d0e591e7-5307-4b5e-9bed-c247e5b332f3`.

The helper shut down its primary. The existing t2 desktop session was retained.
The coordinator then observed all four fixtures in the current native t1 chat;
the primary send result alone was not counted as UI acceptance. No READ/delivery
chain or bidirectional protocol acceptance was performed. Normal primary runtime
ACK/key/backlog behavior was not replaced by a special implementation.

## Gallery, playback and positive-storage integration

Separate implementation agents owned gallery display and playback-bar controls.
The coordinator supplied native failure evidence, reviewed the bounded changes,
rebuilt the bundle and operated the repaired states. Storage selection and avatar
cropping already behaved correctly in the available positive-data fixture and
required no implementation changes.

The gallery previously changed its zoom label without enlarging the image. The
resolved macOS Scroll implementation does not implement its native zoom props.
The app now derives an explicit image/rotated canvas size from the viewport and
existing zoom/rotation. Two scroll axes and local drag offsets handle panning.
The caption reads the selected image rather than the original modal payload.
Only `chat/gallery.rs` and the image-caption branch in `chat/message_detail.rs`
changed; Save/Copy/Forward still use the existing selected-image actions. Native
replay covered the exact geometry, caption, navigation and Save As cancellation
states in the queue above.

The playback bar had an accessible slider with no visible track or thumb. Its
existing account-scoped snapshot now also drives semantic slider styling,
elapsed/total time and the existing 1x/1.5x/2x media command. No second playback
state, persistent speed setting or new business method was added. Pause, seek,
resume, source navigation and close were replayed on the rebuilt native bundle.
Bubble Stop-reset remains the existing Flutter behavior.

Incoming fixtures also exposed a conversation-window refresh defect while
Settings was open: the anchor moved from index 0 to 1, and the nonempty final
page contained only the group. Log entries at 11:58:15 show two rows at offset 0
followed by one row at offset 1; subsequent refreshes retained that incomplete
page until opening t1 from search. The existing tail-backfill condition now also
handles a short page, rather than only an empty page. The change is confined to
`conversation_list/window.rs`, reuses the existing count/requery path and retains
the existing paging bounds. Both rows display in the final bundle. The exact
incoming-message reorder while Settings is open has not been replayed after this
change and is not claimed as native acceptance.

Final formatting, strict UI-crate Clippy, all 23 existing library tests and macOS
package/signature verification passed. The known debug compact-unwind linker
warning remains; the full workspace suite was not rerun for this UI-only batch.
No new UI tests, SDK changes or public Rust API changes were introduced.

The final app is running in the ordinary group chat with t2 signed in, no popup,
draft or playback. Original System theme/System language/font 0/avatar on/ID off
remain restored. Avatar Save and the final destructive cache-clear action were
never activated. No commit or push occurred.

## Primary messaging UI integration

The user redirected this pass toward main features and UI and paused further
image-editor work. Three separate agents owned primary chat, Home search, and
conversation previews. The earlier ChatInfo expiry-value fix was integrated in
the same bundle. The coordinator reviewed their scoped diffs, built the native
bundle and operated the exact primary-page cases in Batch 0 above.

The main header uses existing identity fields and the existing badge component.
The avatar/name region and Info icon share one local toggle; Search toggles the
same existing side-panel state. Direct identity display follows Flutter header
semantics independently of the message-sender ID preference. No header expiry
query or new navigation owner was added.

The reply regression had a native RED result before implementation: the quote
appeared but focus stayed on the window and direct paste entered nothing. The
existing composer is now focused when a quote is present. Rebuilt native replay
focused its actual editor and accepted an immediate paste. The local draft took
precedence in the conversation row; clearing it restored the new sender/article
excerpt. No message was sent.

Conversation previews reuse current row metadata, existing system-message rules,
and the already-used Markdown parser for a bounded plain-text article excerpt.
Draft precedence and delivery/unread presentation were preserved. The current
group article and direct voice cases were observed; unrepresented system/pin/
recall latest-message states and mention-name resolution remain explicit limits.

The search header derives unread scope copy from the existing filter, and its
Clear action changes only the query and restores the existing input focus.
Contacts/Unread > query > Clear > immediate new input passed on the native bundle.
All filters and drafts were restored after checking. ChatInfo displayed the
existing one-week expiry in both the direct and disabled ordinary-member group
rows; no setting was submitted.

Final formatting, strict UI-crate Clippy, 23 existing library tests, macOS
packaging/signature verification and `git diff --check` passed. Clippy caught one
old Conversation enum key whose final consumer disappeared; that unused mapping
was removed without changing the ARB translations. The known debug linker unwind
size warning remains. No full-workspace suite or new UI-test result is claimed.
Tracked pre-existing changes retained the same diff checksum during this batch.

The app is left running on the ordinary group chat with t2 signed in, All chats,
Unread off, an empty search and no reply/draft/modal/playback. Original appearance
preferences were not changed. No business-chain acceptance, commit or push.

## Bot header and mention-picker integration

Two new feature agents separately owned the bot header shortcut and the existing
mention picker. Only `chat.rs` and `composer/mentions.rs` changed in this batch.
The coordinator reviewed both diffs, rebuilt the bundle and operated the cases
recorded in Batch 0. The existing profile Message navigation was used to open
Lucky Coin's empty direct conversation; no message was sent to the bot.

The bot shortcut reads the current header owner and passes the current
conversation ID to the existing LinkNavigation route. It adds no direct URL
bypass, business query, public API or window owner. Integration caught an
unsupported ButtonElement tooltip method; the final code follows the existing
sidebar View/semantic-button composition supported by the resolved NSH 0.4 SDK.
The native window, More menu, cancellation and closing were observed. The remote
Lucky Coin page displayed an error and an existing phone-open handoff appeared
behind it. These were closed without further actions; this is evidence for the
native entry/shell, not the external application or its authentication flow.

The mention failure was reproduced before repair: a no-match query removed all
rows but retained a 128pt gray rectangle above the composer. The existing owner
now derives one visibility value for both presentation and Input::picker. Empty
results occupy no space; a single result uses the actual 48pt avatar/padding row
height, and larger lists retain the original 128pt cap. No new owner or query was
introduced. Native replay passed no-match collapse, single-row bounds, three
members, Up/Down selection, click insertion and draft cleanup. Enter/Send were
not activated. Quote-source highlight and returning to the latest voice message
also passed using existing messages, without changing their implementations.

Final formatting, strict UI-crate Clippy, all 23 existing library tests, macOS
packaging/signature verification and scoped whitespace checks passed. The known
debug linker unwind-size warning remains. The full workspace suite was not
rerun and no new UI tests were added. Pre-existing tracked diff checksum remains
unchanged. The app is running on the ordinary group chat with t2 signed in and
no draft/reply/popup/playback. Appearance preferences were unchanged. No commit
or push.

## Conversation names and link-feedback integration

Two new feature agents owned `conversation_list.rs` and `links/view.rs`. The
coordinator reviewed their scoped diffs and actual SDK cancellation contracts,
then rebuilt and operated the native cases recorded above. No business/public
API changed. The mention projection is bounded to the existing conversation
window and reuses `UserClient::replace_mentions(Vec<String>)` once per displayed
batch. It stores only row ID/original/display text. Request versions, original
source matching and effect-owned task cancellation reject stale replacements.
Unknown identities keep the existing business-layer replacement behavior.

Native replay confirmed the exact original mismatch is repaired: after choosing
t1 in the group mention picker, both the editor and its row display the name,
rather than the row exposing the wire identity number. Settings > Back retained
the draft/name after unmount/remount. Ordinary replacement text and clearing were
also operated; no message was sent.

The old unsupported-link card was reproduced through Home search with
`https://mixin.one/ui-layout-preview`: a tall card retained a large empty area
below its QR and a full-width Close. The rebuilt card follows its content height,
keeps the complete QR and Copy action, and fixes Close at the lower right. Escape
restores the Home query and focus. `https://mixin.one/messages` and a nonexistent
local message exercised the existing compact error state. An invalid app target
read lookup showed the new short loading/spinner card and was cancelled with
Escape. No external web page, signing, authorization, payment or send was used.
The loading request did not visibly reopen a popup after cleanup.

The search entry recognizes HTTP/HTTPS URLs; raw mixin:// input does not display
Open link there. The mixin.one HTTPS aliases are internally routed by the app.
An earlier proposed raw-scheme fixture was corrected before native acceptance;
no CLI URL-open bypass or product debug entry was added.

Final formatting, strict UI-crate Clippy, all 23 existing library tests, native
macOS packaging/signature verification and scoped whitespace checks passed. The
known debug linker unwind-size warning remains. No new UI tests or full-workspace
test result is claimed. Existing tracked diff checksum stayed unchanged. The app
is running on the ordinary group chat with t2, empty draft/search and no popup,
reply or playback. Appearance preferences were unchanged. No commit or push.


## Main responsive navigation and ChatInfo integration

The coordinator reproduced the narrow Settings defect before rebuilding: at a
1276px screenshot width, opening Preferences showed only Edit profile, with no
visible category list. Back exited Settings. The integrated route uses one local
detail flag and the existing category enum; business state and APIs are unchanged.
Native replay after the signed package rebuild established:

- Narrow Preferences initially shows all eight categories; Appearance and
  Notifications open their own details, with no hidden Settings category controls
  left in the detail AX subtree.
- Detail Back returns to categories; category Back returns to the conversation
  list; reopening Preferences starts at categories and retains category selection.
- Native Window > Zoom expands to the existing two-column layout and preserves
  Appearance; a second Zoom restores the narrow detail. Theme/language stayed
  System, font offset 0, avatars on, IDs off; no preferences were changed.

The same native pass caught a separate real focus defect: focusing HomeSearch,
then opening t1 in narrow mode hides the list but retains its native editor focus.
Pasting Local UI focus changes that hidden query. Returning to the list reveals
it. The coordinator cleared the query and assigned a fresh focus repair. Its later
rebuilt result is recorded separately below.

ChatInfo source integration in this package also adds these existing-contract
states, without a public API/core change:

- Creating a circle from this chat now uses the returned ID to add this chat.
  Partial failure retains that ID and offers Retry adding chat rather than
  creating another circle. Owner-scoped waiting releases local UI state while
  the existing committed write continues when the page is left.
- Exited groups (`GROUP && status == 3`) show Delete group with its matching
  confirmation and hide member/mute/expiry actions. Existing conversation-change
  navigation clears the selection after confirmed deletion; no second close
  controller was added.
- Muted chats show their local expiry date and time under Unmute.

Coordinator native checks covered the narrow direct/group Info layouts, current
non-exited group actions, disabled one-week group expiry, direct circle form empty
and whitespace-disabled Create, a valid draft enabling Create, and Back after
clearing the draft. No circle was created; no mute, exit, clear or delete command
was activated. Created/partial-failure circles, exited groups and muted deadline
states remain CONDITIONAL, not native VERIFIED.

Validation for the four-file integration: cargo fmt --all -- --check, strict UI
Clippy across all targets, 23 existing UI library tests, signed macOS package
build/verification and git diff --check passed. The existing debug linker compact
unwind warning remains. The workspace-wide suite was not rerun for these UI-only
changes. The separate focus repair was subsequently rebuilt and operated as recorded
below.


## Responsive focus follow-up result

A fresh bounded agent changed only Home, Chat and the existing Input facade.
Home releases Search when its column becomes hidden. Chat uses the computed
layout/destination and one pending-focus signal: it blurs the hidden editor,
waits for initial editability when needed, focuses a visible composer once, and
keeps Inspector Search's existing autofocus. The new Input::blur simply forwards
to NSH 0.4.0 EditorSession::blur. Controllers, drafts, timelines, SDK and public
business interfaces are unchanged.

After a second signed package rebuild, the coordinator repeated the original
native failure and its adjacent transitions:

- Narrow Search focus > t1 row now reports composer focus; Local UI focus appears
  in the visible composer and draft preview, while Home query stays empty.
- Opening Info releases composer focus to the window. Attempted paste while Info
  covers Chat does not alter the original draft. Back eventually restores the
  composer, retaining the original draft; a subsequent paste appends restored.
  A paste batched directly with Back during the Presence exit did not append;
  post-exit focus restoration is verified, input during exit is not claimed.
- Search takes initial focus, NSH remains in that field and returns the existing
  fixture message. Clearing the query and Back restores the empty composer.
- In wide mode, focusing HomeSearch then native Window > Zoom to narrow transfers
  focus to composer. Resize UI focus appears there, with HomeSearch still empty.
- The coordinator cleared all test drafts and queries and restored wide t1 chat.
  No message was sent, no playback started and no popup remained. t2 stays signed in.

The final three-file follow-up passed cargo fmt --all -- --check, strict UI
Clippy across all targets, all 23 existing UI library tests, signed native package
build/verification and git diff --check. No UI tests were added. The original
tracked baseline diff hash remains b069f49b4dd9d484786d0777c23c715414b4ec28906a1d772d8d094286680b06;
NSH and documentation changes are in the existing untracked migration tree.


## Main compact-surface and motion review

Fresh bounded agents reviewed the remaining common create/lookup/Info surfaces
and existing motion. The coordinator also operated the current wide Search/Info
switches and confirmed that neither retained a selected header state. Only the
existing Chat buttons were assigned a four-line selected binding and accent fill;
no state owner, animation layer, SDK or business interface was added.

The current source review did not establish another compact layout bug. The next
concrete native targets are these original product surfaces at 384x480pt:

1. New circle > chat selection: fixed header/footer, scrollable list, selection
   count, Back retaining the local name and selections, then Cancel. Do not Create.
2. Lookup t1 > profile: 340pt intended width under the shared maximum width, body
   scrolling to all available actions, Back retaining the query, then Cancel.
3. Direct Info > custom expiry: quantity/unit columns within the 320pt content,
   independent body scroll, Select open/cancel, then Back without applying.

These are CONDITIONAL viewport checks, not defects inferred from fixed intended
sizes. Current create cards already retain maximum width/height and compressible
bodies. The chat font preference affects selected chat surfaces rather than all
settings/create copy; do not claim a global large-font check from that preference.
Native edge-drag/Window Zoom attempts in this pass did not reach the intended
minimum size, so wide observations do not close these viewport gaps.

Existing Presence remains: native animation values, interrupted transitions from
sampled values, generation-protected disposal, immediate pointer/AX exit and
Reduced Motion without translation/scale at no more than 80ms. Loaded Settings
category changes remain immediate as prescribed by the design guide. No artificial
loading or full-page slide was added to satisfy motion cosmetically.


## Inspector toggle native integration result

The coordinator inspected the four-line Chat patch against its before snapshot,
then rebuilt and operated the actual native app. Search now exposes AX selected
and retains accent fill after the pointer moves to the search field. Switching
to Info clears Search and selects Info. Opening its Circles page retains Info's
selected fill with the pointer inside that page. Clicking the same Info button
closes the nested panel, clears both selections and returns focus to composer.
No draft, query, message, preference or account data was changed.

The fill uses the app's existing icon-button transition and semantic accent token.
It introduces no state copy, lifecycle layer, business API or custom animation.
Formatting, strict UI Clippy across all targets, all 23 existing UI library tests,
signed macOS packaging/verification and git diff --check passed. The final app
is running as PID 74971 with t2 signed in, ordinary wide t1 chat and no open panel.
The debug compact-unwind linker warning remains unchanged. Windows/Linux and the
three minimum-size checks above remain unverified; no whole-scope completion is
claimed from this one header-state fix.


## Main ChatInfo follow-up regressions

The coordinator used the existing Lucky Coin conversation, without creating a
conversation or sending a message: Info showed name/ID but no description;
Profile showed its nonempty bilingual biography; Back to Info removed it again.
The Info Chats section also had only media and circles, with no direct search.
Both are actual native regressions against the referenced Flutter page, not
inferences from an empty profile. The assigned repair is limited to conversation
State loading/projection and Info composition.

The coordinator independently confirmed that AccountClient.user().user_profile
only queries the existing local user DAO; refresh_user_profile is the separate
network operation. A biography String belongs to the mounted Info projection,
not a second durable profile source. It can join the existing local detail/rows/
members query with the same owner cancellation and request version.

Minimum-size execution remains unverified. Further native attempts found that
an outer-right coordinate is outside the window; inner edges did not resize.
Double-clicking the existing sidebar titlebar changed height but kept full width.
A fresh source audit found native Resizable retained, keep_size=false, one-time
frame restoration and no custom resize interception. No SDK, native geometry
hook or app resize workaround was added without a demonstrated implementation bug.

A separate user question is pending about creating independent t1/t2-only test
group/circle data to expose management and populated circle states. Nothing was
created while that answer was pending; existing test-message authorization is
not being treated as consent for extra account-data setup.


### Biography native pass and search return correction

The first rebuilt ChatInfo package visibly shows Lucky Coin's existing bilingual
biography in Info, matching Profile. The Search row enters the existing Search
panel with autofocus and the correct header selection. Native Back then closed
the panel entirely, which exposed an additional return-path mismatch rather than
a completed search-entry pass.

The coordinator independently checked Flutter ChatSideNotifier.openDestination:
Search is appended after Info, and Back pops to Info. The next bounded repair may
add one transient Info-origin flag in existing UI navigation. It must distinguish
Search Back/header-toggle from result activation: Back returns to Info; choosing
a result still closes the panel and locates the message. Conversation identity,
message jump and Settings changes must clear the origin. No general navigation
stack, business API or core change is authorized for this repair.


## ChatInfo biography and search return final integration

The final repair touches five existing UI files: conversation loading/composition,
UI navigation, Chat callback wiring and Search result activation. The coordinator
reviewed both original two-file diffs and the three additional navigation diffs.
No public Rust API, core query, persistent profile copy or general navigation
stack was introduced. The one private Info-origin flag is cleared through stable
conversation/target/settings scope and panel changes; same-ID row refreshes do not
change the computed scope.

Coordinator native replay of the signed final package proved:

- Lucky Coin Info retains the same nonempty bilingual biography already seen in
  its Profile. The current t1 Info has no leaked bot biography.
- Info > Search focuses Search and updates the selected header entry. Back returns
  to Info with its biography and selected Info entry. Repeating with the same
  header Search button also returns to Info.
- Opening Info-origin Search in Lucky Coin, selecting t1, then ordinary header
  Search > Back closes to t1 Chat rather than returning to the previous Info.
- From t1 Info > Search, NSH finds the existing marked text fixture. Clicking the
  result closes the entire side panel, positions the message and shows its themed
  highlight. Ordinary Search > Back afterward closes to Chat, so the jump has not
  retained the Info origin.
- The coordinator restored Latest, with empty query/draft, no open panel and no
  playback. The final native process is PID 927, t2 remains signed in. No message
  was sent and no profile/group/circle/security value was written.

Both integration rounds passed formatting, strict UI Clippy across all targets,
23 existing UI library tests and signed macOS packaging/verification. Final
snapshot whitespace checks and git diff --check passed. The original tracked
diff hash is still b069f49b4dd9d484786d0777c23c715414b4ec28906a1d772d8d094286680b06.
The known debug linker compact-unwind warning remains. No new UI tests were added;
workspace-wide tests were not rerun for this UI-only scope. Minimum-size viewports,
nonempty group announcement, live biography updates and ordinary same-ID message
refresh during an Info-origin Search are not claimed as native coverage.

Remaining direct-entry review found Shared apps accessible via Profile but lacking
Flutter’s direct Info shortcut. The existing shared_apps operation is networked,
so any shortcut must load on explicit entry rather than joining Info’s local
refresh. Reusing the current profile/apps controller is preferred over copying its
list/history. A separate stranger nickname operation difference is still under
source assessment; Add contact must not be used as a substitute for nickname-only
editing. These findings remain outside the completed biography/search pass.


### Next ChatInfo entry work

The complete source mapping also identifies direct-entry differences now recorded
rather than conflated with absent business features: share contact, shared apps,
groups in common, developer and relationship/report actions are reachable through
Profile. Group editing correctly combines photo/name/announcement in its editor;
role-dependent management is already implemented.

Prioritize Shared apps next. The coordinator verified the public
UserClient::local_shared_apps method and its pure local favorite-app DAO query.
This permits the existing Info query to project the nonempty preview without
network work. Do not settle for an always-visible placeholder if the local preview
can preserve Flutter's empty/nonempty contract. Explicitly entering the Apps page
can reuse the existing network-backed apps controller; returning should refresh
the local preview. A private initial Destination::Apps can reuse that controller's
list/profile history, with Back from its initial root returning to Info and bot
Profile Back returning to the Apps list. Avoid duplicate controllers, two headers,
or a second selected-app state.

After that, assess the existing public nickname Update contract before adding the
stranger Edit name operation. Current Add contact is a relationship mutation and
is not equivalent to nickname-only editing. Copy conversation URI from Info is a
lower-priority direct-action difference. Neither is part of the current completed
biography/search native pass. The independent group/circle fixture question is
still pending; no such data has been created.


## Shared apps and nickname-only editing batch

The coordinator refreshed the existing bin Profile > Shared apps page and observed
its four entries (Lucky Coin, TIGA-safe, Mixin Card, Wheat beer). Normal Profile >
Message navigation opened an empty bin conversation; this added a fourth local
conversation row but sent no message. Its current Info shows the existing biography
and Search row but no Shared apps entry, confirming the missing direct preview
with nonempty source data. No real contact relationship was changed.

SharedApps implementation is limited to the existing three conversation UI files:
local query projection keeps at most four preview items; empty hides the row;
Apps begins through the existing profile controller, with the same list and bot
profile history. No separate selected-app/controller state or remote Info query.

A separate contract audit established that nickname editing needs a real Rust
command: the SDK's existing UPDATE enum cannot carry full_name, and Profile
EditName previously submitted ADD. The new rename_contact command sends UPDATE,
persists the service-returned profile under the existing mutation guard, and emits
the current conversation event. It preserves the old public enum constructor.
Two local HTTP regression tests check exact POST/path/payload, a STRANGER response,
and API error propagation. Native acceptance will inspect drafts and cancellation
only; it will not rename users or add contacts to prove the interface.


## Shared apps and nickname integration result

The coordinator reviewed the SharedApps three-file patch and the subsequent
nickname patch against separate before snapshots, preserving their shared UI
controller work. The final signed package was operated directly:

- Bin Info displays four overlapping application icons. Its direct Apps page has
  one visible title/header and four readable rows. Lucky Coin Profile Back returns
  to Apps, then root Back returns to Info with the same preview.
- T1 Info hides the empty preview. The previous Profile > Shared apps path still
  displays its empty state and Back returns to Profile rather than skipping it.
- Direct Info > Edit name preloads mixtest-t1 and focuses the field. Empty and
  whitespace-only input disable Save; NSH UI nickname draft enables it. Back
  discards that draft, and reentry displays the original name.
- Stranger Profile has separate Edit name and Add contact entries. Add contact's
  form has its own title and cancelling returns to Profile; Profile Edit name
  also returns to Profile. No Save was activated, so no nickname or relationship
  changed. Final ordinary t1 Chat has no open panel, draft, query or playback.

The SDK/core/API command is an actual new UPDATE contract, not a rename of Add.
The coordinator independently reviewed its exact HTTP payload, mutation guard,
returned-user persistence and existing event notification. The new HTTP fixture
asserts endpoint, UPDATE, full Unicode/quoted name, STRANGER decoding and API
error propagation without contacting a real service. The old public SDK enum
constructor remains compatible.

Full validation passed: cargo fmt --all -- --check; cargo test --workspace
--all-targets; cargo clippy --workspace --all-targets -- -D warnings; signed native
packaging/verification; git diff --check. All affected public API consumers compiled.
No UI tests were added. The existing debug compact-unwind linker warning remains.
The original b069 tracked hash necessarily changes for the three authorized
SDK/core/API files; their snapshot deltas were inspected. Excluding those three,
the protected existing tracked diff remains
8a60dfbf7bb3eb4795ba17f376aed6d9539ae4a18ab8a4fa655b06ee47d74553.
Actual service-side rename persistence, FRIEND/self/deactivated variants, more
than four apps, empty-to-populated preview updates and minimum-size bounds remain
unclaimed by this native pass.

## Independent test-data authorization

The user has now explicitly authorized new t1/t2-only NSH UI acceptance groups and
circles, retained without automatic deletion. Existing groups and real contacts
remain outside fixture mutation. A separate agent is preparing a reviewable,
strictly scoped temporary helper because mixtest has no create-group command.
The helper will create only a new t2-owned group with t1 as its sole other member;
the coordinator will create its test circle through NSH to verify that UI directly.
No group/helper write has run yet. Do not relink/release accounts, touch the live
desktop database or include a third participant. This supersedes the earlier
pending fixture question, not the draft-only boundary for real-user renames.
