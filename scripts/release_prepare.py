#!/usr/bin/env python3
"""Plan and prepare a xiu release from a release-request tag.

The script intentionally has no third-party Python dependency so the same
logic can run on a developer machine and in GitHub Actions.  It only changes
release metadata; publishing remains an explicit CI step.
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import re
import subprocess
import sys
import tomllib
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path
from zoneinfo import ZoneInfo


ROOT = Path(__file__).resolve().parents[1]
PACKAGE_FILE = ROOT / "confs/release-packages.toml"
REQUEST_RE = re.compile(r"(?:refs/tags/)?release-request/(v\d+\.\d+\.\d+)$")
SEMVER_RE = re.compile(r"^(\d+)\.(\d+)\.(\d+)$")


class ReleaseError(RuntimeError):
    pass


@dataclass(frozen=True)
class Package:
    name: str
    source: str
    release: str
    manifest: str
    version: str
    dependencies: tuple[str, ...]


def run(*args: str) -> str:
    result = subprocess.run(args, cwd=ROOT, text=True, capture_output=True)
    if result.returncode:
        raise ReleaseError(
            f"command failed: {' '.join(args)}\n{result.stdout}{result.stderr}".rstrip()
        )
    return result.stdout.strip()


def tag_version(tag: str) -> str:
    match = REQUEST_RE.fullmatch(tag)
    if not match:
        raise ReleaseError(
            "tag must be release-request/vMAJOR.MINOR.PATCH "
            f"(received {tag!r})"
        )
    return match.group(1)[1:]


def semver(value: str) -> tuple[int, int, int]:
    match = SEMVER_RE.fullmatch(value)
    if not match:
        raise ReleaseError(f"only stable MAJOR.MINOR.PATCH versions are supported: {value}")
    return tuple(map(int, match.groups()))


def bump_patch(version: str) -> str:
    major, minor, patch = semver(version)
    return f"{major}.{minor}.{patch + 1}"


def read_packages() -> dict[str, Package]:
    with PACKAGE_FILE.open("rb") as fh:
        entries = tomllib.load(fh).get("package", [])
    if not entries:
        raise ReleaseError(f"no packages configured in {PACKAGE_FILE.relative_to(ROOT)}")

    manifests: dict[str, tuple[dict, dict]] = {}
    for entry in entries:
        release = ROOT / entry["release"]
        if not release.is_file():
            raise ReleaseError(f"configured release manifest does not exist: {entry['release']}")
        with release.open("rb") as fh:
            parsed = tomllib.load(fh)
        package = parsed.get("package", {})
        if package.get("name") != entry["name"]:
            raise ReleaseError(
                f"{entry['release']} declares {package.get('name')!r}, "
                f"expected {entry['name']!r}"
            )
        manifests[entry["name"]] = (entry, parsed)

    names = set(manifests)
    packages: dict[str, Package] = {}
    for name, (entry, parsed) in manifests.items():
        deps = parsed.get("dependencies", {})
        internal = tuple(dep for dep in deps if dep in names)
        packages[name] = Package(
            name=name,
            source=entry["source"].rstrip("/"),
            release=entry["release"],
            manifest=entry["manifest"],
            version=parsed["package"]["version"],
            dependencies=internal,
        )
    return packages


def previous_release(tag: str) -> str:
    # The request tag cannot match v*, so asking describe at the request commit
    # returns the nearest preceding formal release (including the case where it
    # was created on the current master tip).
    try:
        return run("git", "describe", "--tags", "--abbrev=0", "--match", "v[0-9]*", tag)
    except ReleaseError as exc:
        raise ReleaseError("no previous formal v* tag is reachable from the request tag") from exc


def changed_paths(base: str, tag: str) -> list[str]:
    output = run("git", "diff", "--name-only", f"{base}..{tag}")
    return [path for path in output.splitlines() if path]


def build_plan(tag: str, packages: dict[str, Package]) -> dict:
    requested_version = tag_version(tag)
    current_xiu = packages["xiu"].version
    if semver(requested_version) <= semver(current_xiu):
        raise ReleaseError(
            f"requested xiu version {requested_version} must be greater than "
            f"the configured version {current_xiu}"
        )

    base = previous_release(tag)
    changed = changed_paths(base, tag)
    direct = {
        package.name
        for package in packages.values()
        if any(path == package.source or path.startswith(package.source + "/") for path in changed)
    }
    # A xiu tag always publishes xiu, even if its own directory did not change.
    direct.add("xiu")

    dependents: dict[str, set[str]] = defaultdict(set)
    for package in packages.values():
        for dependency in package.dependencies:
            dependents[dependency].add(package.name)

    affected = set(direct)
    queue = list(direct)
    while queue:
        dependency = queue.pop()
        for dependent in dependents[dependency]:
            if dependent not in affected:
                affected.add(dependent)
                queue.append(dependent)

    versions = {
        name: (requested_version if name == "xiu" else bump_patch(packages[name].version))
        for name in affected
    }

    # A dependency-first order is required by crates.io. xiu is traversed last
    # so it is always after the libraries it consumes.
    order: list[str] = []
    visited: set[str] = set()

    def visit(name: str) -> None:
        if name in visited:
            return
        visited.add(name)
        for dependency in sorted(packages[name].dependencies):
            if dependency in affected:
                visit(dependency)
        order.append(name)

    for name in sorted(affected - {"xiu"}):
        visit(name)
    visit("xiu")

    return {
        "request_tag": tag,
        "release_tag": f"v{requested_version}",
        "previous_tag": base,
        "changed_paths": changed,
        "direct_packages": sorted(direct),
        "affected_packages": sorted(affected),
        "versions": versions,
        "publish_order": [
            {
                "name": name,
                "version": versions[name],
                "manifest": packages[name].manifest,
                "release_manifest": packages[name].release,
            }
            for name in order
        ],
    }


def replace_in_section(text: str, section: str, key: str, value: str) -> str:
    header = re.compile(rf"(?m)^\[{re.escape(section)}\]\s*$")
    match = header.search(text)
    if not match:
        raise ReleaseError(f"missing [{section}] section")
    next_header = re.compile(r"(?m)^\[").search(text, match.end())
    end = next_header.start() if next_header else len(text)
    body = text[match.end() : end]
    assignment = re.compile(rf'(?m)^(\s*{re.escape(key)}\s*=\s*)"[^"]*"(\s*(?:#.*)?$)')
    if not assignment.search(body):
        raise ReleaseError(f"missing string assignment {key!r} in [{section}]")
    body = assignment.sub(rf'\g<1>"{value}"\2', body, count=1)
    return text[: match.end()] + body + text[end:]


def update_manifest(path: Path, package: Package, versions: dict[str, str]) -> None:
    text = path.read_text()
    if package.name in versions:
        text = replace_in_section(text, "package", "version", versions[package.name])
    # Only release manifests are changed. An unaffected crate remains entirely
    # untouched, avoiding a dependency declaration for a version it does not
    # itself publish in this release.
    if package.name in versions:
        for dependency in package.dependencies:
            if dependency in versions:
                text = replace_in_section(text, "dependencies", dependency, versions[dependency])
    path.write_text(text)


def changelog_entries(plan: dict, package: Package) -> str:
    """Return changelog items appropriate for one published crate."""
    if package.name == "xiu":
        subjects = run(
            "git", "log", "--format=%s", f"{plan['previous_tag']}..{plan['request_tag']}"
        ).splitlines()
    elif package.name in plan["direct_packages"]:
        subjects = run(
            "git",
            "log",
            "--format=%s",
            f"{plan['previous_tag']}..{plan['request_tag']}",
            "--",
            package.source,
        ).splitlines()
    else:
        return "- Update internal dependencies."
    return "\n".join(f"- {subject}" for subject in subjects if subject) or "- Internal release."


def update_changelog(plan: dict, package: Package) -> None:
    if package.name not in plan["versions"]:
        return
    path = ROOT / package.source / "CHANGELOG.md"
    if not path.is_file():
        raise ReleaseError(f"missing CHANGELOG for published package {package.name}: {path.relative_to(ROOT)}")
    text = path.read_text()
    version = plan["versions"][package.name]
    if re.search(rf"(?m)^## \[{re.escape(version)}\]", text):
        raise ReleaseError(f"{package.name} CHANGELOG already has an entry for {version}")
    entries = changelog_entries(plan, package)
    # Releases are maintained in China Standard Time; GitHub-hosted runners
    # use UTC and can otherwise produce the previous calendar date.
    release_date = dt.datetime.now(ZoneInfo("Asia/Shanghai")).date().isoformat()
    heading = f"## [{version}] - {release_date}\n{entries}\n\n"
    unreleased = re.compile(r"(?m)^## \[Unreleased\].*\n")
    match = unreleased.search(text)
    if not match:
        raise ReleaseError("CHANGELOG must contain a ## [Unreleased] heading")
    path.write_text(text[: match.end()] + "\n" + heading + text[match.end() :])


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--tag", required=True, help="release-request/vX.Y.Z tag")
    parser.add_argument("--prepare", action="store_true", help="update manifests and CHANGELOG")
    parser.add_argument("--plan-out", type=Path, help="write the calculated JSON plan")
    args = parser.parse_args()

    try:
        packages = read_packages()
        plan = build_plan(args.tag, packages)
        if args.prepare:
            for package in packages.values():
                update_manifest(ROOT / package.release, package, plan["versions"])
                update_changelog(plan, package)
        encoded = json.dumps(plan, indent=2, sort_keys=True) + "\n"
        if args.plan_out:
            target = args.plan_out if args.plan_out.is_absolute() else ROOT / args.plan_out
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(encoded)
        else:
            print(encoded, end="")
    except ReleaseError as exc:
        print(f"release preparation failed: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
