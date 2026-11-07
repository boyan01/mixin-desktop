#!/usr/bin/env python3
"""Build and stage the native Windows app, then create an unsigned MSIX."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import struct
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ET

IDENTITY = "one.mixin.messenger.nsh"
APPLICATION = "MixinMessenger"
ACTIVATOR = "C7DFB9DF-1F4C-48D8-A3D5-8BDA631A17EC"
FOUNDATION = "http://schemas.microsoft.com/appx/manifest/foundation/windows10"
UAP = "http://schemas.microsoft.com/appx/manifest/uap/windows10"
COM = "http://schemas.microsoft.com/appx/manifest/com/windows10"
DESKTOP = "http://schemas.microsoft.com/appx/manifest/desktop/windows10"
RESTRICTED = "http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities"

# Reviewed Windows OS imports used by the desktop/media runtime. Mere presence
# in the build host's System32 does not make a third-party library an OS input.
# Unknown dependencies must be supplied explicitly and recursively validated.
WINDOWS_OS_DLLS = frozenset("""
advapi32.dll avicap32.dll avifil32.dll avrt.dll bcrypt.dll bcryptprimitives.dll
cabinet.dll cfgmgr32.dll combase.dll comctl32.dll comdlg32.dll coremessaging.dll
credui.dll crypt32.dll cryptbase.dll cryptnet.dll cryptsp.dll cryptui.dll d2d1.dll
d3d9.dll d3d10.dll d3d10_1.dll d3d11.dll d3d12.dll d3dcompiler_47.dll dbghelp.dll
dcomp.dll devobj.dll dnsapi.dll dsound.dll dwmapi.dll dwrite.dll dxgi.dll dxva2.dll
evr.dll gdi32.dll gdi32full.dll gdiplus.dll glu32.dll hid.dll imagehlp.dll imm32.dll
iphlpapi.dll kernel32.dll kernelbase.dll ksuser.dll mf.dll mfcore.dll mfplat.dll
mfreadwrite.dll mmdevapi.dll mpr.dll msacm32.dll msctf.dll msimg32.dll msvcp_win.dll
msvcrt.dll msvfw32.dll mswsock.dll ncrypt.dll netapi32.dll normaliz.dll ntdll.dll
ole32.dll oleacc.dll oleaut32.dll opengl32.dll powrprof.dll propsys.dll psapi.dll
rpcrt4.dll rtworkq.dll schannel.dll sechost.dll secur32.dll setupapi.dll shell32.dll
shcore.dll shlwapi.dll sspicli.dll ucrtbase.dll uiautomationcore.dll urlmon.dll
user32.dll userenv.dll usp10.dll uxtheme.dll version.dll windowscodecs.dll
winhttp.dll wininet.dll winmm.dll wintrust.dll wldap32.dll ws2_32.dll wtsapi32.dll
""".split())


def manifest(metadata, publisher, publisher_name, architecture):
    for prefix, uri in [("", FOUNDATION), ("uap", UAP), ("com", COM),
                        ("desktop", DESKTOP), ("rescap", RESTRICTED)]:
        ET.register_namespace(prefix, uri)
    def add(parent, name, attributes=None, text=None):
        node = ET.SubElement(parent, name, attributes or {})
        node.text = text
        return node
    def qualified(namespace, name):
        return "{" + namespace + "}" + name
    version = metadata["version"].split(".") + [str(metadata["build"])]
    if len(version) != 4 or any(not part.isdigit() or int(part) > 65535 for part in version):
        raise ValueError("nsh.json version and build must form four 16-bit numeric components")
    package = ET.Element(qualified(FOUNDATION, "Package"),
                         {"IgnorableNamespaces": "uap com desktop rescap"})
    add(package, qualified(FOUNDATION, "Identity"), {
        "Name": IDENTITY, "Publisher": publisher, "Version": ".".join(version),
        "ProcessorArchitecture": architecture,
    })
    properties = add(package, qualified(FOUNDATION, "Properties"))
    for key, value in [("DisplayName", metadata["name"]), ("PublisherDisplayName", publisher_name),
                       ("Logo", r"Assets\StoreLogo.png")]:
        add(properties, qualified(FOUNDATION, key), text=value)
    dependencies = add(package, qualified(FOUNDATION, "Dependencies"))
    add(dependencies, qualified(FOUNDATION, "TargetDeviceFamily"), {
        "Name": "Windows.Desktop", "MinVersion": "10.0.17763.0", "MaxVersionTested": "10.0.26100.0",
    })
    resources = add(package, qualified(FOUNDATION, "Resources"))
    add(resources, qualified(FOUNDATION, "Resource"), {"Language": "en-us"})
    applications = add(package, qualified(FOUNDATION, "Applications"))
    application = add(applications, qualified(FOUNDATION, "Application"), {
        "Id": APPLICATION, "Executable": "MixinMessenger.exe", "EntryPoint": "Windows.FullTrustApplication",
    })
    add(application, qualified(UAP, "VisualElements"), {
        "DisplayName": metadata["name"], "Description": metadata["name"], "BackgroundColor": "transparent",
        "Square150x150Logo": r"Assets\Square150x150Logo.png", "Square44x44Logo": r"Assets\Square44x44Logo.png",
    })
    extensions = add(application, qualified(FOUNDATION, "Extensions"))
    extension = add(extensions, qualified(UAP, "Extension"), {"Category": "windows.protocol"})
    protocol = add(extension, qualified(UAP, "Protocol"), {"Name": "mixin"})
    add(protocol, qualified(UAP, "DisplayName"), text=metadata["name"])
    extension = add(extensions, qualified(COM, "Extension"), {"Category": "windows.comServer"})
    server = add(extension, qualified(COM, "ComServer"))
    executable = add(server, qualified(COM, "ExeServer"), {
        "Executable": "MixinMessenger.exe", "Arguments": "--mixin-toast-activation",
        "DisplayName": metadata["name"],
    })
    add(executable, qualified(COM, "Class"), {"Id": ACTIVATOR, "DisplayName": metadata["name"]})
    extension = add(extensions, qualified(DESKTOP, "Extension"), {"Category": "windows.toastNotificationActivation"})
    add(extension, qualified(DESKTOP, "ToastNotificationActivation"), {"ToastActivatorCLSID": ACTIVATOR})
    capabilities = add(package, qualified(FOUNDATION, "Capabilities"))
    for name in ["internetClient", "privateNetworkClientServer"]:
        add(capabilities, qualified(FOUNDATION, "Capability"), {"Name": name})
    add(capabilities, qualified(RESTRICTED, "Capability"), {"Name": "runFullTrust"})
    add(capabilities, qualified(FOUNDATION, "DeviceCapability"), {"Name": "microphone"})
    ET.indent(package)
    return ET.tostring(package, encoding="utf-8", xml_declaration=True)


def pe_architecture(path):
    with path.open("rb") as binary:
        header = binary.read(64)
        if len(header) != 64 or header[:2] != b"MZ":
            raise ValueError(f"Not a Windows executable or library: {path}")
        offset = struct.unpack_from("<I", header, 60)[0]
        if offset > 1024 * 1024:
            raise ValueError(f"Invalid PE header offset: {path}")
        binary.seek(offset)
        header = binary.read(6)
    if len(header) != 6 or header[:4] != b"PE\0\0":
        raise ValueError(f"Invalid PE header: {path}")
    architecture = {0x8664: "x64", 0xAA64: "arm64"}.get(struct.unpack_from("<H", header, 4)[0])
    if architecture is None:
        raise ValueError(f"Unsupported PE architecture: {path}")
    return architecture


def sha256(path):
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def copy_dependencies(binaries, destination, directories, dumpbin, architecture):
    pending = list(binaries)
    copied = {}
    system = set()
    checked = set()
    while pending:
        binary = pending.pop()
        if binary in checked:
            continue
        checked.add(binary)
        output = subprocess.check_output([dumpbin, "/NOLOGO", "/DEPENDENTS", str(binary)], text=True,
                                         errors="replace", timeout=30)
        names = re.findall(r"^\s+([^\s]+\.dll)\s*$", output, re.MULTILINE | re.IGNORECASE)
        for name in names:
            lower = name.lower()
            system_file = Path(os.environ["SystemRoot"]) / "System32" / name
            if lower.startswith(("api-ms-win-", "ext-ms-win-")) or (lower in WINDOWS_OS_DLLS and system_file.is_file()):
                system.add(name)
                continue
            candidates = [folder / name for folder in [binary.parent, *directories] if (folder / name).is_file()]
            if not candidates:
                raise ValueError(f"Unbundled dependency {name}; supply its directory with --dll-dir")
            source = candidates[0].resolve()
            if pe_architecture(source) != architecture:
                raise ValueError(f"Dependency architecture does not match {architecture}: {source}")
            digest = sha256(source)
            if lower in copied:
                if copied[lower]["sha256"] != digest:
                    raise ValueError(f"Conflicting dependency versions: {name}")
                continue
            shutil.copy2(source, destination / name)
            copied[lower] = {"name": name, "sha256": digest}
            pending.append(source)
    return list(copied.values()), sorted(system)


def icons(source, destination):
    # The script is fixed code; paths are passed as separate -File parameters.
    script = r"""param([string]$Source, [string]$Destination)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$inputImage = [System.Drawing.Image]::FromFile($Source)
try {
  foreach ($entry in @(@('Square44x44Logo.png',44), @('Square150x150Logo.png',150), @('StoreLogo.png',50))) {
    $image = [System.Drawing.Bitmap]::new([int]$entry[1], [int]$entry[1])
    $graphics = [System.Drawing.Graphics]::FromImage($image)
    try {
      $graphics.Clear([System.Drawing.Color]::Transparent)
      $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
      $graphics.DrawImage($inputImage, 0, 0, [int]$entry[1], [int]$entry[1])
      $image.Save((Join-Path $Destination $entry[0]), [System.Drawing.Imaging.ImageFormat]::Png)
    } finally { $graphics.Dispose(); $image.Dispose() }
  }
} finally { $inputImage.Dispose() }
"""
    with tempfile.TemporaryDirectory(prefix="mixin-windows-assets-") as temporary:
        path = Path(temporary) / "assets.ps1"
        path.write_text(script, encoding="utf-8")
        subprocess.run(["powershell.exe", "-NoProfile", "-NonInteractive", "-File", str(path),
                        "-Source", str(source), "-Destination", str(destination)], check=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--publisher", required=True, help="Certificate subject for the eventual package signer")
    parser.add_argument("--publisher-name", default="Mixin")
    parser.add_argument("--target", choices=["x86_64-pc-windows-msvc", "aarch64-pc-windows-msvc"], default="x86_64-pc-windows-msvc")
    parser.add_argument("--release", action="store_true")
    parser.add_argument("--binary", type=Path, help="Use an existing real executable instead of building")
    parser.add_argument("--ffprobe", required=True, type=Path)
    parser.add_argument("--ffprobe-notice", required=True, type=Path)
    parser.add_argument("--dll-dir", action="append", default=[], type=Path)
    parser.add_argument("--dumpbin", default="dumpbin")
    parser.add_argument("--makeappx", default="MakeAppx.exe")
    parser.add_argument("--stage-only", action="store_true", help="Validate and stage without invoking MakeAppx")
    parser.add_argument("--out", type=Path, help="New output directory; existing directories are never replaced")
    args = parser.parse_args()
    if sys.platform != "win32":
        raise SystemExit("Windows packaging requires Windows, MSVC/Windows SDK and Windows PowerShell")
    if not args.publisher.strip() or any(ord(character) < 32 for character in args.publisher):
        raise SystemExit("--publisher must be a nonempty certificate subject")
    project = Path(__file__).resolve().parents[1]
    workspace = project.parent
    metadata = json.loads((project / "nsh.json").read_text())
    if metadata["id"] != IDENTITY:
        raise SystemExit("The notification namespace requires the NSH application identity")
    for path in [args.ffprobe, args.ffprobe_notice]:
        if not path.is_file():
            raise SystemExit(f"Required packaging input is missing: {path}")
    if not shutil.which(args.dumpbin):
        raise SystemExit("dumpbin is required; run from the Visual Studio Developer Prompt or use --dumpbin")
    if not args.stage_only and not shutil.which(args.makeappx):
        raise SystemExit("MakeAppx is required; use the Windows SDK tool or explicitly choose --stage-only")
    output = (args.out or project / ".nsh/out/windows" / (metadata["version"] + "-" + args.target)).resolve()
    if output.exists():
        raise SystemExit("Refusing to replace an existing output directory; choose a new --out")
    if args.binary is None:
        command = ["cargo", "build", "-p", "mixin_desktop_ui", "--bin", "mixin_desktop_ui", "--target", args.target]
        if args.release:
            command.append("--release")
        subprocess.run(command, cwd=workspace, check=True)
        cargo = json.loads(subprocess.check_output(["cargo", "metadata", "--no-deps", "--format-version", "1"], cwd=workspace))
        binary = Path(cargo["target_directory"]) / args.target / ("release" if args.release else "debug") / "mixin_desktop_ui.exe"
    else:
        binary = args.binary.resolve()
    architecture = pe_architecture(binary)
    if architecture != ("arm64" if args.target.startswith("aarch64") else "x64") or pe_architecture(args.ffprobe) != architecture:
        raise SystemExit("The app and ffprobe architectures must match --target")
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="mixin-windows-package-", dir=output.parent) as temporary:
        staged = Path(temporary) / "App"
        (staged / "Assets").mkdir(parents=True)
        (staged / "Notices").mkdir()
        shutil.copy2(binary, staged / "MixinMessenger.exe")
        shutil.copy2(args.ffprobe, staged / "ffprobe.exe")
        shutil.copy2(args.ffprobe_notice, staged / "Notices/ffprobe.txt")
        shutil.copy2(workspace / "mixin_desktop_media/THIRD-PARTY-NOTICES.md", staged / "Notices/Media-Third-Party-Notices.md")
        shutil.copy2(project / "assets/emoji.LICENSE", staged / "Notices/Emoji-LICENSE.txt")
        dlls, system = copy_dependencies([binary, args.ffprobe.resolve()], staged,
                                        [*map(Path.resolve, args.dll_dir), binary.parent, args.ffprobe.resolve().parent],
                                        args.dumpbin, architecture)
        icons(project / "assets/app_icon.png", staged / "Assets")
        (staged / "AppxManifest.xml").write_bytes(manifest(metadata, args.publisher, args.publisher_name, architecture))
        report = {"application_id": IDENTITY, "architecture": architecture, "binary_sha256": sha256(binary),
                  "ffprobe_sha256": sha256(args.ffprobe), "bundled_dlls": dlls, "system_dlls": system,
                  "signed": False, "native_runtime_verified": False}
        (staged / "Package-inputs.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
        staged.rename(output)
    if not args.stage_only:
        package = output.parent / (output.name + ".msix")
        if package.exists():
            raise SystemExit("The staged app is ready, but the .msix output already exists")
        subprocess.run([args.makeappx, "pack", "/d", str(output), "/p", str(package)], check=True)
        print(f"Created unsigned package: {package}")
    print(f"Staged native Windows app: {output}; installation, signing and runtime verification were not performed")


if __name__ == "__main__":
    main()
