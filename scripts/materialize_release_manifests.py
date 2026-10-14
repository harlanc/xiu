#!/usr/bin/env python3
"""Temporarily materialize release manifests for a crates.io publish job.

This must only run in a disposable CI checkout. Repository Cargo.toml files
retain local path dependencies and are never changed in a release commit.
"""

from __future__ import annotations

import argparse
import shutil
import sys
from pathlib import Path

from release_prepare import ROOT, ReleaseError, read_packages


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("mode", choices=("release",))
    parser.add_argument("--dry-run", action="store_true", help="validate and print copies without writing")
    args = parser.parse_args()

    try:
        for package in read_packages().values():
            source = ROOT / "confs" / args.mode / Path(package.release).name
            target = ROOT / package.manifest
            if not source.is_file():
                raise ReleaseError(f"missing release manifest for {package.name}: {source.relative_to(ROOT)}")
            print(f"{source.relative_to(ROOT)} -> {target.relative_to(ROOT)}")
            if not args.dry_run:
                shutil.copyfile(source, target)
    except ReleaseError as exc:
        print(f"configuration sync failed: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
