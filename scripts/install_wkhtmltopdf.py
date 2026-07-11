#!/usr/bin/env python3
"""Install or verify the project-local wkhtmltopdf copy."""

from __future__ import annotations

import argparse
import os
import platform
import shutil
import stat
import sys
from pathlib import Path


PROJECT_ROOT = Path(__file__).resolve().parents[1]
WINDOWS_RENDERER = PROJECT_ROOT / "tools" / "wkhtmltox" / "bin" / "wkhtmltopdf.exe"
LINUX_X64_RENDERER = PROJECT_ROOT / "tools" / "wkhtmltox" / "linux-x64" / "bin" / "wkhtmltopdf"
GENERIC_UNIX_RENDERER = PROJECT_ROOT / "tools" / "wkhtmltox" / "bin" / "wkhtmltopdf"


def expected_renderer() -> Path:
    if sys.platform.startswith("win"):
        return WINDOWS_RENDERER
    if sys.platform.startswith("linux") and platform.machine().lower() in {"x86_64", "amd64"}:
        return LINUX_X64_RENDERER
    return GENERIC_UNIX_RENDERER


def find_source(explicit: str | None) -> Path | None:
    if explicit:
        path = Path(explicit).expanduser().resolve()
        return path if path.is_file() else None

    env_source = os.environ.get("WKHTMLTOPDF_SOURCE")
    if env_source:
        path = Path(env_source).expanduser().resolve()
        return path if path.is_file() else None

    system_copy = shutil.which("wkhtmltopdf")
    return Path(system_copy).resolve() if system_copy else None


def copy_renderer(source: Path, target: Path) -> None:
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, target)
    if not sys.platform.startswith("win"):
        mode = target.stat().st_mode
        target.chmod(mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Verify or seed project-local wkhtmltopdf.")
    parser.add_argument("--source", help="Explicit wkhtmltopdf binary to copy into tools/")
    parser.add_argument("--check", action="store_true", help="Only verify; do not copy")
    args = parser.parse_args(argv)

    target = expected_renderer()
    if target.is_file():
        print(f"OK: {target.relative_to(PROJECT_ROOT)}")
        return 0

    if args.check:
        print(
            f"missing project PDF renderer: {target.relative_to(PROJECT_ROOT)}",
            file=sys.stderr,
        )
        return 1

    source = find_source(args.source)
    if not source:
        print(
            "wkhtmltopdf was not found. Install it or pass --source/ set WKHTMLTOPDF_SOURCE, "
            "then rerun this script to copy it into tools/.",
            file=sys.stderr,
        )
        return 1

    copy_renderer(source, target)
    print(f"Copied {source} to {target.relative_to(PROJECT_ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
