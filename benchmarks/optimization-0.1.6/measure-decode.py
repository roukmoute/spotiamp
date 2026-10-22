#!/usr/bin/env python3
"""Measure original synthetic files through the production librespot decoder.
No network, Spotify authentication, encryption, streaming, audio output or UI.
CPU is child user+system time; files are warmed first, profiles interleaved.
"""
import argparse
import array
import hashlib
import json
import math
from pathlib import Path
import random
import resource
import statistics
import subprocess
import time

ROOT = Path(__file__).resolve().parent


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--profile', action='append', required=True)
    parser.add_argument('--passes', type=int, default=100)
    parser.add_argument('--runs', type=int, default=3)
    parser.add_argument('--verify-only', action='store_true')
    parser.add_argument('--format', action='append', choices=['ogg', 'mp3', 'flac'])
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    assert args.passes > 0 and args.runs > 0
    formats = args.format or ['ogg', 'mp3', 'flac']
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    manifest = json.loads((ROOT / 'fixtures/manifest.json').read_text())
    samples_per_pass = manifest['frames'] * manifest['channels']
    report = {'scope': __doc__, 'passes': args.passes, 'runs': args.runs,
              'fixtures': manifest, 'profiles': {}, 'results': [], 'success': False}
    for name in args.profile:
        directory = ROOT / 'builds' / name
        build = json.loads((directory / 'decode-probe-build.json').read_text())
        assert build['exit_code'] == 0
        binary = directory / 'decode-probe'
        assert hashlib.sha256(binary.read_bytes()).hexdigest() == build['sha256']
        artifacts = json.loads((directory / 'dependency-artifacts.json').read_text())
        features = next(item['features'] for item in artifacts if item['target'] == 'symphonia')
        report['profiles'][name] = {'binary_sha256': build['sha256'], 'features': features,
                                   'build': build}
    pairs = [(name, extension) for name in args.profile for extension in formats
             if extension != 'flac' or 'flac' in report['profiles'][name]['features']]

    def invoke(name, extension, passes):
        fixture = ROOT / 'fixtures' / ('synthetic-stereo.' + extension)
        binary = ROOT / 'builds' / name / 'decode-probe'
        before = resource.getrusage(resource.RUSAGE_CHILDREN)
        started = time.perf_counter()
        completed = subprocess.run([str(binary), str(fixture), str(passes), str(samples_per_pass)],
                                   capture_output=True, text=True, timeout=180)
        wall = time.perf_counter() - started
        after = resource.getrusage(resource.RUSAGE_CHILDREN)
        if completed.returncode:
            raise RuntimeError(f'{name}/{extension}: {completed.stderr}')
        payload = json.loads(completed.stdout)
        assert payload['samples'] == samples_per_pass * passes
        return {**payload, 'profile': name, 'process_wall_seconds': wall,
                'cpu_seconds': (after.ru_utime + after.ru_stime) - (before.ru_utime + before.ru_stime),
                'audio_seconds_decoded': manifest['duration_seconds'] * passes}

    references, verification = {}, []
    for name, extension in pairs:
        pcm = out / f'{name}-{extension}.f64le'
        binary = ROOT / 'builds' / name / 'decode-probe'
        fixture = ROOT / 'fixtures' / ('synthetic-stereo.' + extension)
        completed = subprocess.run([str(binary), str(fixture), '1', str(samples_per_pass),
            '--pcm-output', str(pcm)], capture_output=True, text=True, check=True, timeout=30)
        payload = json.loads(completed.stdout)
        assert pcm.stat().st_size == samples_per_pass * 8
        digest = hashlib.sha256(pcm.read_bytes()).hexdigest()
        item = {'profile': name, 'format': extension, 'samples': payload['samples'],
                'pcm_sha256': digest, 'bytes': pcm.stat().st_size}
        if extension not in references:
            references[extension] = (name, pcm, digest)
        reference, reference_path, reference_sha = references[extension]
        item.update({'reference_profile': reference, 'identical_pcm_bits': digest == reference_sha})
        if digest != reference_sha:
            a, b = array.array('d'), array.array('d')
            a.frombytes(reference_path.read_bytes()); b.frombytes(pcm.read_bytes())
            assert len(a) == len(b)
            differences = [abs(x - y) for x, y in zip(a, b)]
            item.update({'max_absolute_difference': max(differences),
                         'rms_difference': math.sqrt(sum(x*x for x in differences) / len(differences))})
        verification.append(item)
        print(f'PCM {name}/{extension}: {digest} identical={item["identical_pcm_bits"]}', flush=True)
    report['pcm_verification'] = verification
    if args.verify_only:
        report['success'] = True
        (out / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
        return
    for name, extension in pairs:
        invoke(name, extension, 1)
    randomizer = random.Random(20261022)
    try:
        for iteration in range(args.runs):
            randomizer.shuffle(pairs)
            for name, extension in pairs:
                row = invoke(name, extension, args.passes)
                row['iteration'] = iteration
                row['cpu_ms_per_audio_second'] = row['cpu_seconds'] * 1000 / row['audio_seconds_decoded']
                report['results'].append(row)
                print(f'{name}/{extension} run{iteration + 1}: CPU {row["cpu_seconds"]:.4f}s, '
                      f'{row["cpu_ms_per_audio_second"]:.4f}ms/audio-second', flush=True)
                (out / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
        report['summary'] = {name: {extension: {
            'median_cpu_ms_per_audio_second': statistics.median(row['cpu_ms_per_audio_second']
                for row in report['results'] if row['profile'] == name and row['format'] == extension)
            } for extension in formats if any(row['profile'] == name and row['format'] == extension
                 for row in report['results'])} for name in args.profile}
        report['success'] = True
    finally:
        (out / 'report.json').write_text(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
