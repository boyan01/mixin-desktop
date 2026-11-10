#!/usr/bin/env python3
"""Run the preview scenes in a browser: preview_web.py [--port 8796] (or $PORT)."""

import os
from pathlib import Path
import shutil
import sys

NSH = Path(__file__).resolve().parent.parent


def main():
    if sys.argv[1:2] == ["--cargo"]:
        # NSH always selects its custom entropy backend for web builds. The core
        # enables getrandom's Web Crypto backend for its own browser tests, and
        # getrandom cannot build with both, so use Web Crypto here as well.
        arguments = [
            argument.replace('getrandom_backend=\\"custom\\"',
                             'getrandom_backend=\\"wasm_js\\"')
            for argument in sys.argv[2:]
        ]
        os.execvp("cargo", ["cargo", *arguments])

    env = dict(os.environ)
    llvm = Path("/opt/homebrew/opt/llvm/bin")
    if llvm.is_dir():
        env["PATH"] = f"{llvm}{os.pathsep}{env.get('PATH', '')}"
    if not shutil.which("llvm-ar", path=env.get("PATH")):
        sys.exit("llvm-ar is required. Install LLVM and add its bin directory to PATH.")
    env["CARGO_TARGET_DIR"] = str(NSH.parent / "target")
    # Preview scenes are debug-only. NSH's web-dev profile disables assertions
    # by default, unlike its native dev builds.
    env["CARGO_PROFILE_WEB_DEV_DEBUG_ASSERTIONS"] = "true"
    wrapper = NSH / "preview/.nsh/cargo"
    wrapper.parent.mkdir(parents=True, exist_ok=True)
    wrapper.write_text(
        f"#!{sys.executable}\n"
        "import os\n"
        f"os.execv({sys.executable!r}, [{sys.executable!r}, "
        f"{str(Path(__file__).resolve())!r}, '--cargo', *__import__('sys').argv[1:]])\n"
    )
    wrapper.chmod(0o755)
    env["CARGO"] = str(wrapper)
    os.chdir(NSH / "preview")
    arguments = sys.argv[1:] or ["--port", env.get("PORT", "8796")]
    os.execvpe("nsh", ["nsh", "run", "--target", "web", *arguments], env)


if __name__ == "__main__":
    main()
