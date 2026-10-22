#!/usr/bin/env python3
"""Build an explicit isolated source tree and preserve its exact decoder deps.
Run this beside the measurement scripts. Never points at personal configuration,
never launches the GUI, and never changes the supplied source tree.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import time
import tomllib

ROOT = Path(__file__).resolve().parent
PROFILES = {'baseline-thin3': ('3', 'thin'), 'fat3': ('3', 'fat'),
            'pruned-fat3': ('3', 'fat'), 'fats': ('s', 'fat'),
            'fatz': ('z', 'fat'), 'mixeds': ('s', 'fat')}
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--source', type=Path, required=True)
parser.add_argument('--source-commit', required=True, help='Commit used to create the source archive')
parser.add_argument('--profile', choices=PROFILES, required=True)
parser.add_argument('--jobs', type=int, default=3)
parser.add_argument('--offline', action='store_true')
args = parser.parse_args()
source = args.source.resolve(strict=True)
manifest = tomllib.loads((source / 'Cargo.toml').read_text())
output = ROOT / 'builds' / args.profile
output.mkdir(parents=True, exist_ok=False)
level, lto = PROFILES[args.profile]
profile = {'opt-level': level, 'lto': lto, 'codegen-units': '1', 'strip': 'symbols', 'panic': 'abort'}
environment = {'CARGO_TARGET_DIR': str(ROOT / 'target'), **{
    'CARGO_PROFILE_RELEASE_' + key.upper().replace('-', '_'): value for key, value in profile.items()}}
env = {**os.environ, **environment}
assert not env.get('RUSTFLAGS') and not env.get('CARGO_ENCODED_RUSTFLAGS'), 'Unexpected compiler flags'
command = ['cargo', 'build', '--locked', '--release', '--bin', 'spotiamp',
           '-j', str(args.jobs), '--message-format=json']
if args.offline:
    command.append('--offline')
started = time.perf_counter()
with (output / 'artifacts.jsonl').open('w') as stdout, (output / 'cargo.log').open('w') as stderr:
    completed = subprocess.run(command, cwd=source, env=env, stdout=stdout, stderr=stderr)
metadata = {'profile_name': args.profile, 'profile': profile, 'command': command,
            'source': str(source), 'source_commit': args.source_commit,
            'version': manifest['package']['version'], 'environment_overrides': environment,
            'package_overrides': manifest.get('profile', {}).get('release', {}).get('package', {}),
            'exit_code': completed.returncode, 'build_elapsed_seconds': time.perf_counter() - started,
            'compiler': subprocess.check_output(['rustc', '-Vv'], text=True).strip()}
if completed.returncode == 0:
    binary = output / ('spotiamp-' + args.profile)
    shutil.copy2(ROOT / 'target/release/spotiamp', binary)
    metadata.update({'binary': str(binary), 'binary_bytes': binary.stat().st_size,
                     'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest()})
    directory = output / 'deps'
    directory.mkdir()
    for file in (ROOT / 'target/release/deps').iterdir():
        if file.is_file():
            os.link(file, directory / file.name)
    artifacts = []
    for line in (output / 'artifacts.jsonl').read_text().splitlines():
        event = json.loads(line)
        if event.get('reason') == 'compiler-artifact':
            artifacts.append({'package_id': event['package_id'], 'target': event['target']['name'],
                'profile': event['profile'], 'features': event['features'], 'filenames': event['filenames']})
    (output / 'dependency-artifacts.json').write_text(json.dumps(artifacts, indent=2) + '\n')
(output / 'build.json').write_text(json.dumps(metadata, indent=2) + '\n')
print(json.dumps(metadata, indent=2))
raise SystemExit(completed.returncode)
