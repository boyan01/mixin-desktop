#!/usr/bin/env python3
"""Build the app's native Rust entry point into an ad hoc signed macOS bundle."""
import argparse
import json
from pathlib import Path
import plistlib
import re
import shutil
import subprocess
import sys
import tempfile


def run(*args, **kwargs):
    return subprocess.run(args, check=True, **kwargs)


def main():
    if sys.platform != "darwin":
        raise SystemExit("This packager requires macOS and its native command-line tools")
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--release", action="store_true")
    parser.add_argument("--out", type=Path, help="Output .app path")
    args = parser.parse_args()
    project = Path(__file__).resolve().parents[1]
    workspace = project.parent
    metadata = json.loads((project / "nsh.json").read_text())
    output = (args.out or project / ".nsh/out/macos" / f"{metadata['name']}.app").resolve()
    if output.suffix != ".app":
        raise SystemExit("--out must name a .app bundle")
    if output.exists():
        info = output / "Contents/Info.plist"
        if not info.is_file() or plistlib.loads(info.read_bytes()).get("CFBundleIdentifier") != metadata["id"]:
            raise SystemExit("Refusing to replace a bundle belonging to another application")
    command = ["cargo", "build", "-p", "mixin_desktop_ui", "--bin", "mixin_desktop_ui"]
    if args.release:
        command.append("--release")
    run(*command, cwd=workspace)
    cargo = json.loads(subprocess.check_output(["cargo", "metadata", "--no-deps", "--format-version", "1"], cwd=workspace))
    binary = Path(cargo["target_directory"]) / ("release" if args.release else "debug") / "mixin_desktop_ui"
    libraries = subprocess.check_output(["otool", "-L", binary], text=True)
    external = [line.split(" (", 1)[0].strip() for line in libraries.splitlines()[1:]
                if not line.strip().startswith(("/System/Library/", "/usr/lib/"))]
    if external:
        raise SystemExit("The executable has unbundled native libraries: " + ", ".join(external))
    load_commands = subprocess.check_output(["otool", "-l", binary], text=True)
    minimum = re.search(r"\bminos\s+(\d+(?:\.\d+)+)", load_commands)
    minimum = minimum.group(1) if minimum else "11.0"
    contents = output / "Contents"
    macos = contents / "MacOS"
    resources = contents / "Resources"
    macos.mkdir(parents=True, exist_ok=True)
    resources.mkdir(parents=True, exist_ok=True)
    executable = macos / "MixinMessenger"
    shutil.copy2(binary, executable)
    executable.chmod(0o755)
    shutil.copy2(workspace / "mixin_desktop_media/THIRD-PARTY-NOTICES.md",
                 resources / "Media-Third-Party-Notices.md")
    shutil.copy2(project / "assets/emoji.LICENSE", resources / "Emoji-LICENSE.txt")
    with tempfile.TemporaryDirectory(prefix="mixin-app-icon-") as temporary:
        iconset = Path(temporary) / "AppIcon.iconset"
        iconset.mkdir()
        for size in [16, 32, 128, 256, 512]:
            for scale in [1, 2]:
                name = f"icon_{size}x{size}{'@2x' if scale == 2 else ''}.png"
                run("sips", "-z", str(size * scale), str(size * scale), project / "assets/app_icon.png", "--out", iconset / name,
                    stdout=subprocess.DEVNULL)
        run("iconutil", "-c", "icns", iconset, "-o", resources / "AppIcon.icns")
    info = {
        "CFBundlePackageType": "APPL",
        "CFBundleExecutable": executable.name,
        "CFBundleIdentifier": metadata["id"],
        "CFBundleName": metadata["name"],
        "CFBundleDisplayName": metadata["name"],
        "CFBundleShortVersionString": metadata["version"],
        "CFBundleVersion": str(metadata["build"]),
        "CFBundleIconFile": "AppIcon.icns",
        "NSPrincipalClass": "NSApplication",
        "NSHighResolutionCapable": True,
        "NSSupportsAutomaticGraphicsSwitching": True,
        "LSMinimumSystemVersion": minimum,
        "NSMicrophoneUsageDescription": "Record voice messages to send in your conversations.",
        "NSLocalNetworkUsageDescription": "Transfer your chat history between your own devices.",
        "CFBundleURLTypes": [{
            "CFBundleURLName": metadata["id"],
            "CFBundleURLSchemes": ["mixin"],
            "CFBundleTypeRole": "Viewer",
            "LSHandlerRank": "Alternate",
        }],
    }
    (contents / "Info.plist").write_bytes(plistlib.dumps(info))
    (contents / "PkgInfo").write_bytes(b"APPL????")
    # Local development signing only; distribution signing/notarization is separate.
    run("codesign", "--force", "--sign", "-", output)
    run("codesign", "--verify", "--strict", output)
    print(f"Built and verified {output}")


if __name__ == "__main__":
    main()
