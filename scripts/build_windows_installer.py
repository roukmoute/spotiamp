#!/usr/bin/env python3
"""Build the per-user Windows installer from the exact validated portable ZIP.

Requires Python 3.11+ and Inno Setup 6.3+ on Windows. --validate-only also works
on Linux/macOS. No application compilation, dependency installation or downloads
are performed here; CI supplies the portable package and compiler.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import stat
import struct
import subprocess
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]
SETUP_NAME = "Spotiamp-Setup-Windows-x64.exe"
TARGET = "x86_64-pc-windows-msvc"
MAX_ARCHIVE = 64 * 1024 * 1024
MAX_UNPACKED = 128 * 1024 * 1024
MAX_FILES = 10_000
VERSION = re.compile(
    r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)"
    r"(?:-[0-9A-Za-z]+(?:[.-][0-9A-Za-z]+)*)?"
    r"(?:\+[0-9A-Za-z]+(?:[.-][0-9A-Za-z]+)*)?"
)
RESERVED = {"CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$"} | {
    f"{prefix}{digit}" for prefix in ("COM", "LPT") for digit in "123456789¹²³"
}
REQUIRED = {"spotiamp.exe", "BUILD-INFO.json", "README.md", "LICENSE",
            "THIRD-PARTY.json", "THIRD-PARTY.txt", "license-origins.json"}


def file_sha256(path):
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def safe_windows_path(name):
    """Reject traversal, Windows aliases/ADS, and files outside the ZIP contract."""
    if not name or any(c in name for c in '\\:*?"<>|\x00'):
        raise ValueError(f"invalid Windows archive path: {name!r}")
    parts = name.split("/")
    for part in parts:
        if (part in ("", ".", "..") or part.endswith((".", " "))
                or any(ord(c) < 32 for c in part)
                or part.split(".", 1)[0].upper() in RESERVED):
            raise ValueError(f"unsafe Windows archive path: {name!r}")
    allowed = (
        name in REQUIRED or name.endswith(".md") or name.startswith("LICENSE")
        if len(parts) == 1 else parts[0] in ("licenses", "mpl-source", "scripts")
    )
    if not allowed:
        raise ValueError(f"unexpected portable package file: {name!r}")
    return PurePosixPath(name)


def archive_entries(archive):
    entries = archive.infolist()
    if not 1 <= len(entries) <= MAX_FILES:
        raise ValueError("invalid number of files in portable package")
    seen = set()
    total = 0
    for entry in entries:
        name = entry.filename
        safe_windows_path(name)
        mode = stat.S_IFMT(entry.external_attr >> 16)
        if entry.is_dir() or mode not in (0, stat.S_IFREG) or entry.flag_bits & 1:
            raise ValueError(f"non-regular or encrypted package entry: {name!r}")
        folded = name.casefold()
        if folded in seen:
            raise ValueError(f"duplicate Windows package path: {name!r}")
        seen.add(folded)
        total += entry.file_size
        if entry.file_size > MAX_ARCHIVE or total > MAX_UNPACKED:
            raise ValueError("portable package exceeds the extraction size limit")
    names = {entry.filename for entry in entries}
    if not REQUIRED <= names:
        raise ValueError(f"portable package is missing: {sorted(REQUIRED - names)}")
    for directory in ("licenses/", "mpl-source/"):
        if not any(name.startswith(directory) for name in names):
            raise ValueError(f"portable package is missing {directory}")
    return entries, total


def validate_metadata(archive, expected_commit=None, expected_version=None):
    if archive.getinfo("BUILD-INFO.json").file_size > 64 * 1024:
        raise ValueError("BUILD-INFO.json exceeds its size limit")
    metadata = json.loads(archive.read("BUILD-INFO.json"))
    if not isinstance(metadata, dict):
        raise ValueError("BUILD-INFO.json must be an object")
    if metadata.get("target") != TARGET or metadata.get("binary_name") != "spotiamp.exe":
        raise ValueError("installer requires the x86_64-pc-windows-msvc portable package")
    version = metadata.get("app_version", "")
    match = VERSION.fullmatch(version) if isinstance(version, str) else None
    if not match or any(int(number) > 65535 for number in match.groups()):
        raise ValueError("invalid application version in BUILD-INFO.json")
    for field, length in (("git_commit", 40), ("binary_sha256", 64)):
        value = metadata.get(field)
        if not isinstance(value, str) or not re.fullmatch(rf"[0-9a-fA-F]{{{length}}}", value):
            raise ValueError(f"invalid {field} in BUILD-INFO.json")
    if expected_commit and metadata["git_commit"].lower() != expected_commit.lower():
        raise ValueError("package commit does not match --expected-commit")
    if expected_version and version != expected_version:
        raise ValueError("package version does not match --expected-version")
    size = metadata.get("binary_bytes")
    if type(size) is not int or not 1 <= size <= MAX_ARCHIVE:
        raise ValueError("invalid binary_bytes in BUILD-INFO.json")
    metadata["numeric_version"] = ".".join(match.groups()) + ".0"
    return metadata


def validate_pe(archive):
    with archive.open("spotiamp.exe") as binary:
        header = binary.read(64)
        if len(header) != 64 or header[:2] != b"MZ":
            raise ValueError("spotiamp.exe is not a Windows executable")
        offset = struct.unpack_from("<I", header, 0x3C)[0]
        if offset < 64 or offset > 1024 * 1024:
            raise ValueError("invalid PE header offset")
        binary.seek(offset)
        if binary.read(6) != b"PE\0\0\x64\x86":
            raise ValueError("spotiamp.exe is not an x64 PE executable")


def prepare_payload(path, destination=None, expected_commit=None, expected_version=None):
    """Validate every CRC/size and executable hash, optionally extracting verbatim."""
    if not path.is_file() or not 1 <= path.stat().st_size <= MAX_ARCHIVE:
        raise ValueError("portable archive is missing, empty, or larger than 64 MiB")
    with zipfile.ZipFile(path) as archive:
        entries, total = archive_entries(archive)
        metadata = validate_metadata(archive, expected_commit, expected_version)
        validate_pe(archive)
        for entry in entries:
            target = None
            digest = hashlib.sha256()
            count = 0
            try:
                if destination is not None:
                    output = destination.joinpath(*PurePosixPath(entry.filename).parts)
                    output.parent.mkdir(parents=True, exist_ok=True)
                    target = output.open("xb")
                with archive.open(entry) as source:
                    while chunk := source.read(64 * 1024):
                        count += len(chunk)
                        if count > entry.file_size:
                            raise ValueError(f"incorrect package file size: {entry.filename}")
                        digest.update(chunk)
                        if target is not None:
                            target.write(chunk)
            finally:
                if target is not None:
                    target.close()
            if count != entry.file_size:
                raise ValueError(f"truncated package file: {entry.filename}")
            if entry.filename == "spotiamp.exe" and (
                count != metadata["binary_bytes"]
                or digest.hexdigest() != metadata["binary_sha256"].lower()
            ):
                raise ValueError("executable size or SHA256 differs from BUILD-INFO.json")
    return {
        "app_version": metadata["app_version"],
        "numeric_version": metadata["numeric_version"],
        "git_commit": metadata["git_commit"],
        "target": TARGET,
        "binary_name": "spotiamp.exe",
        "binary_sha256": metadata["binary_sha256"].lower(),
        "portable_archive": path.name,
        "portable_sha256": file_sha256(path),
        "payload_files": len(entries),
        "payload_bytes": total,
    }


def find_compiler(explicit):
    if explicit:
        candidate = shutil.which(explicit) or explicit
        if Path(candidate).is_file():
            return str(Path(candidate).resolve())
        raise ValueError(f"Inno Setup compiler not found: {explicit}")
    candidate = shutil.which("ISCC.exe") or shutil.which("iscc")
    if candidate:
        return candidate
    for variable in ("ProgramFiles(x86)", "ProgramFiles"):
        if base := os.environ.get(variable):
            candidate = Path(base) / "Inno Setup 6" / "ISCC.exe"
            if candidate.is_file():
                return str(candidate)
    raise ValueError("Inno Setup 6.3+ is required; pass --iscc or add ISCC.exe to PATH")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", type=Path, required=True, help="validated Windows MSVC portable ZIP")
    parser.add_argument("--output-dir", type=Path, default=ROOT / "dist")
    parser.add_argument("--iscc", help="path to the Inno Setup 6 command-line compiler")
    parser.add_argument("--expected-commit", help="require this BUILD-INFO Git commit (CI: GITHUB_SHA)")
    parser.add_argument("--expected-version", help="require this application version")
    parser.add_argument("--validate-only", action="store_true", help="check the ZIP without building an installer")
    args = parser.parse_args()
    try:
        if args.validate_only:
            report = prepare_payload(args.archive, expected_commit=args.expected_commit,
                                     expected_version=args.expected_version)
        else:
            compiler = find_compiler(args.iscc)
            icon = ROOT / "assets" / "windows" / "spotiamp.ico"
            if not icon.is_file():
                raise ValueError(f"installer icon is missing: {icon}")
            output = args.output_dir.resolve()
            output.mkdir(parents=True, exist_ok=True)
            with tempfile.TemporaryDirectory(prefix="spotiamp-installer-") as temporary:
                payload = Path(temporary) / "payload"
                report = prepare_payload(args.archive, payload, args.expected_commit, args.expected_version)
                subprocess.run([
                    compiler,
                    f"/DPayloadDir={payload}",
                    f"/DAppVersion={report['app_version']}",
                    f"/DNumericVersion={report['numeric_version']}",
                    f"/DInstallerOutputDir={output}",
                    f"/DInstallerIcon={icon}",
                    str(ROOT / "scripts" / "windows" / "installer.iss"),
                ], check=True)
            setup = output / SETUP_NAME
            if not setup.is_file() or setup.stat().st_size == 0:
                raise ValueError("Inno Setup did not produce the expected installer")
            report.update(installer=SETUP_NAME, installer_bytes=setup.stat().st_size,
                          installer_sha256=file_sha256(setup))
            (output / "Spotiamp-Setup-Windows-x64.build.json").write_text(
                json.dumps(report, indent=2) + "\n", encoding="utf-8",
            )
        print(json.dumps(report, indent=2))
    except (OSError, ValueError, zipfile.BadZipFile, RuntimeError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"Installer build failed: {error}\n")


if __name__ == "__main__":
    main()
