#!/usr/bin/env python3
"""Build/check self-contained document-templating-system release archives."""

from __future__ import annotations

import argparse
import os
import platform
import shutil
import subprocess
import sys
import tarfile
import zipfile
from pathlib import Path


PROJECT_ROOT = Path(__file__).resolve().parents[1]
DIST_DIR = PROJECT_ROOT / "dist"


def platform_name() -> str:
    system = platform.system().lower()
    machine = platform.machine().lower()
    if system == "windows":
        return "windows-x64"
    if system == "linux" and machine in {"x86_64", "amd64"}:
        return "linux-x64"
    return f"{system}-{machine}"


def binary_path() -> Path:
    name = "document-templating-system.exe" if sys.platform.startswith("win") else "document-templating-system"
    return PROJECT_ROOT / "target" / "release" / name


def renderer_path() -> Path:
    if sys.platform.startswith("win"):
        return PROJECT_ROOT / "tools" / "wkhtmltox" / "bin" / "wkhtmltopdf.exe"
    if sys.platform.startswith("linux"):
        return PROJECT_ROOT / "tools" / "wkhtmltox" / "linux-x64" / "bin" / "wkhtmltopdf"
    return PROJECT_ROOT / "tools" / "wkhtmltox" / "bin" / "wkhtmltopdf"


def cargo_command() -> tuple[list[str], dict[str, str]]:
    env = os.environ.copy()
    if sys.platform.startswith("win") and shutil.which("link.exe") is None:
        toolchain = Path.home() / ".rustup" / "toolchains" / "stable-x86_64-pc-windows-gnu" / "bin"
        cargo = toolchain / "cargo.exe"
        rustc = toolchain / "rustc.exe"
        rustdoc = toolchain / "rustdoc.exe"
        if cargo.is_file() and rustc.is_file() and rustdoc.is_file():
            env["PATH"] = f"{toolchain}{os.pathsep}{env.get('PATH', '')}"
            env["CARGO_BUILD_TARGET"] = "x86_64-pc-windows-gnu"
            env["RUSTC"] = str(rustc)
            env["RUSTDOC"] = str(rustdoc)
            return [str(cargo)], env
    return ["cargo"], env


def check_inputs(require_binary: bool) -> None:
    if require_binary and not binary_path().is_file():
        raise SystemExit(f"missing release binary: {binary_path()}")
    if not renderer_path().is_file():
        raise SystemExit(f"missing project PDF renderer: {renderer_path()}")


def files_for_archive() -> list[tuple[Path, str]]:
    binary = binary_path()
    renderer = renderer_path()
    files = [
        (binary, binary.name),
        (PROJECT_ROOT / "README.md", "README.md"),
    ]
    for source in sorted(renderer.parent.glob("*")):
        if source.is_file():
            files.append((source, str(source.relative_to(PROJECT_ROOT)).replace("\\", "/")))
    return files


def build_archive() -> Path:
    check_inputs(require_binary=True)
    DIST_DIR.mkdir(parents=True, exist_ok=True)
    name = f"document-templating-system-{platform_name()}"
    if sys.platform.startswith("win"):
        archive = DIST_DIR / f"{name}.zip"
        with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as handle:
            for source, target in files_for_archive():
                handle.write(source, f"{name}/{target}")
    else:
        archive = DIST_DIR / f"{name}.tar.gz"
        with tarfile.open(archive, "w:gz") as handle:
            for source, target in files_for_archive():
                handle.add(source, f"{name}/{target}")
    return archive


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Package document-templating-system release archives.")
    parser.add_argument("--build", action="store_true", help="Run cargo build --release first.")
    parser.add_argument("--check", action="store_true", help="Only check package inputs.")
    args = parser.parse_args(argv)

    if args.build:
        cargo, env = cargo_command()
        subprocess.run([*cargo, "build", "--release"], cwd=PROJECT_ROOT, env=env, check=True)

    if args.check:
        check_inputs(require_binary=False)
        print(f"OK: package inputs for {platform_name()}")
        return 0

    archive = build_archive()
    print(f"Wrote {archive}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
