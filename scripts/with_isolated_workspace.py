#!/usr/bin/env python3
"""Run one Cargo command with exactly one workspace member.

Cargo resolves every workspace member even when `--manifest-path` selects one
package. During a chained release that makes a dependent crate require a
version which is planned but not yet on crates.io. This helper temporarily
reduces the root workspace to the requested crate, then restores it even when
the Cargo command fails. It is intended for the disposable CI checkout only.
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
ROOT_MANIFEST = ROOT / "Cargo.toml"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("manifest", help="crate Cargo.toml, relative to repository root")
    parser.add_argument("cargo_args", nargs=argparse.REMAINDER, help="Cargo subcommand and its arguments")
    args = parser.parse_args()

    if not args.cargo_args:
        parser.error("a Cargo subcommand is required")

    manifest = (ROOT / args.manifest).resolve()
    try:
        member = manifest.parent.relative_to(ROOT)
    except ValueError:
        parser.error("manifest must be inside the repository")
    if manifest.name != "Cargo.toml" or not manifest.is_file():
        parser.error(f"manifest does not exist: {args.manifest}")

    original = ROOT_MANIFEST.read_bytes()
    isolated = (
        "[workspace]\n"
        'resolver = "1"\n'
        f'members = ["{member.as_posix()}"]\n\n'
        "[profile.release]\n"
        "codegen-units = 1\n"
        "lto = true\n"
    ).encode()

    try:
        ROOT_MANIFEST.write_bytes(isolated)
        result = subprocess.run(
            ["cargo", *args.cargo_args, "--manifest-path", str(manifest)], cwd=ROOT
        )
        return result.returncode
    finally:
        ROOT_MANIFEST.write_bytes(original)


if __name__ == "__main__":
    raise SystemExit(main())
