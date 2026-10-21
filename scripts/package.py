#!/usr/bin/env python3
"""Package an already-built executable with resolved dependency notices and MPL source."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import stat
import subprocess
import zipfile

ROOT = Path(__file__).resolve().parents[1]
LICENSE_NAMES = ("license", "licence", "copying", "notice", "copyright")


def command(*args):
    return subprocess.check_output(args, cwd=ROOT, encoding="utf-8")


def reachable_packages(metadata):
    nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
    pending = [metadata["resolve"]["root"]]
    reachable = set()
    while pending:
        package_id = pending.pop()
        if package_id in reachable:
            continue
        reachable.add(package_id)
        pending.extend(nodes[package_id]["dependencies"])
    return sorted(
        (
            p for p in metadata["packages"]
            if p["id"] in reachable and p["id"] != metadata["resolve"]["root"]
        ),
        key=lambda package: (package["name"], package["version"]),
    )


def put(archive, name, data, executable=False):
    info = zipfile.ZipInfo(name)
    info.compress_type = zipfile.ZIP_DEFLATED
    info.create_system = 3
    info.external_attr = (stat.S_IFREG | (0o755 if executable else 0o644)) << 16
    archive.writestr(info, data)


def put_file(archive, name, path, executable=False):
    put(archive, name, path.read_bytes(), executable)


def project_files():
    for path in sorted(ROOT.iterdir()):
        if path.is_file() and (
            path.suffix in (".md", ".toml")
            or path.name in ("Cargo.lock", "LICENSE", "LICENSE-MIT", "LICENSE-APACHE", ".gitignore", "preview.png")
        ):
            yield path
    for directory in ("src", "scripts", ".github", "assets", "vendor", "benchmarks"):
        for path in sorted((ROOT / directory).rglob("*")):
            if path.is_file() and "__pycache__" not in path.parts:
                yield path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", required=True)
    parser.add_argument("--binary", type=Path, help="override target/<target>/release/spotiamp[.exe]")
    parser.add_argument("--output-dir", type=Path, default=ROOT / "dist")
    args = parser.parse_args()
    binary_name = "spotiamp.exe" if "windows" in args.target else "spotiamp"
    binary = args.binary or ROOT / "target" / args.target / "release" / binary_name
    if not binary.is_file():
        parser.error(f"release executable does not exist: {binary}")
    for name in ("README.md", "LICENSE", "Cargo.lock"):
        if not (ROOT / name).is_file():
            parser.error(f"required distribution file is missing: {name}")
    metadata = json.loads(command(
        "cargo", "metadata", "--locked", "--offline", "--format-version", "1",
        "--filter-platform", args.target,
    ))
    packages = reachable_packages(metadata)
    notices = []
    license_entries = []
    mpl_entries = []
    missing = []
    overrides = ROOT / "scripts" / "license-overrides"
    for package in packages:
        source = Path(package["manifest_path"]).parent
        slug = f"{package['name']}-{package['version']}"
        files = sorted(
            path for path in source.rglob("*")
            if path.is_file() and path.name.lower().startswith(LICENSE_NAMES)
        )
        file_pairs = [(path, path.relative_to(source)) for path in files]
        if package.get("license_file"):
            explicit = source / package["license_file"]
            if explicit.is_file() and explicit not in files:
                file_pairs.append((explicit, Path(explicit.name)))
        if not file_pairs:
            file_pairs = [
                (path, path.relative_to(overrides / slug))
                for path in sorted((overrides / slug).rglob("*")) if path.is_file()
            ]
        if not file_pairs:
            missing.append(slug)
        license_paths = []
        for path, relative in file_pairs:
            archive_path = f"licenses/{slug}/{relative.as_posix()}"
            license_entries.append((archive_path, path))
            license_paths.append(archive_path)
        uses_mpl = "MPL-2.0" in (package.get("license") or "")
        if uses_mpl:
            for path in sorted(source.rglob("*")):
                if path.is_file() and path.name != ".cargo-ok":
                    mpl_entries.append((f"mpl-source/{slug}/{path.relative_to(source).as_posix()}", path))
            for path, relative in file_pairs:
                if not (source / relative).is_file():
                    mpl_entries.append((f"mpl-source/{slug}/{relative.as_posix()}", path))
        notices.append({
            "name": package["name"],
            "version": package["version"],
            "authors": package["authors"],
            "license_expression": package["license"],
            "selected_license": (
                "MPL-2.0" if uses_mpl else "Apache-2.0"
                if package["name"] == "linux-keyutils" else None
            ),
            "repository": package.get("repository"),
            "source": (
                f"spotiamp-source.zip/spotiamp/{source.relative_to(ROOT).as_posix()}"
                if source.is_relative_to(ROOT)
                else f"https://crates.io/api/v1/crates/{package['name']}/{package['version']}/download"
            ),
            "license_files": license_paths,
            "bundled_source": f"mpl-source/{slug}" if uses_mpl else None,
        })
    if missing:
        parser.error("license text missing; add a reviewed scripts/license-overrides/<crate-version>/ entry: " + ", ".join(missing))

    args.output_dir.mkdir(parents=True, exist_ok=True)
    build_info = {
        "target": args.target,
        "build_host": platform.platform(),
        "build_host_libc": platform.libc_ver(),
        "rustc": command("rustc", "--version", "--verbose").strip(),
        "cargo": command("cargo", "--version").strip(),
        "cargo_lock_sha256": hashlib.sha256((ROOT / "Cargo.lock").read_bytes()).hexdigest(),
        "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "binary_bytes": binary.stat().st_size,
        "rustflags": os.environ.get("RUSTFLAGS", ""),
        "windows_baseline": (
            "Windows 10/11 x64 (Rust baseline: Windows 10+ / Windows Server 2016+)"
            if args.target.startswith("x86_64-pc-windows") else None
        ),
        "windows_baseline_source": (
            "https://doc.rust-lang.org/rustc/platform-support.html"
            if "windows" in args.target else None
        ),
        "native_interfaces": (
            {"window": "minifb / Win32 GDI", "audio": "Rodio / CPAL / WASAPI", "credentials": "Windows Credential Manager"}
            if "windows" in args.target else None
        ),
        "windows_code_signed": False if "windows" in args.target else None,
        "linux_portability_note": (
            "Built against the build host's system libraries. Compatibility with older distributions has not been verified."
            if "linux" in args.target else None
        ),
    }
    rust_docs = Path(command("rustc", "--print", "sysroot").strip()) / "share" / "doc" / "rust"
    rust_notices = []
    library_notice = rust_docs / "COPYRIGHT-library.html"
    if not library_notice.is_file():
        library_notice = rust_docs / "COPYRIGHT.html"
    if library_notice.is_file():
        rust_notices.append((f"licenses/rust-runtime/{library_notice.name}", library_notice))
    for path in sorted((rust_docs / "licenses").glob("*")):
        if path.is_file():
            rust_notices.append((f"licenses/rust-runtime/{path.name}", path))
    if args.target.endswith("windows-gnu"):
        for path in sorted((ROOT / "scripts" / "runtime-licenses" / "windows-gnu").glob("*")):
            if path.is_file():
                rust_notices.append((f"licenses/windows-gnu-runtime/{path.name}", path))
    third_party = (
        "This distribution includes third-party Rust libraries. See THIRD-PARTY.json for\n"
        "exact versions, license expressions, source locations, and license text paths.\n"
        "The inventory includes reachable build dependencies as well as runtime dependencies.\n\n"
        "Rust runtime notices supplied by the build toolchain are under licenses/rust-runtime/.\n\n"
        "The complete sources used for MPL-2.0 components are included\n"
        "under mpl-source/. Those sources remain available under MPL-2.0. For the\n"
        "dual-licensed priority-queue library, this distribution uses the MPL-2.0 option.\n"
        "See https://www.mozilla.org/en-US/MPL/2.0/ and each included license text.\n\n"
        "Spotiamp source, Cargo.lock, and build instructions are in spotiamp-source.zip\n"
        "(also distributed as Spotiamp-source.zip),\n"
        "provided with this distribution. Cargo.lock pins registry source checksums.\n"
        "License texts absent from published crates were copied from upstream\n"
        "repositories or standard license texts linked by their licensing notice;\n"
        "exact source URLs are recorded in license-origins.json.\n"
    )
    zip_path = args.output_dir / f"spotiamp-{args.target}.zip"
    with zipfile.ZipFile(zip_path, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
        put_file(archive, binary_name, binary, executable=True)
        for path in sorted(ROOT.iterdir()):
            if path.is_file() and (path.suffix == ".md" or path.name.startswith("LICENSE")):
                put_file(archive, path.name, path)
        for name in ("measure-windows.ps1", "measure-linux.py", "README.md"):
            path = ROOT / "scripts" / name
            if path.is_file():
                put_file(archive, f"scripts/{name}", path)
        put(archive, "BUILD-INFO.json", json.dumps(build_info, indent=2) + "\n")
        put(archive, "THIRD-PARTY.json", json.dumps(notices, indent=2) + "\n")
        put(archive, "THIRD-PARTY.txt", third_party)
        put_file(archive, "license-origins.json", overrides / "origins.json")
        for name, path in license_entries + mpl_entries + rust_notices:
            put_file(archive, name, path)
    source_path = args.output_dir / "spotiamp-source.zip"
    with zipfile.ZipFile(source_path, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
        for path in project_files():
            put_file(archive, f"spotiamp/{path.relative_to(ROOT).as_posix()}", path)
    print(f"Packaged {zip_path} ({zip_path.stat().st_size / 1024 / 1024:.2f} MiB)")
    print(f"Packaged {source_path} ({source_path.stat().st_size / 1024 / 1024:.2f} MiB)")
    print(f"Included notices for {len(notices)} crates and source for {sum(n['bundled_source'] is not None for n in notices)} MPL crates.")


if __name__ == "__main__":
    main()
