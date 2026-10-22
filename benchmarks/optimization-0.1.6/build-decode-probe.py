#!/usr/bin/env python3
"""Link the offline probe against a preserved release-profile dependency set."""
from pathlib import Path
import hashlib
import json
import subprocess
import sys
import time

root = Path(__file__).resolve().parent
build = root / 'builds' / sys.argv[1]
metadata = json.loads((build / 'build.json').read_text())
artifacts = json.loads((build / 'dependency-artifacts.json').read_text())
profile = metadata['profile']
output = build / 'decode-probe'
command = ['rustc', '--edition=2024', str(root / 'decode-probe.rs'), '-o', str(output),
           '-L', 'dependency=' + str(build / 'deps')]
for key in ['opt-level', 'lto', 'codegen-units', 'panic', 'strip']:
    command += ['-C', key + '=' + str(profile[key])]
for crate in ['librespot_playback', 'symphonia']:
    matches = [item for item in artifacts if item['target'] == crate]
    assert len(matches) == 1, (crate, len(matches))
    paths = [build / 'deps' / Path(path).name for path in matches[0]['filenames'] if path.endswith('.rlib')]
    assert len(paths) == 1 and paths[0].exists(), paths
    command += ['--extern', crate + '=' + str(paths[0])]
started = time.perf_counter()
result = subprocess.run(command, capture_output=True, text=True)
(build / 'decode-probe-build.log').write_text(result.stdout + result.stderr)
report = {'command': command, 'exit_code': result.returncode,
          'elapsed_seconds': time.perf_counter() - started,
          'source_sha256': hashlib.sha256((root / 'decode-probe.rs').read_bytes()).hexdigest()}
if result.returncode == 0:
    report.update({'binary': str(output), 'bytes': output.stat().st_size,
                   'sha256': hashlib.sha256(output.read_bytes()).hexdigest()})
(build / 'decode-probe-build.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(report, indent=2))
if result.returncode:
    print(result.stderr)
raise SystemExit(result.returncode)
