---
name: nsh-preview
description: 'Capture before/after screenshots of NSH UI changes with the preview scenes in nsh/src/preview and nsh/tool/preview.py, and attach them to the hand-off summary. Use when changing, reviewing or debugging how an NSH screen or ui component looks or animates, or when adding a preview scene.'
---

# NSH preview captures

`nsh/src/preview/` is a debug-only scene browser (`Preview` root, `nsh/preview/nsh.json`)
that renders production components over fixtures, with no account or database.
`nsh/tool/preview.py` drives it on macOS and writes images to
`nsh/preview/output/<scene>/<label>/` (git-ignored). Requires the `nsh` CLI and `ffmpeg`.

List scenes and their toolbar actions in `SCENES` in `nsh/src/preview/mod.rs`
and each scene's `ACTIONS`; flags in `python3 nsh/tool/preview.py capture --help`.

## Steps for a UI change

1. **Pick scenes.** Every scene that renders a component you will touch. When none
   does, add one (below) before editing, so the baseline exists.
2. **Capture `before` on the unchanged code**, from the repository root:
   `python3 nsh/tool/preview.py capture <scene> [--action <Action>] --label before`.
   Add `--scheme dark`, `--locale zh-CN` or `--width/--height` captures with their own
   labels when the change touches colors, copy or layout widths.
   If the code is already edited, capture `before` in a `git worktree` of the base
   commit and copy its `nsh/preview/output/<scene>/before/` here.
3. Make the change.
4. **Capture `after`** with identical flags, then
   `python3 nsh/tool/preview.py compare <scene> before after`.
5. **Look at every image** (`start.png`, `end.png`, `keyframes.png`, the compare
   stacks). Compare against the intent and `nsh/DESIGN.md`. For each defect you find,
   changed or pre-existing, capture it narrowly with `--crop x,y,w,h` and a label such
   as `issue-<short-name>`.
6. **Attach to the summary.** Done when the hand-off includes, per scene:
   - the compare images as Markdown images with repository-relative paths, e.g.
     `![sidebar before/after](nsh/preview/output/sidebar/before-vs-after/start.png)`;
   - one line on what visibly changed;
   - each `issue-*` image with a sentence on the defect and whether this change caused it;
   - what the captures do not cover (screens without a scene, Windows/Linux, smoothness).

The output directory is local; a PR needs the images uploaded by hand, so name the
paths in the PR body.

## Add a scene

1. `nsh/src/preview/<name>.rs` with `pub(super) const ACTIONS: &[&str]` and
   `pub(super) fn scene(actions: Actions) -> View`; register it in `SCENES`.
2. Render production components. When one reads account services or `Home` context,
   give it a constructor over plain signals (as `sidebar::Rail::detached`), gated
   `#[cfg(debug_assertions)]`, rather than a preview branch inside the component.
3. Fixed data lives in `fixtures.rs`. Ids are UUIDs: avatar and name colors hash them.
4. Wire each action with `actions.on("<Action>", prepare, run)`. `prepare` sets the
   state the action starts from, without animation, at the press; `run` follows 800 ms
   later, so every capture of the action shows a change.
5. Keep a scene with actions still once settled: put spinners and looping animations
   in a scene without actions (as `loading`). A capture fails when its end keeps moving.

## Browser iteration

`python3 nsh/tool/preview_web.py` serves the same scenes in a browser (port 8796 or
`$PORT`; `.claude/launch.json` names it `preview-web`). Switch scenes with the
`Next scene` / `Previous scene` buttons and run actions with `Play action`; NSH
buttons react to pointer events, so use real clicks rather than `element.click()`.
Use it for fast layout, copy and interaction checks. Hand-off images still come
from the native `capture`/`compare` steps above, since browser text and scrolling differ.

## Limits

- `capture.txt` times frames from issuing the press and labels each `start`, `moving`
  or `end` by pixels; `first change` brackets when the action began. A capture whose
  start and end match fails as a no-op.
- A failed capture or compare leaves the previous output in place. Scene and label
  names are letters, digits, `-` and `_`. Concurrent captures queue on a lock.
- Frames arrive about every 100 ms: they show an animation's start, end and path,
  not its frame rate. Smoothness needs a release build and `nsh run --fps`.
- The toolbar (40 pt, `TOOLBAR_HEIGHT` in both `mod.rs` and `preview.py`) is cropped off.
- `cd nsh/preview && nsh run` opens the browser interactively.
