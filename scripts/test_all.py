#!/usr/bin/env python3
"""Run the repo regression suite from one place."""

from __future__ import annotations

import argparse
import os
import shutil
import socket
import subprocess
import sys
import tempfile
import time
from pathlib import Path


PROJECT_ROOT = Path(__file__).resolve().parents[1]
BRUNO_ROOT = PROJECT_ROOT / "bruno" / "document-templating-system-web"


def run(command: list[str], *, cwd: Path = PROJECT_ROOT, env: dict[str, str] | None = None) -> None:
    print(f"+ {' '.join(command)}")
    subprocess.run(command, cwd=cwd, env=env, check=True)


def command_exists(name: str) -> bool:
    return shutil.which(name) is not None


def python_command() -> str:
    return sys.executable


def cargo_command() -> tuple[list[str], dict[str, str]]:
    env = os.environ.copy()
    if sys.platform.startswith("win") and not command_exists("link.exe"):
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


def free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as handle:
        handle.bind(("127.0.0.1", 0))
        return int(handle.getsockname()[1])


def wait_for_server(base_url: str) -> None:
    import urllib.request

    deadline = time.time() + 15
    while time.time() < deadline:
        try:
            with urllib.request.urlopen(f"{base_url}/api/health", timeout=1) as response:
                if response.status == 200:
                    return
        except OSError:
            time.sleep(0.2)
    raise RuntimeError(f"server did not become ready at {base_url}")


def ensure_ci_renderer() -> None:
    if not sys.platform.startswith("linux"):
        return
    run([python_command(), str(PROJECT_ROOT / "scripts" / "install_wkhtmltopdf.py"), "--check"])


def run_bruno() -> None:
    if not command_exists("bru"):
        raise RuntimeError(
            "Bruno CLI is required for regression tests. Install @usebruno/cli so the `bru` command is available."
        )

    with tempfile.TemporaryDirectory(prefix="document-templating-system-bruno-") as temp_name:
        temp_root = Path(temp_name)
        workspace = temp_root / "workspace"

        port = free_port()
        base_url = f"http://127.0.0.1:{port}"
        cargo, env = cargo_command()
        run([*cargo, "run", "--", "--init", str(workspace)], env=env)
        server = subprocess.Popen(
            [
                *cargo,
                "run",
                "--",
                "--workspace",
                str(workspace),
                "--web",
                "--host",
                "127.0.0.1",
                "--port",
                str(port),
                "--no-open",
            ],
            cwd=PROJECT_ROOT,
            env=env,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        try:
            wait_for_server(base_url)
            env_file = temp_root / "bruno-local.bru"
            env_file.write_text(f"vars {{\n  baseUrl: {base_url}\n}}\n", encoding="utf-8")
            run(["bru", "run", "--env-file", str(env_file)], cwd=BRUNO_ROOT, env=os.environ.copy())
        finally:
            server.terminate()
            try:
                server.wait(timeout=5)
            except subprocess.TimeoutExpired:
                server.kill()
                server.wait(timeout=5)


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Run all regression checks.")
    parser.add_argument("--pre-commit", action="store_true", help="Run from the repo git hook.")
    parser.add_argument("--ci", action="store_true", help="Run in CI and require CI-only assets.")
    parser.add_argument("--skip-bruno", action="store_true", help="Skip Bruno API checks.")
    args = parser.parse_args(argv)

    try:
        if args.ci:
            ensure_ci_renderer()

        cargo, cargo_env = cargo_command()
        run([*cargo, "fmt", "--check"], env=cargo_env)
        run([*cargo, "clippy", "--all-targets", "--all-features", "--", "-D", "warnings"], env=cargo_env)
        run([*cargo, "test", "--all-features"], env=cargo_env)
        run([python_command(), "-m", "compileall", "-q", "scripts"])
        run([python_command(), str(PROJECT_ROOT / "scripts" / "package_release.py"), "--check"])
        run(["node", "--check", "web/app.js"])
        if not args.skip_bruno:
            run_bruno()
    except (RuntimeError, subprocess.CalledProcessError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
