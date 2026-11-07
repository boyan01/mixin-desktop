#!/usr/bin/env python3
"""Build a native Linux AppDir and archive, retaining explicit system dependencies."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import struct
import subprocess
import sys
import tarfile
import tempfile

IDENTITY = "one.mixin.messenger.nsh"


def elf_architecture(path):
    with path.open("rb") as source:
        header = source.read(20)
    if len(header) != 20 or header[:4] != b"\x7fELF" or header[4] != 2 or header[5] != 1:
        raise ValueError(f"Expected a real little-endian 64-bit Linux ELF file: {path}")
    architecture = {62: "x86_64", 183: "aarch64"}.get(struct.unpack_from("<H", header, 18)[0])
    if architecture is None:
        raise ValueError(f"Unsupported ELF architecture: {path}")
    return architecture


def dependencies(path):
    result = subprocess.run(["ldd", str(path)], text=True, stdout=subprocess.PIPE,
                            stderr=subprocess.STDOUT, timeout=30,
                            env={**os.environ, "LC_ALL": "C"})
    if "not a dynamic executable" in result.stdout or "statically linked" in result.stdout:
        return []
    if result.returncode or "not found" in result.stdout:
        raise ValueError(f"Unresolved native dependencies for {path.name}:\n{result.stdout}")
    return sorted(line.strip() for line in result.stdout.splitlines() if line.strip())


def sha256(path):
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def desktop_entry(name):
    if any(character in name for character in "\r\n"):
        raise ValueError("Invalid application name")
    return f"""[Desktop Entry]
Type=Application
Name={name}
Comment=Private messaging for desktop
Exec=mixin-nsh %U
Icon={IDENTITY}
Terminal=false
Categories=Network;InstantMessaging;
MimeType=x-scheme-handler/mixin;
StartupNotify=true
StartupWMClass={IDENTITY}
"""


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--release", action="store_true")
    parser.add_argument("--binary", type=Path, help="Use an existing real executable instead of building")
    media = parser.add_mutually_exclusive_group(required=True)
    media.add_argument("--ffprobe", type=Path, help="Bundle this native ffprobe executable")
    media.add_argument("--system-ffprobe", action="store_true", help="Explicitly depend on the distribution's ffprobe")
    parser.add_argument("--ffprobe-notice", type=Path, help="Required when bundling ffprobe")
    parser.add_argument("--stage-only", action="store_true", help="Build and validate AppDir without creating tar.gz")
    parser.add_argument("--out", type=Path, help="New AppDir directory; existing paths are never replaced")
    args = parser.parse_args()
    if sys.platform != "linux":
        raise SystemExit("Linux packaging requires native Linux, GTK/WebKitGTK development packages and a C compiler")
    for program in ["cargo", "pkg-config", "ldd", "desktop-file-validate", "sh"]:
        if not shutil.which(program):
            raise SystemExit(f"Required native packaging tool is missing: {program}")
    subprocess.run(["pkg-config", "--atleast-version=4.14", "gtk4"], check=True)
    subprocess.run(["pkg-config", "--exists", "webkitgtk-6.0", "webkitgtk-web-process-extension-6.0", "dbus-1", "alsa"], check=True)
    project = Path(__file__).resolve().parents[1]
    workspace = project.parent
    metadata = json.loads((project / "nsh.json").read_text())
    if metadata["id"] != IDENTITY:
        raise SystemExit("The desktop entry must use the native application identity")
    architecture = {"amd64": "x86_64", "arm64": "aarch64"}.get(platform.machine().lower(), platform.machine().lower())
    if architecture not in ["x86_64", "aarch64"]:
        raise SystemExit("Only native x86_64 and aarch64 Linux packages are supported")
    output = (args.out or project / ".nsh/out/linux" / f"MixinMessenger-{metadata['version']}-{architecture}.AppDir").resolve()
    if output.exists():
        raise SystemExit("Refusing to replace an existing output directory; choose a new --out")
    if args.ffprobe:
        if not args.ffprobe.is_file() or not args.ffprobe_notice or not args.ffprobe_notice.is_file():
            raise SystemExit("Bundled ffprobe requires a real --ffprobe file and --ffprobe-notice")
        ffprobe = args.ffprobe.resolve()
    else:
        executable = shutil.which("ffprobe")
        if not executable:
            raise SystemExit("The explicit system-ffprobe dependency is not installed on this build host")
        ffprobe = Path(executable).resolve()
    if elf_architecture(ffprobe) != architecture:
        raise SystemExit("ffprobe architecture does not match the native build host")
    if args.binary is None:
        command = ["cargo", "build", "-p", "mixin_desktop_ui", "--bin", "mixin_desktop_ui"]
        if args.release:
            command.append("--release")
        subprocess.run(command, cwd=workspace, check=True)
        cargo = json.loads(subprocess.check_output(["cargo", "metadata", "--no-deps", "--format-version", "1"], cwd=workspace))
        binary = Path(cargo["target_directory"]) / ("release" if args.release else "debug") / "mixin_desktop_ui"
    else:
        binary = args.binary.resolve()
    if elf_architecture(binary) != architecture:
        raise SystemExit("Application architecture does not match the native build host")
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="mixin-linux-package-", dir=output.parent) as temporary:
        staged = Path(temporary) / output.name
        executable_directory = staged / "usr/bin"
        executable_directory.mkdir(parents=True)
        executable = executable_directory / "mixin-nsh"
        shutil.copy2(binary, executable)
        executable.chmod(0o755)
        subprocess.run(["sh", str(project / "native/webkit-extension/build.sh"), str(executable_directory)], check=True)
        extension = executable_directory / "mixin-webkit-extension/libmixin-webkit-extension.so"
        if elf_architecture(extension) != architecture:
            raise SystemExit("Browser extension architecture does not match the application")
        notices = staged / "usr/share/doc" / IDENTITY
        notices.mkdir(parents=True)
        shutil.copy2(workspace / "mixin_desktop_media/THIRD-PARTY-NOTICES.md", notices / "Media-Third-Party-Notices.md")
        shutil.copy2(project / "assets/emoji.LICENSE", notices / "Emoji-LICENSE.txt")
        library_report = {"application": dependencies(executable), "browser_extension": dependencies(extension), "ffprobe": dependencies(ffprobe)}
        if args.ffprobe:
            shutil.copy2(ffprobe, executable_directory / "ffprobe")
            (executable_directory / "ffprobe").chmod(0o755)
            shutil.copy2(args.ffprobe_notice, notices / "ffprobe.txt")
        applications = staged / "usr/share/applications"
        applications.mkdir(parents=True)
        desktop = applications / f"{IDENTITY}.desktop"
        desktop.write_text(desktop_entry(metadata["name"]), encoding="utf-8")
        shutil.copy2(desktop, staged / desktop.name)
        subprocess.run(["desktop-file-validate", str(desktop)], check=True)
        icons = staged / "usr/share/pixmaps"
        icons.mkdir(parents=True)
        for destination in [icons / f"{IDENTITY}.png", staged / f"{IDENTITY}.png", staged / ".DirIcon"]:
            shutil.copy2(project / "assets/app_icon.png", destination)
        launcher = staged / "AppRun"
        launcher.write_text('''#!/bin/sh
set -eu
HERE=$(CDPATH='' cd -- "$(dirname -- "$(readlink -f -- "$0")")" && pwd)
export XDG_DATA_DIRS="$HERE/usr/share${XDG_DATA_DIRS:+:$XDG_DATA_DIRS}:/usr/local/share:/usr/share"
exec "$HERE/usr/bin/mixin-nsh" "$@"
''', encoding="utf-8")
        launcher.chmod(0o755)
        subprocess.run(["sh", "-n", str(launcher)], check=True)
        profile_generator = staged / "Print-AppArmor-Profile.py"
        shutil.copy2(project / "tool/linux_apparmor.py", profile_generator)
        profile_generator.chmod(0o755)
        report = {"application_id": IDENTITY, "architecture": architecture, "binary_sha256": sha256(binary),
                  "system_ffprobe": args.system_ffprobe, "ffprobe_sha256": sha256(ffprobe),
                  "apparmor_profile_generator": profile_generator.name,
                  "system_dynamic_libraries": library_report, "native_runtime_verified": False}
        (notices / "Package-inputs.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
        staged.rename(output)
    if not args.stage_only:
        archive_name = output.name.removesuffix(".AppDir") + ".tar.gz"
        archive = output.parent / archive_name
        if archive.exists():
            raise SystemExit("The AppDir is ready, but its tar.gz output already exists")
        with tarfile.open(archive, "w:gz") as bundle:
            bundle.add(output, arcname=output.name)
        print(f"Created native Linux archive: {archive}")
    print(f"Staged native Linux AppDir: {output}; installation and desktop runtime verification were not performed")


if __name__ == "__main__":
    main()
