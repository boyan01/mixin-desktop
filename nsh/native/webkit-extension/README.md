# Linux bot browser extension

The app uses GTK4 0.11 and WebKitGTK 6 (`webkit6` 0.6), matching NSH's GTK
dependency. Build on Linux with GTK4 and WebKitGTK 6 development packages,
including `webkitgtk-web-process-extension-6.0.pc` and a C compiler:

```sh
cargo build -p mixin_desktop_ui
sh nsh/native/webkit-extension/build.sh "$(pwd)/target/debug"
```

Package `mixin-webkit-extension/libmixin-webkit-extension.so` alongside the
native executable, preserving that relative directory. This is an app-owned
extension; it does not install a system extension or alter the external NSH SDK.
The runtime refuses to open a bot window if this artifact is absent. The account
anchor still creates its isolated NetworkSession so logout can clear old data.
A missing
or incompatible extension handshake produces `failed` and never enables signing.

Only the default ScriptWorld's native `window-object-cleared` main-frame callback
installs `__mixinNativePost`. The native function captures weak references to the
WebPage, Frame and JSCContext. Every call compares the Frame's current ScriptWorld
context with that captured context, checks main-frame and page ownership, reads
the native Frame URI, and sends a bounded IPC envelope. A retained old closure
cannot borrow the URI of a reused Frame after navigation.

Each web process creates a random UUID epoch. Each committed UI navigation sends
a process-unique UI generation challenge to the current native document. Native
acknowledgement carries its process epoch, document counter, native URI and that
generation. Messages carry the same tuple. The UI accepts no
signing messages before this acknowledgement and rechecks the tuple, committed
WebView URI, account/session ownership and pending request limit. Delayed IPC
from an old document, including reloads at the same URL, cannot establish trust
through a page-supplied token. Reply commands recheck generation, native document,
URL and a pending request ID before evaluating the reply in that document. Native
document announcements have a monotonic counter within their process epoch; an acknowledgement
must match that mark and advance past the preceding navigation's document. An old
acknowledgement cannot overwrite a newer document, even at the same URL. Process
termination retires its epoch and invalidates the UI generation, allowing the next
process to start at counter one while rejecting delayed IPC from the old process.
Same-document SPA URL changes retain document authority. Each message still
compares the native Frame's current URI with the UI WebView's current URI; replies
retain their captured URL so a changed path discards the old in-flight reply.
WebKit's
suspended JavaScript page cache is disabled so Back creates a new document token;
the HTTP resource cache remains enabled. `window.open` navigates the current bot
view, following the Flutter desktop behavior, rather than creating window trees.

Cookies/cache are persistent under a hash of canonical application data root and
account ID. Locking closes windows while the account anchor retains its session.
Logout closes every owned window and waits for the real `WebsiteDataManager`
clear callback. Cancellation, failure or timeout emits `failed`, permitting retry;
only a completed clear emits `purged`. The canonical profile's purge-owner guard
survives errors, timeouts and disposal. Another session key cannot reopen it until
the actual owning clear completes successfully.

macOS type or syntax checks cannot establish Linux linking, extension loading,
WebKit sandbox compatibility, native dialogs, cookies or actual IPC ordering.
Those require a real Linux build and runtime verification. No native Linux
acceptance is claimed by this implementation alone.
