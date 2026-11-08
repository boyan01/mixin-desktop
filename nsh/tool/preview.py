#!/usr/bin/env python3
"""Capture NSH preview scenes and compare captures (macOS).

  preview.py capture sidebar --action Collapse --label before
  preview.py capture sidebar --action Collapse --label after
  preview.py compare sidebar before after

Captures go to nsh/preview/output/<scene>/<label>/. See .agents/skills/nsh-preview/SKILL.md.
"""

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

NSH = Path(__file__).resolve().parent.parent
PREVIEW = NSH / "preview"
OUTPUT = PREVIEW / "output"
# Mirrors ACTION_LEAD and TOOLBAR_HEIGHT in nsh/src/preview/mod.rs.
ACTION_LEAD_MS = 800
TOOLBAR_HEIGHT = 40

TEST = """import {{ open, test }} from "nsh/test";

const config = {config};

test("preview capture", async () => {{
  const app = await open("..", {{
    width: config.width,
    height: config.height,
    locale: config.locale,
    colorScheme: config.scheme,
  }});
  await app.press(`label:Scene ${{config.scene}}`);
  await app.screenshot(`${{config.out}}/start.png`);
  if (!config.action) return;
  const start = Date.now();
  await app.press(`label:Action ${{config.action}}`);
  for (let frame = 0; ; frame++) {{
    const elapsed = Date.now() - start;
    if (elapsed > config.lead + config.duration) break;
    const name = `frame-${{String(frame).padStart(3, "0")}}-${{elapsed}}ms.png`;
    await app.screenshot(`${{config.out}}/${{name}}`);
  }}
  await app.wait(300);
  await app.screenshot(`${{config.out}}/end.png`);
}}, 120_000);
"""


def run(command, **kwargs):
    subprocess.run(command, check=True, **kwargs)


def ffmpeg(*args):
    run(["ffmpeg", "-hide_banner", "-loglevel", "error", "-y", *args])


def image_size(path):
    probe = subprocess.run(
        ["ffprobe", "-v", "error", "-select_streams", "v:0", "-show_entries",
         "stream=width,height", "-of", "csv=p=0", str(path)],
        check=True, capture_output=True, text=True,
    )
    width, height = probe.stdout.strip().split(",")
    return int(width), int(height)


def crop_filter(path, window_width, region):
    """Crops the toolbar off, then to `region` (x,y,w,h in points of the scene)."""
    pixels, _ = image_size(path)
    scale = pixels / window_width
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


def capture(args):
    for tool in ("nsh", "ffmpeg", "ffprobe"):
        if not shutil.which(tool):
            sys.exit(f"{tool} is required")
    out = OUTPUT / args.scene / args.label
    if out.exists():
        shutil.rmtree(out)
    out.mkdir(parents=True)
    raw = out / "raw"
    raw.mkdir()
    config = {
        "scene": args.scene, "action": args.action, "out": str(raw),
        "width": args.width, "height": args.height, "locale": args.locale,
        "scheme": args.scheme, "lead": ACTION_LEAD_MS, "duration": args.duration,
    }
    tests = PREVIEW / ".capture"
    tests.mkdir(exist_ok=True)
    test = tests / "capture.test.tsx"
    test.write_text(TEST.format(config=json.dumps(config, indent=2)))
    # Share the workspace target directory instead of building a second one.
    env = dict(os.environ, CARGO_TARGET_DIR=str(NSH.parent / "target"))
    run(["nsh", "test", str(test.relative_to(PREVIEW)), "--target", "macos"],
        cwd=PREVIEW, env=env)

    region = [float(value) for value in args.crop.split(",")] if args.crop else None
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
        frames = []
        for path in sorted(out.glob("frame-*.png")):
            match = re.fullmatch(r"frame-\d+-(\d+)ms\.png", path.name)
            frames.append((int(match.group(1)) - ACTION_LEAD_MS, path))
        moving = [frame for frame in frames if frame[0] >= 0]
        if not moving:
            sys.exit("No frame was taken after the action started")
        # Evenly spaced frames across the action, then its settled end.
        count = min(args.keyframes - 1, len(moving))
        picked = [moving[round(i * (len(moving) - 1) / max(count - 1, 1))] for i in range(count)]
        picked.append((None, out / "end.png"))
        lines.append("keyframes.png, row by row (ms since the action started):")
        lines += [f"  {i + 1}: {'end' if t is None else f'{t} ms'} ({p.name})"
                  for i, (t, p) in enumerate(picked)]
        lines.append("frames (ms since the action started):")
        lines += [f"  {t:>5} ms  {p.name}" for t, p in frames]
        strip = manifest(out / "keyframes.ffconcat", [(p, 0.04) for _, p in picked])
        columns = min(4, len(picked))
        rows = -(-len(picked) // columns)
        ffmpeg("-f", "concat", "-safe", "0", "-i", str(strip),
               "-vf", f"scale=iw/2:-1,tile={columns}x{rows}:padding=8:color=white",
               "-frames:v", "1", str(out / "keyframes.png"))
        clip = [(-ACTION_LEAD_MS, out / "start.png")] + frames
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
    print(f"\n{out}")
    print("\n".join(lines[:3 + (args.keyframes + 1 if args.action else 0)]))


def compare(args):
    before, after = (OUTPUT / args.scene / label for label in (args.before, args.after))
    for directory in (before, after):
        if not directory.is_dir():
            sys.exit(f"No capture at {directory}")
    out = OUTPUT / args.scene / f"{args.before}-vs-{args.after}"
    out.mkdir(exist_ok=True)
    made = []
    for name in ("start.png", "end.png", "keyframes.png"):
        if (before / name).exists() and (after / name).exists():
            # Before above after, so a strip of keyframes stays readable.
            ffmpeg("-i", str(before / name), "-i", str(after / name),
                   "-filter_complex", "[0][1]vstack=inputs=2", str(out / name))
            made.append(name)
    if not made:
        sys.exit("The captures share no images")
    print(f"{out}: {', '.join(made)} (top: {args.before}, bottom: {args.after})")


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    commands = parser.add_subparsers(dest="command", required=True)
    shot = commands.add_parser("capture", help="screenshot a scene, optionally through an action")
    shot.add_argument("scene")
    shot.add_argument("--action", help="toolbar action to record, e.g. Collapse")
    shot.add_argument("--label", default="current", help="capture name (default: current)")
    shot.add_argument("--duration", type=int, default=700, help="ms to record after the action starts")
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
