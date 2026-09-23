"""Resolve package versions for GitHub Release publishing workflows."""

from __future__ import annotations

import argparse
import json
import os
import re
import tomllib
from pathlib import Path
from typing import Mapping


_SEMVER = re.compile(r"^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$")


def parse_version(value: str) -> tuple[int, int, int]:
    match = _SEMVER.fullmatch(value)
    if match is None:
        raise ValueError(
            f"Expected a stable SemVer version in MAJOR.MINOR.PATCH form, got {value!r}"
        )
    return tuple(int(part) for part in match.groups())


def normalize_release_tag(tag: str) -> str:
    version = tag[1:] if tag.startswith("v") else tag
    parse_version(version)
    return version


def bump_patch(version: str) -> str:
    major, minor, patch = parse_version(version)
    return f"{major}.{minor}.{patch + 1}"


def read_pyproject_version(path: Path) -> str:
    data = tomllib.loads(path.read_text(encoding="utf-8"))
    version = data["project"]["version"]
    if not isinstance(version, str):
        raise ValueError(f"Expected a static project version in {path}")
    parse_version(version)
    return version


def write_github_output(values: Mapping[str, str]) -> None:
    output_path = os.environ.get("GITHUB_OUTPUT")
    lines = "".join(f"{key}={value}\n" for key, value in values.items())
    if output_path:
        with open(output_path, "a", encoding="utf-8") as output:
            output.write(lines)
    else:
        print(lines, end="")


def resolve_version(event_name: str, release_tag: str, pyproject: Path) -> dict[str, str]:
    if event_name == "release":
        version = normalize_release_tag(release_tag)
        publish_target = "pypi"
        next_version = bump_patch(version)
    elif event_name == "workflow_dispatch":
        version = read_pyproject_version(pyproject)
        publish_target = "testpypi"
        next_version = ""
    else:
        raise ValueError(f"Unsupported publishing event: {event_name!r}")

    return {
        "version": version,
        "next_version": next_version,
        "publish_target": publish_target,
    }


def read_npm_version(path: Path) -> str:
    data = json.loads(path.read_text(encoding="utf-8"))
    version = data["version"]
    if not isinstance(version, str):
        raise ValueError(f"Expected a string package version in {path}")
    parse_version(version)
    return version


def prepare_bump(
    release_next_version: str,
    pyproject: Path,
    npm_package: Path,
) -> dict[str, str]:
    target = parse_version(release_next_version)
    current_python = read_pyproject_version(pyproject)
    current_npm = read_npm_version(npm_package)
    version = max(
        target,
        parse_version(current_python),
        parse_version(current_npm),
    )
    return {
        "version": ".".join(str(part) for part in version),
        "changed": str(
            current_python != ".".join(str(part) for part in version)
            or current_npm != ".".join(str(part) for part in version)
        ).lower(),
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    resolve = subparsers.add_parser("resolve")
    resolve.add_argument("--event-name", required=True)
    resolve.add_argument("--release-tag", default="")
    resolve.add_argument("--pyproject", type=Path, required=True)

    bump = subparsers.add_parser("prepare-bump")
    bump.add_argument("--release-next-version", required=True)
    bump.add_argument("--pyproject", type=Path, required=True)
    bump.add_argument("--npm-package", type=Path, required=True)

    args = parser.parse_args()
    try:
        if args.command == "resolve":
            values = resolve_version(args.event_name, args.release_tag, args.pyproject)
        else:
            values = prepare_bump(
                args.release_next_version,
                args.pyproject,
                args.npm_package,
            )
    except (KeyError, OSError, tomllib.TOMLDecodeError, json.JSONDecodeError, ValueError) as exc:
        parser.error(str(exc))

    write_github_output(values)


if __name__ == "__main__":
    main()
