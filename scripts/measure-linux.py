#!/usr/bin/env python3
"""Sample one Linux process. A scenario label does not verify that audio is playing."""

import argparse
import datetime as dt
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import statistics
import subprocess
import time


def positive(value):
    number = float(value)
    if not math.isfinite(number) or number <= 0:
        raise argparse.ArgumentTypeError("must be a finite number greater than zero")
    return number


def fields(path):
    result = {}
    for line in path.read_text().splitlines():
        key, _, value = line.partition(":")
        pieces = value.split()
        if pieces and pieces[0].isdigit():
            result[key] = int(pieces[0]) * (1024 if pieces[-1] == "kB" else 1)
    return result


def sample(pid, started):
    proc = Path("/proc") / str(pid)
    status = fields(proc / "status")
    if "VmRSS" not in status:
        return None  # Includes zombie processes.
    rollup = {}
    try:
        rollup = fields(proc / "smaps_rollup")
    except PermissionError:
        pass
    stat = (proc / "stat").read_text().rsplit(")", 1)[1].split()
    return {
        "elapsed_seconds": round(time.monotonic() - started, 4),
        "rss_bytes": status["VmRSS"],
        "peak_rss_bytes": status.get("VmHWM"),
        "virtual_bytes": status.get("VmSize"),
        "pss_bytes": rollup.get("Pss"),
        "private_resident_bytes": (
            rollup.get("Private_Clean", 0) + rollup.get("Private_Dirty", 0)
            if rollup else None
        ),
        "cpu_seconds": (int(stat[11]) + int(stat[12])) / os.sysconf("SC_CLK_TCK"),
        "threads": status.get("Threads"),
    }


def distribution(values):
    values = sorted(value for value in values if value is not None)
    if not values:
        return None
    return {
        "min_bytes": values[0],
        "median_bytes": statistics.median(values),
        "p95_bytes": values[max(0, math.ceil(len(values) * 0.95) - 1)],
        "max_bytes": values[-1],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    target = parser.add_mutually_exclusive_group(required=True)
    target.add_argument("--pid", type=int, help="attach to an existing process")
    target.add_argument("--exe", type=Path, help="launch this executable; leave it open afterwards")
    parser.add_argument("--argument", action="append", default=[], help="argument for --exe (repeatable)")
    parser.add_argument("--scenario", choices=("idle", "playing"), required=True)
    parser.add_argument("--seconds", type=positive, default=60)
    parser.add_argument("--interval", type=positive, default=1)
    parser.add_argument("--warmup", type=float, default=5, help="seconds before sampling")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    if not platform.system() == "Linux":
        parser.error("this sampler requires Linux /proc")
    if not math.isfinite(args.warmup) or args.warmup < 0:
        parser.error("--warmup must be finite and nonnegative")
    if args.pid is not None and args.pid <= 0:
        parser.error("--pid must be positive")
    if args.argument and not args.exe:
        parser.error("--argument requires --exe")

    started = time.monotonic()
    process = subprocess.Popen([str(args.exe.resolve()), *args.argument]) if args.exe else None
    pid = process.pid if process else args.pid
    proc = Path("/proc") / str(pid)
    try:
        executable = (proc / "exe").resolve(strict=True)
        digest = hashlib.sha256(executable.read_bytes()).hexdigest()
    except (OSError, PermissionError):
        executable, digest = None, None
    report = {
        "schema_version": 1,
        "platform": platform.platform(),
        "recorded_at_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
        "pid": pid,
        "executable": str(executable) if executable else None,
        "executable_sha256": digest,
        "scenario": args.scenario,
        "scenario_verified": False,
        "measurement_scope": "single process; children excluded",
        "metric_note": "Linux private_resident_bytes is resident private memory, not Windows Private Bytes (committed memory).",
        "startup_to_visible_ms": None,
        "startup_note": "Window visibility is not measured on Linux.",
        "requested_seconds": args.seconds,
        "interval_seconds": args.interval,
        "warmup_seconds": args.warmup,
        "samples": [],
        "completion": "duration_elapsed",
    }
    output = args.output or Path(f"memory-linux-{args.scenario}.json")
    try:
        time.sleep(args.warmup)
        deadline = time.monotonic() + args.seconds
        while time.monotonic() < deadline:
            try:
                row = sample(pid, started)
            except (FileNotFoundError, ProcessLookupError):
                row = None
            if row is None:
                report["completion"] = "process_exited"
                break
            report["samples"].append(row)
            time.sleep(max(0, min(args.interval, deadline - time.monotonic())))
    except KeyboardInterrupt:
        report["completion"] = "interrupted"
    finally:
        report["elapsed_seconds"] = round(time.monotonic() - started, 4)
        report["sample_count"] = len(report["samples"])
        report["summary"] = {
            metric: distribution(row[metric] for row in report["samples"])
            for metric in ("rss_bytes", "pss_bytes", "private_resident_bytes")
        }
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
        print(f"Saved {report['sample_count']} samples to {output.resolve()}")
    return 0 if report["samples"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
