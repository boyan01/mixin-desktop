# Native desktop packaging

These tools package the app's own Rust entry point. They do not use the NSH CLI's
generic runner, install an app, register a user-selected default protocol handler,
create a signing certificate, or publish a release. Existing output directories are
never replaced. Use a new `--out` for each build.

Use Python 3.9 or later. Run the Windows tool on Windows and the Linux tool on Linux.
A Mac type check or a generated manifest does not prove a Windows or Linux package works. `--stage-only`
still builds or validates a real target binary, checks native dependencies and
creates all resources; it only skips final archive creation.

## Windows

Use a Visual Studio Developer Prompt with Rust, the MSVC C++ workload, Windows SDK
(`dumpbin` and `MakeAppx.exe`) and Windows PowerShell. WebView2 Evergreen Runtime is
a runtime prerequisite for embedded bot windows. The app reports native WebView2
initialization failure if that runtime is absent; the packager does not install it.

```powershell
python nsh/tool/package_windows.py --release `
  --publisher "CN=Your distributor" --publisher-name "Your distributor" `
  --ffprobe C:\distribution-inputs\ffprobe.exe `
  --ffprobe-notice C:\distribution-inputs\ffprobe-LICENSE.txt `
  --dll-dir C:\distribution-inputs\runtime `
  --out C:\artifacts\MixinMessenger-0.1.0-x64
```

The publisher must match the certificate used for later signing. This command only
creates an **unsigned** MSIX and its reviewable staging directory. Installing a
package, signing it with `SignTool`, and any Store submission are separate actions.
Use `--target aarch64-pc-windows-msvc` only with its real toolchain and matching ARM64
ffprobe/DLL inputs. `--binary` reuses an already built PE executable; it does not
bypass architecture or dependency validation.

`dumpbin /DEPENDENTS` is checked recursively for the app, ffprobe and bundled DLLs.
Only API sets and a reviewed Windows OS import list remain system dependencies.
Presence in the build host’s System32 does not classify an unknown DLL as part of
Windows. Unknown imports must be explicitly supplied or staging fails, including
when the build host happens to have that third-party library installed. VC runtime DLLs must be supplied
beside an input binary or through `--dll-dir`; they are not silently assumed to exist
on the recipient's machine. Missing or mismatched DLLs fail staging. This check cannot
discover arbitrary libraries loaded by name at runtime, such as WebView2 itself;
native execution remains part of package acceptance.

The generated manifest declares:

- Identity `one.mixin.messenger.nsh` and application `MixinMessenger`.
- The `mixin:` protocol without choosing it as the user's default.
- Toast activator CLSID `C7DFB9DF-1F4C-48D8-A3D5-8BDA631A17EC` and the fixed
  `--mixin-toast-activation` COM server argument. The same executable receives the
  activation before ordinary single-instance forwarding.
- Internet, local-network, microphone and full-trust desktop capabilities.

The COM and toast declarations follow Microsoft's [packaged desktop toast sample](https://github.com/WindowsNotifications/desktop-toasts/blob/master/CPP-WRL/DesktopToastsCppWrlPackageProject/Package.appxmanifest).
The native adapter obtains its actual application identity from Windows; it does
not invent an AUMID or write runtime registry/shortcut registrations.

## Linux

The native build needs Rust, C/C++ compilers, CMake, `protoc`, `pkg-config`,
`desktop-file-validate`,
GTK 4.14 or later, WebKitGTK 6.0, WebKit's web-process-extension development package,
ALSA and libdbus development files. A desktop session also needs a notification
service and, for screen-lock credentials, a Secret Service provider. If secure
storage is temporarily unavailable, the app keeps protected content covered and
offers retry; an offline provider is not treated as an empty credential store.
Video playback needs the distribution's GTK media backend and appropriate codecs.

The coordinator validated Ubuntu 24.04 ARM with Rust 1.97.0, GTK 4.14.5 and
WebKitGTK 2.52.6. `protobuf-compiler` supplies `protoc` for the shared Signal
protocol build; it is a build dependency, not a packaged application binary.

```sh
python3 nsh/tool/package_linux.py --release --system-ffprobe \
  --out /absolute/artifacts/MixinMessenger-0.1.0-x86_64.AppDir

# Or bundle an explicitly supplied native ffprobe and its distributor's notices:
python3 nsh/tool/package_linux.py --release \
  --ffprobe /absolute/inputs/ffprobe \
  --ffprobe-notice /absolute/inputs/ffprobe-LICENSE.txt \
  --out /absolute/artifacts/MixinMessenger-bundled-media.AppDir
```

The tool produces an AppDir and `tar.gz`. This is a native distribution archive,
**not a universal AppImage**: GTK/WebKit/GLib/ALSA/D-Bus and other dynamic dependencies
remain distribution packages. `ldd` must resolve every dependency on the build host,
and the resolved dependency inventory is recorded in `Package-inputs.json`. Build
and run on the intended distribution/ABI; do not treat this inventory as proof of
compatibility with every Linux distribution.

The WebKit extension is built from app-owned C source with the installed WebKit
headers. The app and extension are staged together:

```text
MixinMessenger.AppDir/
  AppRun
  Print-AppArmor-Profile.py               # optional, prints policy without loading it
  one.mixin.messenger.nsh.desktop
  usr/bin/mixin-nsh
  usr/bin/ffprobe                         # only when explicitly bundled
  usr/bin/mixin-webkit-extension/libmixin-webkit-extension.so
  usr/share/applications/one.mixin.messenger.nsh.desktop
  usr/share/pixmaps/one.mixin.messenger.nsh.png
  usr/share/doc/one.mixin.messenger.nsh/   # notices and dependency inventory
```

Launch `AppRun` from any working directory. A distributor may later install the
launcher as `mixin-nsh` and the desktop entry into its desktop/MIME database. This
tool does not copy into system/user application directories, run
`update-desktop-database`, or set a default `mixin:` handler. The desktop entry's
`Exec=mixin-nsh %U` forwards actual URI arguments to the same native instance.

### AppArmor and WebKit's sandbox

Ubuntu systems that restrict unprivileged user namespaces can reject WebKit's
`bwrap` sandbox setup. On the tested Ubuntu 24.04 host this made WebKit fail before
creating its web process. Keep WebKit's sandbox and the system namespace
restriction enabled. If this host requires an application policy, the optional
generator prints one for the AppDir's actual canonical executable:

```sh
python3 /absolute/MixinMessenger.AppDir/Print-AppArmor-Profile.py > /tmp/mixin-nsh.apparmor
cat /tmp/mixin-nsh.apparmor
# After reviewing its single exact executable attachment:
sudo apparmor_parser -r /tmp/mixin-nsh.apparmor

# Remove that same loaded policy when it is no longer needed:
sudo apparmor_parser -R /tmp/mixin-nsh.apparmor
```

The ABI 4.0 profile retains the application's ordinary unconfined permissions and
allows user-namespace creation for that attachment. It does not grant a global
exception to `bwrap`, disable AppArmor or install a policy automatically. Its name
is a stable SHA-256 of the canonical executable path. Paths are escaped as literal
AppArmor attachments; control/formatting characters and internal symlink targets
are rejected. The helper needs Python 3 only when explicitly invoked.

Keep the generated file to unload the same profile. After moving the AppDir,
unload its old policy, regenerate for the new canonical path and review it again.
These commands load/remove runtime policy; they do not install boot-persistent
configuration. A distributor that installs a fixed executable path can separately
review and install the corresponding policy through its package manager.

## Media and verification boundaries

The current nonmacOS video preparation path invokes **ffprobe**. FFmpeg itself is
used to author test fixtures, and is not an additional application runtime
requirement. Bundling ffprobe requires its actual distributor's license notices and
matching binaries; the packagers neither download an unknown build nor infer its
licensing configuration. The media crate's existing third-party notices and the
embedded emoji data's license are copied as well.

Package generation is not runtime acceptance. Verify installation identity, native
startup, cold/warm protocol and toast activation, notification withdrawal, WebView
profile isolation/purge, video preparation and process shutdown on each target.
Signing, installer behavior, deployment and original user-data import are not
claimed by a successful staging or archive command.
