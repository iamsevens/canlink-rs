#!/usr/bin/env python3
"""Check public rustdoc coverage for publishable CANLink-RS crates."""

from __future__ import annotations

import os
import re
import subprocess
import sys
from dataclasses import dataclass


TOTAL_RE = re.compile(r"^\|\s*Total\s*\|\s*(\d+)\s*\|\s*([0-9.]+)%\s*\|")


@dataclass(frozen=True)
class CoverageTarget:
    name: str
    command: list[str]


TARGETS = [
    CoverageTarget(
        name="canlink-hal",
        command=[
            "cargo",
            "+nightly",
            "rustdoc",
            "-p",
            "canlink-hal",
            "--lib",
            "--all-features",
            "--",
            "-Z",
            "unstable-options",
            "--show-coverage",
        ],
    ),
    CoverageTarget(
        name="canlink-tscan-sys",
        command=[
            "cargo",
            "+nightly",
            "rustdoc",
            "-p",
            "canlink-tscan-sys",
            "--lib",
            "--",
            "-Z",
            "unstable-options",
            "--show-coverage",
        ],
    ),
    CoverageTarget(
        name="canlink-mock",
        command=[
            "cargo",
            "+nightly",
            "rustdoc",
            "-p",
            "canlink-mock",
            "--lib",
            "--all-features",
            "--",
            "-Z",
            "unstable-options",
            "--show-coverage",
        ],
    ),
    CoverageTarget(
        name="canlink-tscan",
        command=[
            "cargo",
            "+nightly",
            "rustdoc",
            "-p",
            "canlink-tscan",
            "--lib",
            "--",
            "-Z",
            "unstable-options",
            "--show-coverage",
        ],
    ),
    CoverageTarget(
        name="canlink-cli",
        command=[
            "cargo",
            "+nightly",
            "rustdoc",
            "-p",
            "canlink-cli",
            "--bin",
            "canlink",
            "--",
            "-Z",
            "unstable-options",
            "--show-coverage",
        ],
    ),
]


def extract_total_coverage(output: str) -> tuple[int, float]:
    for line in output.splitlines():
        match = TOTAL_RE.match(line.strip())
        if match:
            documented = int(match.group(1))
            percentage = float(match.group(2))
            return documented, percentage
    raise ValueError("failed to parse rustdoc coverage total row")


def main() -> int:
    env = os.environ.copy()
    env.setdefault("PYTHONUTF8", "1")
    env.setdefault("CANLINK_TSCAN_ALLOW_MISSING_BUNDLE", "1")

    print("Checking rustdoc coverage for publishable crates...")
    failures: list[str] = []

    for target in TARGETS:
        print(f"\n==> {target.name}")
        result = subprocess.run(
            target.command,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            encoding="utf-8",
            errors="replace",
            env=env,
            check=False,
        )
        print(result.stdout, end="" if result.stdout.endswith("\n") else "\n")

        if result.returncode != 0:
            failures.append(f"{target.name}: command failed with exit code {result.returncode}")
            continue

        try:
            documented, percentage = extract_total_coverage(result.stdout)
        except ValueError as exc:
            failures.append(f"{target.name}: {exc}")
            continue

        if documented == 0:
            failures.append(
                f"{target.name}: total documented item count is 0, unable to assert public docs coverage"
            )
            continue

        if percentage < 100.0:
            failures.append(f"{target.name}: documented coverage is {percentage:.1f}%")
            continue

        print(f"[OK] {target.name}: documented coverage {percentage:.1f}% ({documented} items)")

    if failures:
        print("\nDocumentation coverage check failed:")
        for failure in failures:
            print(f"- {failure}")
        return 1

    print("\nAll publishable crates have 100.0% documented public API coverage.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
