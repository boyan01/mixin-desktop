#!/usr/bin/env python3
"""Capture NSH preview scenes and compare captures (macOS).

  preview.py capture sidebar --action Collapse --label before
  preview.py capture sidebar --action Collapse --label after
  preview.py compare sidebar before after
  preview.py run

Captures stay below normal application windows by default; --foreground shows
one above them for manual inspection. Captures go to
nsh/preview/output/<scene>/<label>/. See .agents/skills/nsh-preview/SKILL.md.
"""

import argparse
import fcntl
import hashlib
import html
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from contextlib import contextmanager
from pathlib import Path

NSH = Path(__file__).resolve().parent.parent
PREVIEW = NSH / "preview"
OUTPUT = PREVIEW / "output"
RUNS = PREVIEW / ".capture"
# Mirrors TOOLBAR_HEIGHT in nsh/src/preview/mod.rs.
TOOLBAR_HEIGHT = 40
NAME = re.compile(r"[A-Za-z0-9][A-Za-z0-9_-]*")

TEST = """import {{ open, test }} from "nsh/test";

const config = {config};

test("preview capture", async () => {{
  const app = await open({entry}, {{
    width: config.width,
    height: config.height,
    locale: config.locale,
    colorScheme: config.scheme,
  }});
  // The root selects this scene before mounting its native controls.
  if (config.focus) await app.type(config.focus, "");
  // Let the native scene switch and deferred syntax colors reach the capture.
  await app.wait(300);
  if (!config.action) {{
    await app.screenshot(`${{config.out}}/start.png`);
    return;
  }}
  // The press prepares the action's start state at once and runs it later, so the
  // start frame follows the press. Times count from issuing the press.
  await app.press("label:Preview action");
  await app.press(`role:option[${{config.action}}]`);
  const pressed = Date.now();
  await app.press("label:Play action");
  await app.screenshot(`${{config.out}}/start.png`);
  for (let frame = 0; ; frame++) {{
    const elapsed = Date.now() - pressed;
    if (elapsed > config.duration) break;
    const name = `frame-${{String(frame).padStart(3, "0")}}-${{elapsed}}ms.png`;
    await app.screenshot(`${{config.out}}/${{name}}`);
  }}
  await app.wait(300);
  await app.screenshot(`${{config.out}}/end.png`);
  await app.wait(300);
  await app.screenshot(`${{config.out}}/settled.png`);
}}, 120_000);
"""


CATALOG_TEST = """import {{ open, test }} from "nsh/test";
const config = {config};
test("preview catalog", async () => {{
  const app = await open({entry}, {{ width: config.width, height: config.height,
    locale: config.locale, colorScheme: config.scheme }});
  for (const [index, scene] of config.scenes.entries()) {{
    if (config.filtered && index > 0) {{
      await app.press("label:Preview scene");
      await app.press(`role:option[${{scene.name}}]`);
    }}
    await app.wait(300);
    await app.screenshot(`${{config.out}}/${{scene.name}}.png`);
    for (const state of scene.states) {{
      await app.press("label:Preview action");
      await app.press(`role:option[${{state.action}}]`);
      await app.press("label:Play action");
      await app.wait(1150);
      await app.screenshot(`${{config.out}}/${{scene.name}}--${{state.key}}.png`);
    }}
    if (!config.filtered) await app.press("label:Next scene");
  }}
}}, 600_000);
"""


def scene_catalog():
    source = (NSH / "src/preview/mod.rs").read_text()
    entries = re.findall(r'name:\s*"([A-Za-z0-9_-]+)"\s*,\s*actions:\s*(\w+)::(\w+)', source)
    result = []
    for name, module, constant in entries:
        module_source = (NSH / f"src/preview/{module}.rs").read_text()
        definition = re.search(rf'const {constant}:.*?=\s*&\[(.*?)\];', module_source, re.S)
        if not definition:
            sys.exit(f"Cannot find actions for {name}")
        actions = re.findall(r'"([^"]+)"', definition[1])
        states = [{"action": action, "key": re.sub(r"[^a-z0-9]+", "-", action.lower()).strip("-")}
                  for action in actions if action != "Reset"]
        result.append({"name": name, "states": states})
    return result


def scenes():
    return [scene["name"] for scene in scene_catalog()]


def checked_scene(name):
    checked_name("scene", name)
    if name not in scenes():
        sys.exit(f"Unknown scene {name!r}; run preview.py list")
    return name


def run(command, **kwargs):
    subprocess.run(command, check=True, **kwargs)


def ffmpeg(*args):
    run(["ffmpeg", "-hide_banner", "-loglevel", "error", "-y", *args])


def pixels(path):
    """A digest of the decoded image, so equal pictures compare equal."""
    result = subprocess.run(
        ["ffmpeg", "-hide_banner", "-loglevel", "error", "-i", str(path), "-f", "md5", "-"],
        check=True, capture_output=True, text=True,
    )
    return result.stdout.strip()


def image_width(path):
    probe = subprocess.run(
        ["ffprobe", "-v", "error", "-select_streams", "v:0", "-show_entries",
         "stream=width", "-of", "csv=p=0", str(path)],
        check=True, capture_output=True, text=True,
    )
    return int(probe.stdout.strip())


def checked_name(kind, value):
    """A plain directory name: no separators, no `..`, nothing absolute."""
    if not NAME.fullmatch(value):
        sys.exit(f"Invalid {kind} {value!r}: use letters, digits, '-' and '_'")
    return value


def output_dir(*names):
    path = OUTPUT.joinpath(*names).resolve()
    if not path.is_relative_to(OUTPUT.resolve()):
        sys.exit(f"{path} is outside {OUTPUT}")
    return path


@contextmanager
def staged(final):
    """Yields an empty sibling directory that replaces `final` only on success."""
    final.parent.mkdir(parents=True, exist_ok=True)
    work = Path(tempfile.mkdtemp(prefix=f".{final.name}-", dir=final.parent))
    try:
        yield work
        old = None
        if final.exists():
            old = Path(tempfile.mkdtemp(prefix=f".{final.name}-old-", dir=final.parent))
            final.rename(old / "previous")
        work.rename(final)
        if old:
            shutil.rmtree(old)
    finally:
        if work.exists():
            shutil.rmtree(work)


@contextmanager
def capture_lock():
    """One capture at a time: concurrent runs would share the generated host and window."""
    RUNS.mkdir(exist_ok=True)
    with open(RUNS / "lock", "w") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        yield


def crop_filter(path, window_width, region):
    """Crops the toolbar off, then to `region` (x,y,w,h in points of the scene)."""
    scale = image_width(path) / window_width
    top = round(TOOLBAR_HEIGHT * scale)
    if region:
        x, y, w, h = (round(value * scale) for value in region)
        return f"crop={w}:{h}:{x}:{top + y}"
    return f"crop=iw:ih-{top}:0:{top}"


def manifest(path, entries):
    path.write_text("ffconcat version 1.0\n" + "".join(
        f"file '{frame.name}'\nduration {duration:.3f}\n" for frame, duration in entries
    ) + f"file '{entries[-1][0].name}'\n")
    return path


def capture_entry(foreground):
    """Apply capture-only window options before the native window is created."""
    if foreground:
        return PREVIEW
    app = json.loads((PREVIEW / "nsh.json").read_text())
    # Keep the same crate and theme from this separate, cached capture entry.
    for key in ("rust", "theme"):
        app[key] = str((PREVIEW / app[key]).resolve())
    app.setdefault("window", {}).update({
        "level": "desktopIcon", "activate": False, "taskbar": False,
    })
    entry = RUNS / "app"
    entry.mkdir(exist_ok=True)
    manifest = entry / "nsh.json"
    content = json.dumps(app, indent=2) + "\n"
    if not manifest.exists() or manifest.read_text() != content:
        manifest.write_text(content)
    return entry


def build_env():
    # NSH selects its own target directory instead of Cargo's configured default.
    # Resolve caller overrides before changing to the preview entry directory.
    env = dict(os.environ)
    target = env.get("CARGO_TARGET_DIR")
    env["CARGO_TARGET_DIR"] = str(Path(target).resolve() if target else NSH.parent / "target")
    return env


def seed_lockfile(entry):
    # Generated hosts need extra packages, but should start with the application's
    # pinned versions rather than resolving all shared dependencies independently.
    source = (NSH.parent / "Cargo.lock").read_bytes()
    digest = hashlib.sha256(source).hexdigest()
    workspace = entry / ".nsh/rust"
    lock = workspace / "Cargo.lock"
    stamp = workspace / ".mixin-source-lock"
    if lock.exists() and stamp.exists() and stamp.read_text() == digest:
        return
    workspace.mkdir(parents=True, exist_ok=True)
    lock.write_bytes(source)
    stamp.write_text(digest)


def open_preview(_):
    with capture_lock():
        seed_lockfile(PREVIEW)
    run(["nsh", "run"], cwd=PREVIEW, env=build_env())


def shoot(args, raw):
    config = {
        "scene": args.scene, "action": args.action, "out": str(raw),
        "width": args.width, "height": args.height, "locale": args.locale,
        "scheme": args.scheme, "duration": args.duration,
        "focus": getattr(args, "focus", None),
    }
    with capture_lock():
        entry = capture_entry(args.foreground)
        seed_lockfile(entry)
        # nsh discovers the app from the test file's ancestors, not only cwd.
        run_dir = Path(tempfile.mkdtemp(prefix="run-", dir=RUNS if args.foreground else entry))
        try:
            test = run_dir / "capture.test.tsx"
            if getattr(args, "scenes", None):
                config["scenes"] = args.scenes
                config["filtered"] = bool(getattr(args, "scene_names", None))
            template = CATALOG_TEST if "scenes" in config else TEST
            test.write_text(template.format(config=json.dumps(config, indent=2),
                                        entry=json.dumps(os.path.relpath(entry, run_dir))))
            env = build_env()
            env["MIXIN_PREVIEW_SCENE"] = args.scene
            command = ["nsh", "test", os.path.relpath(test, entry), "--target", "macos"]
            if args.foreground:
                command.append("--foreground")
            run(command, cwd=entry, env=env)
        finally:
            shutil.rmtree(run_dir)


def timeline(out, keyframes):
    """Classifies frames against start and end; fails when the action showed nothing."""
    frames = []
    for path in sorted(out.glob("frame-*.png")):
        match = re.fullmatch(r"frame-\d+-(\d+)ms\.png", path.name)
        frames.append((int(match.group(1)), path))
    start, end = pixels(out / "start.png"), pixels(out / "end.png")
    settled = out / "settled.png"
    if pixels(settled) != end:
        sys.exit("The scene still moves 300 ms after end.png (a spinner or a looping "
                 "animation), so frames cannot be told apart. Keep continuous motion out "
                 "of scenes with actions.")
    settled.unlink()
    if start == end:
        sys.exit("The action changed nothing: start.png and end.png are identical")
    states = []
    for time, path in frames:
        digest = pixels(path)
        states.append("start" if digest == start else "end" if digest == end else "moving")
    first = next((i for i, state in enumerate(states) if state != "start"), None)
    lines = ["frames (ms since the action's press was issued; state against start/end):"]
    lines += [f"  {t:>5} ms  {state:<6}  {p.name}" for (t, p), state in zip(frames, states)]
    if first is None:
        lines.insert(0, "No frame moved before end.png: raise --duration to see the motion.")
        return lines, [(None, out / "start.png"), (None, out / "end.png")], frames
    done = next((i for i in range(first, len(states)) if states[i] == "end"), len(frames) - 1)
    # The last unchanged frame, the motion, and the first settled frame.
    span = frames[max(first - 1, 0):done + 1]
    count = min(keyframes - 1, len(span))
    picked = [span[round(i * (len(span) - 1) / max(count - 1, 1))] for i in range(count)]
    picked.append((None, out / "end.png"))
    if first == 0:
        when = f"at or before the first frame, {frames[0][0]} ms after the press"
    else:
        when = f"between {frames[first - 1][0]} and {frames[first][0]} ms after the press"
    lines.insert(0, f"first change: {when}")
    return lines, picked, frames


def capture(args):
    checked_scene(args.scene)
    checked_name("label", args.label)
    for tool in ("nsh", "ffmpeg", "ffprobe"):
        if not shutil.which(tool):
            sys.exit(f"{tool} is required")
    final = output_dir(args.scene, args.label)
    region = [float(value) for value in args.crop.split(",")] if args.crop else None
    with staged(final) as out:
        raw = out / "raw"
        raw.mkdir()
        shoot(args, raw)
        shots = sorted(raw.glob("*.png"))
        if not shots:
            sys.exit("No screenshots were captured")
        view = crop_filter(shots[0], args.width, region)
        for shot in shots:
            ffmpeg("-i", str(shot), "-vf", view, str(out / shot.name))
        shutil.rmtree(raw)

        lines = [f"scene: {args.scene}", f"action: {args.action or '-'}",
                 f"window: {args.width}x{args.height} pt, {args.locale}, {args.scheme}",
                 f"presentation: {'foreground' if args.foreground else 'below application windows'}"]
        if args.action:
            frame_lines, picked, frames = timeline(out, args.keyframes)
            lines += frame_lines[:1]
            lines.append("keyframes.png, row by row:")
            lines += [f"  {i + 1}: {p.name}" for i, (_, p) in enumerate(picked)]
            lines += frame_lines[1:]
            strip = manifest(out / "keyframes.ffconcat", [(p, 0.04) for _, p in picked])
            columns = min(4, len(picked))
            rows = -(-len(picked) // columns)
            ffmpeg("-f", "concat", "-safe", "0", "-i", str(strip),
                   "-vf", f"scale=iw/2:-1,tile={columns}x{rows}:padding=8:color=white",
                   "-frames:v", "1", str(out / "keyframes.png"))
            clip = [(0, out / "start.png")] + frames
            clip.append((clip[-1][0] + 1, out / "end.png"))
            entries = [(p, max(0.001, (n - t) / 1000)) for (t, p), (n, _) in zip(clip, clip[1:])]
            entries.append((clip[-1][1], 0.5))
            movie = manifest(out / "recording.ffconcat", entries)
            ffmpeg("-f", "concat", "-safe", "0", "-i", str(movie), "-fps_mode", "vfr",
                   "-vf", "pad=ceil(iw/2)*2:ceil(ih/2)*2", "-pix_fmt", "yuv420p",
                   str(out / "recording.mp4"))
            for path in out.glob("*.ffconcat"):
                path.unlink()
        (out / "capture.txt").write_text("\n".join(lines) + "\n")
    print(f"\n{final}")
    print("\n".join(line for line in lines if not line.startswith("  ") or "keyframe" in line))


def catalog(args):
    """Capture the scene browser in one native run and make a local review gallery."""
    checked_name("label", args.label)
    available = scene_catalog()
    if args.scene_names:
        by_name = {scene["name"]: scene for scene in available}
        args.scenes = [by_name[checked_scene(name)] for name in dict.fromkeys(args.scene_names)]
    else:
        args.scenes = available
    args.scene = args.scenes[0]["name"]
    args.action = None
    args.duration = 0
    final = output_dir("button-review", args.label)
    with staged(final) as out:
        raw = out / "raw"
        raw.mkdir()
        shoot(args, raw)
        cards = []
        previous = None
        total_states = 0
        for scene in args.scenes:
            name = scene["name"]
            variants = [("Normal", name + ".png")] + [(state["action"], f"{name}--{state['key']}.png") for state in scene["states"]]
            options = []
            for label, filename in variants:
                shot = raw / filename
                if not shot.exists():
                    sys.exit(f"Missing capture for {name}: {label}")
                ffmpeg("-i", str(shot), "-vf", crop_filter(shot, args.width, None), str(out / filename))
                digest = pixels(out / filename)
                if label == "Normal":
                    if previous and previous[1] == digest:
                        sys.exit(f"Identical captures for {previous[0]} and {name}; scene switching may be obscured")
                    previous = (name, digest)
                options.append(f'<option value="{filename}">{html.escape(label)}</option>')
            total_states += len(variants)
            cards.append(f'<article id="{name}"><div class="title"><h2>{name}</h2>'
                         f'<select aria-label="{name} state" onchange="changeState(this)">{"".join(options)}</select></div>'
                         f'<a href="{name}.png"><img loading="lazy" src="{name}.png" alt="{name}: Normal"></a></article>')
        shutil.rmtree(raw)
        options = "".join(f'<option value="#{scene["name"]}">{scene["name"]}</option>' for scene in args.scenes)
        title = html.escape(f"Button design review · {args.scheme} · {args.locale}")
        (out / "index.html").write_text(
            '<!doctype html><html lang="en"><meta charset="utf-8">'
            f'<title>{title}</title><style>'
            'body{margin:0;background:#202328;color:#eee;font:15px system-ui}'
            'header{position:sticky;top:0;padding:16px;background:#292d33;z-index:1}'
            'select{font:inherit;padding:8px;margin-left:16px;max-width:70%}'
            'main{display:grid;grid-template-columns:repeat(auto-fit,minmax(420px,1fr));gap:24px;padding:24px}'
            'article{scroll-margin-top:90px}.title{display:flex;align-items:center;justify-content:space-between;margin-bottom:12px}'
            'h2{font-size:16px;font-weight:500}img{display:block;width:100%;border:1px solid #555;border-radius:8px}'
            '</style><script>function changeState(select){const card=select.closest("article");'
            'const file=select.value;card.querySelector("img").src=file;card.querySelector("a").href=file;'
            'card.querySelector("img").alt=card.id+": "+select.selectedOptions[0].text}</script>'
            f'<header>{title}<select aria-label="Scene" onchange="location.hash=this.value">{options}</select></header>'
            f'<main>{"".join(cards)}</main></html>')
        (out / "capture.txt").write_text(
            f"window: {args.width}x{args.height} pt, {args.locale}, {args.scheme}\n"
            + "\n".join(scene["name"] for scene in args.scenes) + "\n")
        (out / "scenes.json").write_text(json.dumps(args.scenes, indent=2) + "\n")
    print(f"{final / 'index.html'} ({len(args.scenes)} scenes, {total_states} states)")


def compare(args):
    for kind, value in (("scene", args.scene), ("label", args.before), ("label", args.after)):
        checked_name(kind, value)
    before, after = (output_dir(args.scene, label) for label in (args.before, args.after))
    for directory in (before, after):
        if not directory.is_dir():
            sys.exit(f"No capture at {directory}")
    final = output_dir(args.scene, f"{args.before}-vs-{args.after}")
    made = []
    with staged(final) as out:
        for name in ("start.png", "end.png", "keyframes.png"):
            if (before / name).exists() and (after / name).exists():
                # Before above after, so a strip of keyframes stays readable.
                ffmpeg("-i", str(before / name), "-i", str(after / name),
                       "-filter_complex", "[0][1]vstack=inputs=2", str(out / name))
                made.append(name)
        if not made:
            sys.exit("The captures share no images")
    print(f"{final}: {', '.join(made)} (top: {args.before}, bottom: {args.after})")


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    commands = parser.add_subparsers(dest="command", required=True)
    interactive = commands.add_parser("run", help="open the native browser using the shared build cache")
    interactive.set_defaults(handler=open_preview)
    shot = commands.add_parser("capture", help="screenshot a scene, optionally through an action")
    shot.add_argument("scene")
    shot.add_argument("--focus", help="focus an input selector before a static capture")
    shot.add_argument("--action", help="toolbar action to record, e.g. Collapse")
    shot.add_argument("--label", default="current", help="capture name (default: current)")
    shot.add_argument("--duration", type=int, default=1500,
                      help="ms to record after issuing the action's press (it runs ~800 ms later)")
    shot.add_argument("--keyframes", type=int, default=8, help="images in keyframes.png")
    shot.add_argument("--width", type=int, default=900)
    shot.add_argument("--height", type=int, default=650)
    shot.add_argument("--locale", default="en")
    shot.add_argument("--scheme", choices=("light", "dark"), default="light")
    shot.add_argument("--foreground", action="store_true",
                      help="show the capture window above other apps (default: below them)")
    shot.add_argument("--crop", help="x,y,w,h in points of the scene area below the toolbar")
    shot.set_defaults(handler=capture)
    listing = commands.add_parser("list", help="list registered native scenes")
    listing.set_defaults(handler=lambda _: print("\n".join(scenes())))
    gallery = commands.add_parser("catalog", help="capture all scenes as a local HTML review gallery")
    gallery.add_argument("--label", default="current")
    gallery.add_argument("--scenes", dest="scene_names", nargs="+", help="capture only these scenes in the given order")
    gallery.add_argument("--width", type=int, default=980)
    gallery.add_argument("--height", type=int, default=820)
    gallery.add_argument("--locale", default="zh-CN")
    gallery.add_argument("--scheme", choices=("light", "dark"), default="dark")
    gallery.add_argument("--foreground", action="store_true")
    gallery.set_defaults(handler=catalog)
    diff = commands.add_parser("compare", help="stack two captures of one scene")
    diff.add_argument("scene")
    diff.add_argument("before")
    diff.add_argument("after")
    diff.set_defaults(handler=compare)
    args = parser.parse_args()
    args.handler(args)


if __name__ == "__main__":
    main()
