#!/usr/bin/env python3
"""Print an exact-path AppArmor user-namespace policy for this AppDir."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
import unicodedata

IDENTITY = "one.mixin.messenger.nsh"


def quoted_attachment(path):
    value = str(path)
    if any(unicodedata.category(character) in {"Cc", "Cf", "Cs"} for character in value):
        raise ValueError("The executable path contains control or formatting characters")
    # Quotes alone do not make an AppArmor attachment literal. Hex escapes keep
    # AARE operators and variable syntax literal, including inside quoted paths.
    special = '\\"*?[]{}@^'
    escaped = "".join(f"\\x{ord(character):02x}" if character in special else character
                      for character in value)
    return '"' + escaped + '"'


def profile(appdir):
    root = appdir.resolve(strict=True)
    quoted_attachment(root)
    binary = root / "usr/bin/mixin-nsh"
    executable = binary.resolve(strict=True)
    if executable != binary or not executable.is_file() or not os.access(executable, os.X_OK):
        raise ValueError("Expected a regular executable at this AppDir's usr/bin/mixin-nsh")
    with executable.open("rb") as source:
        header = source.read(20)
    if (len(header) != 20 or header[:6] != b"\x7fELF\x02\x01"
            or struct.unpack_from("<H", header, 18)[0] not in {62, 183}):
        raise ValueError("Expected a native 64-bit Linux ELF executable")
    inputs = root / "usr/share/doc" / IDENTITY / "Package-inputs.json"
    metadata = json.loads(inputs.read_text(encoding="utf-8"))
    if not isinstance(metadata, dict) or metadata.get("application_id") != IDENTITY:
        raise ValueError("This AppDir does not have the Mixin application identity")
    attachment = quoted_attachment(executable)
    name = "mixin-nsh-userns-" + hashlib.sha256(os.fsencode(executable)).hexdigest()
    return f"""# Generated for this AppDir's canonical executable. Regenerate after moving it.
# Keep this file to remove the same profile with apparmor_parser -R.
abi <abi/4.0>,
profile {name} {attachment} flags=(unconfined) {{
  userns,
}}
"""


def main():
    argparse.ArgumentParser(description=__doc__).parse_args()
    try:
        print(profile(Path(__file__).resolve(strict=True).parent), end="")
    except (OSError, ValueError) as error:
        raise SystemExit(f"Cannot create AppArmor profile: {error}") from error


if __name__ == "__main__":
    main()
