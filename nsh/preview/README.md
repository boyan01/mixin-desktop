# Button design review previews

The inventory covers the framed `Secondary` buttons in `src/ui/button.rs` and
related primary commitments, pagination, disabled actions, forms and retry actions.
Ghost commands, pill actions, bot-defined custom button colors and native OS alerts
use different treatments.

There are 83 native scenes, including the original 21 scenes. Each scene represents
a page, dialog or existing component showcase. Errors belong to their owning scene;
there are no separate `*-error` scenes or invented title-plus-error page frames.
The full catalog contains 203 captures, grouped by scene and state. `--scenes`
selects a smaller review set, without rendering the rest of the application.

```text
[<] [create-group                 v] [>] [Select           v] [Play]
+----------------------------------------------------------------+
| Close                      New group                     Next  |
|                            2 selected                          |
| Search                                                         |
| Selected avatars (removable)                                   |
| Contacts                                                       |
| Selection mark, avatar, name; scroll to load more               |
+----------------------------------------------------------------+
```

## Open and capture

Run from the repository root:

```sh
# Native scene browser; select a scene, select a state/action, then Play.
python3 nsh/tool/preview.py run
```

The initial scene is `create-group`, following the reference's centered title,
leading close command and conditional header Next action. An empty selection has
no Next action or selected-avatar strip. Forwarding shares the visual parts but
stays immediate and single-select, with no selection marks or confirmation. Open another scene directly with:

```sh
MIXIN_PREVIEW_SCENE=profile-settings python3 nsh/tool/preview.py run
```

Stateful fixture forms suppress explicit auto-focus where possible. Live screens
keep their original focus behavior. Click an input manually to inspect its focus
treatment.

The interactive command and capture helper share the root `target/` with normal
Cargo tests and inherit the same development profile from `.cargo/config.toml`.
Generated lockfiles start with the repository's pinned dependency versions.
Use these helpers to avoid NSH's default entry-local target directories. Explicit
`CARGO_TARGET_DIR` overrides are respected. See [shared development builds](../README.md#shared-development-builds)
for the remaining source/feature differences that can require separate artifacts.

The capture helper resolves the entry itself. From the repository root:

```sh
python3 nsh/tool/preview.py list
python3 nsh/tool/preview.py capture profile-settings --action Error --label error --scheme dark --locale zh-CN --width 980 --height 820
python3 nsh/tool/preview.py capture create-group --action Next --label naming-dark --scheme dark --locale zh-CN --width 980 --height 820
python3 nsh/tool/preview.py catalog --label review-dark --scheme dark --locale zh-CN
python3 nsh/tool/preview.py catalog --label review-light --scheme light --locale zh-CN
python3 nsh/tool/preview.py catalog --scenes create-group message-forward --label picker-review-dark --scheme dark --locale zh-CN
```

`catalog` captures normal and available action states in one native run. It writes
`output/button-review/<label>/index.html`. Each page/dialog has one card with a
state selector. Changing it replaces the screenshot in that card. Click the image
to inspect it at full size. `scenes.json` records the captured states. `Reset` is
available in the native browser but is omitted from the catalog because it returns
to the normal state. Captures are ignored by Git; a failed run preserves the
previous completed gallery.

## Scene ownership

Paths are relative to `nsh/src/`. Fixtures reuse the production page/card builders.
Account queries and subscriptions stay in live entry points. Modal cards render
inline so the native scene toolbar remains accessible. These fixtures support
design review; service-backed save/send/download commands require a live account.

| Production surfaces | Owning scenes | Included states |
| --- | --- | --- |
| `create/contacts.rs`, `create/circle.rs`, `create/lookup.rs` | `create-group`, `create-chat`, `create-circle`, `create-lookup` | Selection, naming, infinite scrolling, command failure |
| `conversation/user.rs` | `contact-add`, `contact-rename`, `contact-profile`, `contact-groups`, `contact-apps` | Full forms/profile/lists and their failures |
| `sidebar/`, `conversation/circles.rs` | `sidebar`, `circle-rename`, `circle-conversations`, `conversation-circles` | Collapse/expand, rename, membership, retry |
| `conversation/info.rs`, `conversation/members.rs` | `conversation-info`, `group-edit`, `group-members`, `group-add-members`, `group-invite`, `group-expiry` | Full conversation panels, update failure, scrolling, Add/Save/Copy/Reset |
| `conversation/shared.rs` | `shared-media` | Media/posts/files tabs with loaded content and load failure |
| `conversation_list/`, `home.rs` | `conversations`, `mute`, `empty-filtered` | Loaded conversation list, load failure, command failure, mute and filter actions |
| `chat/` | `messages`, `message-selection`, `message-reply`, `message-qr` | Timeline and composer context, draft/history/unread/pinned failures, send restriction |
| `chat/message_transcript.rs`, `chat/message_content.rs` | `transcript`, `transcript-summary`, `messages-cards` | Reader/summary failures and failed outgoing forward inside the message list |
| `chat/gallery.rs`, `chat/video.rs`, `chat/message_post.rs` | `photo-viewer`, `video-viewer`, `markdown-attachment` | Actual viewer/reader chrome, playback controls and failed content |
| `chat/message_detail.rs`, `composer/sticker_store.rs` | `sticker-save`, `sticker-detail`, `sticker-store` | Save sticker, details/retry, subscriptions and store failures |
| `composer/` | `composer`, `composer-details`, `composer-fold`, `emoji`, `stickers`, `write-post`, `share-contact`, `image-editor` | Attachment preparation failure in editor, picker failures, send/cancel and media controls |
| `avatar/crop.rs` | `avatar-crop` | Crop/rotate/reset/cancel/done |
| `links/` | `link-invite`, `link-send`, `link-unsupported` | Link resolution failure, text/sticker/contact payloads, payload failure, recipient and send failure |
| `settings/` | `profile-settings`, `notifications-settings`, `storage-settings`, `appearance-settings`, `proxy-settings`, `proxy-add`, `clear-cache`, `sign-out` | Full settings pages with their own failures and confirmation dialogs |
| `inspector/`, `search/` | `pinned-messages`, `message-search`, `global-search`, `quick-search` | Loaded results, senders, result/sender load failure |
| `security/` | `screen-lock`, `passcode-setup` | Locked screen, privacy load failure, passcode form |
| `transfer/` | `transfer-settings`, `transfer-approval`, `transfer-progress` | Transfer progress, completion and failure in the same dialog |
| `login.rs`, `lib.rs`, `bot.rs` | `login`, `bot-ending` | QR sign-in, sign-in/startup failure, browser cleanup failure |
| `snapshot.rs` | `snapshot-inscription` | Full detail, document/text content, detail/content failure |
| Existing component/message showcases | Remaining original scenes | Shared button variants and message-level context |

## Review limits and observations

The generated gallery uses macOS native rendering, dark mode, Chinese, 980×820 pt.
Windows/Linux rendering, service-backed commands and interaction smoothness are
unverified. The capture toolbar and native title bar are cropped from gallery
screenshots. Some action recordings fail the strict end-frame stability check;
the static catalog and targeted static recaptures succeeded. Animation stability
has not been established.

The following baseline observations remain available. The redesigned contact/chat
pickers now use an explicit-height pill search field; live-account focus behavior
is unverified:

- The previous contact search displayed a horizontal blue focus line.
  [Close-up](output/create-group/issue-search-focus/start.png).
- The `create-lookup` fixture does not show its search input area.
  [Close-up](output/create-lookup/issue-search-field/start.png).

The old gallery is preserved locally under `output/button-review/before-grouping/`
for comparing the removed empty error frames with the corresponding full scenes.
The baseline before the quieter failure styling is preserved under
`output/button-review/before-quiet-errors/`.

Picker review captures cover dark and light appearances, plus a 480pt-wide native
window. Group creation exposes `Next` only after selection. Forwarding has no
selection marks or confirmation action. Both lists scroll vertically without page
controls. Recoverable failures use secondary text; load retries are inline text
links rather than framed buttons.


## Image viewer review

`photo-viewer` uses a conversation backdrop and the production viewer. `Zoom`,
`Edge`, `Rotate`, and `Error` are states of that same scene. The edge state zooms
and pans to the clamped top/right limit; failure keeps the sender and commands
while hiding the undecodable image. Native macOS component registration is shared
with the application so the input surface is present in CLI previews too.

```sh
python3 nsh/tool/preview.py catalog --scenes photo-viewer --label viewer-redesign-after --scheme dark --locale zh-CN
```

The captures show layout and bounded camera states, not physical trackpad gesture
smoothness. Pure camera tests cover pointer anchoring, per-axis bounds, resize,
rotation, invalid input and clamped input reversal. Windows/Linux wheel and pinch
input are not yet implemented by the app-owned adapter.

## Sticker detail review

`sticker-detail` renders the production dialog with a close control in its top
right corner. Album metadata and its Add/Remove command sit below the artwork.
The remaining stickers occupy one horizontal virtual list. Selecting a thumbnail
changes the large preview without sending a message. The initial strip position
keeps the opened sticker visible. Thumbnails are static posters; the selected
JSON sticker plays in the large preview.

`Animation`, `Not added`, `No album`, and `Error` are states of this same scene.
Album commands in this account-free fixture update fixture state only. The live
entry uses the shared album subscription API. `messages-media` also includes an
authored inline Lottie fixture, so message playback can be checked without an
account or network. Reduced motion, background lifecycle, screen lock and message
viewability still pause playback.

Targeted native captures are under `output/button-review/sticker-redesign-after/`,
`sticker-redesign-light/`, and `sticker-redesign-narrow/`. The baseline is preserved
in `sticker-redesign-before/`. These captures do not verify a live account's
network download or album subscription changes, nor Windows/Linux rendering.

## Video viewer review

`video-viewer` uses the production video surface over a conversation backdrop.
`Playing`, `Muted` and `Error` are states of this same scene. The normal state is
paused so its chrome can be reviewed. Its eight-second H.264/AAC fixture is made
from the existing local photo with a silent audio track; it needs no account or
network. Native progress/load events drive the transport in this preview too.

The photo and video viewers share `chat/media_viewer.rs` for sender identity,
header layout and icon commands. Video adds a dedicated seek/volume/speed strip.
Review both scenes after changing their shared chrome. Account-backed forwarding
and saving, and Windows/Linux playback remain separate validation requirements.

## Forwarded chat record review

`transcript` and `transcript-summary` render the production 600pt dialog shell,
centered header and leading close/back command. The full transcript reuses the
chat's real message rows in chronological order, without delivery checks or page
buttons. Its `Media` state adds local photos and a static sticker. `Error` remains
inside the same dialog. The summary uses the chat row module and the same bubble/text styling, with a quiet footer;
its data has no original timestamps, sender IDs or attachment assets.

Capture these two scenes individually when the scene menu's visible range does
not include both entries. Narrow/light captures review wrapping and dialog bounds.
Fixtures do not verify live account paging, attachment commands or nested Back
navigation. Windows/Linux presentation is unverified.

The local review gallery is `output/transcript-review/index.html`. It includes
before/after comparisons, dark/light and narrow captures, media/error states, and
close-ups of the text-wrapping and anonymous-avatar contrast fixes found during
this review.

## Composer reply review

`message-reply` now includes the real pending-reply preview and the production
composer frame below a chat timeline, sharing one persistent fixture input.
`Long`, `Image`, `Attachments`, `Text` and `Cancel reply` cover truncation,
thumbnails, file/caption composition, send presentation and removing the reply
without clearing the draft. Close and Escape also work directly in this fixture.
The account-free fixture does not send messages or perform a live history jump.

Review `composer`, `composer-details` and `composer-fold` after changing the shared
frame. The first scene stacks text, files, recording and recorded-preview states;
capture it at 1000pt height so the lower states stay visible.

The local gallery is `output/composer-reply-review/index.html`, with before/after
close-ups, light/narrow, long, image, attachment and cancellation states. The file
caption inset follow-up is in `output/caption-inset-review/index.html`: captions
align with the attachment strip while ordinary input keeps its tool spacing.
Real account sends, history jumps and recording remain unverified.
