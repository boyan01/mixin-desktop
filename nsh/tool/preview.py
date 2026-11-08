#!/usr/bin/env python3
"""Capture NSH preview scenes and compare captures (macOS).

  preview.py capture sidebar --action Collapse --label before
  preview.py capture sidebar --action Collapse --label after
  preview.py compare sidebar before after

Captures go to nsh/preview/output/<scene>/<label>/. See .agents/skills/nsh-preview/SKILL.md.
"""

import argparse
import fcntl
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
  await app.press(`label:Scene ${{config.scene}}`);
  if (!config.action) {{
    await app.screenshot(`${{config.out}}/start.png`);
    return;
  }}
  // The press prepares the action's start state at once and runs it later, so the
  // start frame follows the press. Times count from issuing the press.
  const pressed = Date.now();
  await app.press(`label:Action ${{config.action}}`);
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


def shoot(args, raw):
    config = {
        "scene": args.scene, "action": args.action, "out": str(raw),
        "width": args.width, "height": args.height, "locale": args.locale,
        "scheme": args.scheme, "duration": args.duration,
    }
    with capture_lock():
        run_dir = Path(tempfile.mkdtemp(prefix="run-", dir=RUNS))
        try:
            test = run_dir / "capture.test.tsx"
            entry = os.path.relpath(PREVIEW, run_dir)
            test.write_text(TEST.format(config=json.dumps(config, indent=2),
                                        entry=json.dumps(entry)))
            # Share the workspace target directory instead of building a second one.
            env = dict(os.environ, CARGO_TARGET_DIR=str(NSH.parent / "target"))
            run(["nsh", "test", str(test.relative_to(PREVIEW)), "--target", "macos"],
                cwd=PREVIEW, env=env)
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
    checked_name("scene", args.scene)
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
                 f"window: {args.width}x{args.height} pt, {args.locale}, {args.scheme}"]
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
    shot = commands.add_parser("capture", help="screenshot a scene, optionally through an action")
    shot.add_argument("scene")
    shot.add_argument("--action", help="toolbar action to record, e.g. Collapse")
    shot.add_argument("--label", default="current", help="capture name (default: current)")
    shot.add_argument("--duration", type=int, default=1500,
                      help="ms to record after issuing the action's press (it runs ~800 ms later)")
    shot.add_argument("--keyframes", type=int, default=8, help="images in keyframes.png")
    shot.add_argument("--width", type=int, default=900)
    shot.add_argument("--height", type=int, default=650)
    shot.add_argument("--locale", default="en")
    shot.add_argument("--scheme", choices=("light", "dark"), default="light")
    shot.add_argument("--crop", help="x,y,w,h in points of the scene area below the toolbar")
    shot.set_defaults(handler=capture)
    diff = commands.add_parser("compare", help="stack two captures of one scene")
    diff.add_argument("scene")
    diff.add_argument("before")
    diff.add_argument("after")
    diff.set_defaults(handler=compare)
    args = parser.parse_args()
    args.handler(args)


if __name__ == "__main__":
    main()
