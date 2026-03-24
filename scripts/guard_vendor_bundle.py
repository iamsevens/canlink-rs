#!/usr/bin/env python3
"""Guard against shipping LibTSCAN vendor binaries in release artifacts."""

from __future__ import annotations

import subprocess
import sys
import tarfile
from pathlib import Path

FORBIDDEN_NAMES = {
    "libtscan.dll",
    "libtscan.lib",
    "libtscan.so",
    "libtscan.dylib",
}

PUBLISH_CRATES = [
    "canlink-hal",
    "canlink-tscan-sys",
    "canlink-tscan",
    "canlink-cli",
]


def repo_root() -> Path:
    return Path(__file__).resolve().parents[1]


def run(cmd: list[str], cwd: Path) -> subprocess.CompletedProcess:
    return subprocess.run(
        cmd,
        cwd=cwd,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        encoding="utf-8",
        errors="replace",
        check=False,
    )


def check_git_tracked(root: Path) -> list[str]:
    result = subprocess.run(
        ["git", "ls-files", "-z"],
        cwd=root,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(
            "git ls-files failed:\n" + result.stderr.decode("utf-8", "replace")
        )

    bad: list[str] = []
    for raw in result.stdout.split(b"\0"):
        if not raw:
            continue
        path = raw.decode("utf-8", "replace")
        name = Path(path).name.lower()
        if name in FORBIDDEN_NAMES:
            bad.append(path)
    return bad


def ensure_package(root: Path, crate: str) -> Path:
    package_dir = root / "target" / "package"
    package_dir.mkdir(parents=True, exist_ok=True)

    result = run(
        ["cargo", "package", "-p", crate, "--allow-dirty", "--no-verify"], root
    )
    if result.returncode != 0:
        raise RuntimeError(
            f"cargo package failed for {crate}:\n{result.stdout}"
        )

    candidates = sorted(
        package_dir.glob(f"{crate}-*.crate"),
        key=lambda p: p.stat().st_mtime,
        reverse=True,
    )
    if not candidates:
        raise RuntimeError(f"missing package artifact for {crate}")
    return candidates[0]


def check_crate_archive(crate_path: Path) -> list[str]:
    bad: list[str] = []
    with tarfile.open(crate_path, "r:gz") as tar:
        for member in tar.getmembers():
            name = Path(member.name).name.lower()
            if name in FORBIDDEN_NAMES:
                bad.append(member.name)
    return bad


def main() -> int:
    root = repo_root()
    print("Checking vendor bundle guard...")

    tracked_bad = check_git_tracked(root)
    if tracked_bad:
        print("ERROR: forbidden LibTSCAN files are tracked in git:")
        for path in tracked_bad:
            print(f"- {path}")
        return 1

    archive_bad: list[str] = []
    for crate in PUBLISH_CRATES:
        crate_path = ensure_package(root, crate)
        matches = check_crate_archive(crate_path)
        if matches:
            archive_bad.extend(f"{crate_path.name}: {name}" for name in matches)

    if archive_bad:
        print("ERROR: forbidden LibTSCAN files found in package artifacts:")
        for entry in archive_bad:
            print(f"- {entry}")
        return 1

    print("OK: no LibTSCAN binaries detected in tracked files or packages.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
