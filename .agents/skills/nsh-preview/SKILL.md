---
name: nsh-preview
description: 'Check how NSH UI changes look in the browser preview of the scenes in nsh/src/preview (nsh/tool/preview_web.py), with native captures for what the browser cannot show. Use when changing, reviewing or debugging how an NSH screen or ui component looks or behaves, or when adding a preview scene.'
---

# NSH preview

`nsh/src/preview/` is a debug-only scene browser (`Preview` root, `nsh/preview/nsh.json`)
that renders production components over fixtures, with no account or database. The
same scenes run in a browser and natively. Scenes and their toolbar actions are
`SCENES` in `nsh/src/preview/mod.rs` and each scene's `ACTIONS`.

## Steps for a UI change

1. **Pick scenes.** Every scene that renders a component you will touch. When none
   does, add one (below) before editing, so the baseline exists.
2. **Start the browser preview** from the repository root:
   `python3 nsh/tool/preview_web.py` (port 8796 or `$PORT`; `.claude/launch.json`
   names it `preview-web`). Rust edits rebuild and reload the page.
3. **Look before editing.** Open each scene, run the actions the change affects, and
   note what it looks like now.
4. Make the change.
5. **Check after.** Each picked scene, in every state its actions reach. Add dark
   scheme (`prefers-color-scheme`), another browser language and narrower viewport
   widths when the change touches colors, copy or layout widths. Read exact sizes,
   spacing and colors from computed styles rather than judging a screenshot, and
   compare against the intent and `nsh/DESIGN.md`.
6. **Report.** Done when the hand-off names, per scene: the states checked, one line
   on what visibly changed, each defect found (changed or pre-existing) with whether
   this change caused it, and what the check does not cover (below, screens without a
   scene, Windows/Linux).

## Driving the page

- Switch scenes with `Next scene` / `Previous scene`; pick a state with
  `Preview action` and run it with `Play action`. Controls carry these accessibility
  labels.
- NSH buttons react to pointer events: use a real click or dispatched pointer
  events, since `element.click()` does nothing.
- Views are absolutely positioned DOM elements, so the accessibility tree gives text
  and structure, and computed styles give frames and colors.

## What the browser cannot show

Use native captures (below) when a change depends on one of these:

- Single-color SVG icons and shapes drawn with `currentColor` (bubble tails, stroke
  icons) render black in the browser ([longbridge/nsh#1466](https://github.com/longbridge/nsh/issues/1466)).
- Native fonts and text measurement, scrolling physics, animation smoothness, menus,
  windows and input methods.
- Audio playback, voice recording, animated stickers and video metadata: in the
  browser they report unsupported.

## Native captures

`nsh/tool/preview.py` drives the native preview on macOS and writes images to
`nsh/preview/output/<scene>/<label>/` (git-ignored). Requires the `nsh` CLI and
`ffmpeg`; flags in `python3 nsh/tool/preview.py capture --help`.

1. `python3 nsh/tool/preview.py capture <scene> [--action <Action>] --label before` on
   the unchanged code (or in a `git worktree` of the base commit), with `--scheme`,
   `--locale` or `--width/--height` as needed.
2. After the change, capture `after` with identical flags, then
   `python3 nsh/tool/preview.py compare <scene> before after`.
3. Look at `start.png`, `end.png`, `keyframes.png` and the compare stacks; capture each
   defect narrowly with `--crop x,y,w,h --label issue-<short-name>`. Embed the images
   in the hand-off with repository-relative paths.

Limits: frames arrive about every 100 ms, so they show an animation's path, not its
frame rate (smoothness needs a release build and `nsh run --fps`). A capture whose
start and end match fails as a no-op. The 40 pt toolbar (`TOOLBAR_HEIGHT` in `mod.rs`
and `preview.py`) is cropped off. `python3 nsh/tool/preview.py run` opens the native
preview interactively.

## Add a scene

1. `nsh/src/preview/<name>.rs` with `pub(super) const ACTIONS: &[&str]` and
   `pub(super) fn scene(actions: Actions) -> View`; register it in `SCENES`.
2. Render production components. When one reads account services or `Home` context,
   give it a constructor over plain signals (as `sidebar::Rail::detached`), gated
   `#[cfg(debug_assertions)]`, rather than a preview branch inside the component.
3. Fixed data lives in `fixtures.rs`. Ids are UUIDs: avatar and name colors hash them.
   Reference repository images with `fixture_image!`, so the browser build embeds them.
4. Wire each action with `actions.on("<Action>", prepare, run)`. `prepare` sets the
   state the action starts from, without animation, at the press; `run` follows 800 ms
   later, so every capture of the action shows a change.
5. Keep a scene with actions still once settled: put spinners and looping animations
   in a scene without actions (as `loading`). A native capture fails when its end
   keeps moving.
6. The scene must build for the browser too: `scripts/test-web-wasm.sh` checks
   `cargo check -p mixin_desktop_ui --target wasm32-unknown-unknown --lib`.
