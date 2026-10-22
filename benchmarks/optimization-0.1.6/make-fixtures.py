#!/usr/bin/env python3
"""Generate original synthetic stereo fixtures through installed libsndfile.
ABI/constants verified against upstream libsndfile 1.2.2 include/sndfile.h.
No microphone, audio playback or external media is used.
"""
import ctypes as C
import hashlib
import json
import math
from pathlib import Path

OUT = Path(__file__).resolve().parent / 'fixtures'
OUT.mkdir(exist_ok=True)
RATE, SECONDS = 44100, 30

class Info(C.Structure):
    _fields_ = [('frames', C.c_int64), ('samplerate', C.c_int), ('channels', C.c_int),
               ('format', C.c_int), ('sections', C.c_int), ('seekable', C.c_int)]

lib = C.CDLL('libsndfile.so.1')
for name, restype, argtypes in [
    ('sf_open', C.c_void_p, [C.c_char_p, C.c_int, C.POINTER(Info)]),
    ('sf_close', C.c_int, [C.c_void_p]),
    ('sf_writef_float', C.c_int64, [C.c_void_p, C.POINTER(C.c_float), C.c_int64]),
    ('sf_format_check', C.c_int, [C.POINTER(Info)]),
    ('sf_strerror', C.c_char_p, [C.c_void_p]),
    ('sf_version_string', C.c_char_p, []),
]:
    fn = getattr(lib, name)
    fn.restype, fn.argtypes = restype, argtypes

report = {'encoder': lib.sf_version_string().decode(), 'sample_rate': RATE,
          'channels': 2, 'duration_seconds': SECONDS, 'frames': RATE * SECONDS,
          'signal': 'Original deterministic stereo chirps, harmonic tones and LCG noise; non-silent; no media input',
          'files': []}
formats = [('ogg', 0x200000 | 0x0060), ('flac', 0x170000 | 0x0002),
           ('mp3', 0x230000 | 0x0082)]
for extension, format_code in formats:
    info = Info(0, RATE, 2, format_code, 0, 0)
    if not lib.sf_format_check(C.byref(info)):
        report['files'].append({'format': extension, 'supported': False})
        continue
    path = OUT / ('synthetic-stereo.' + extension)
    output = lib.sf_open(str(path).encode(), 0x20, C.byref(info))
    if not output:
        raise RuntimeError(lib.sf_strerror(None).decode())
    state = 0x5A0F2026
    try:
        samples = (C.c_float * (RATE * 2))()
        for second in range(SECONDS):
            for index in range(RATE):
                t = second + index / RATE
                state = (1664525 * state + 1013904223) & 0xffffffff
                noise = (state / 0xffffffff * 2 - 1) * .018
                envelope = .65 + .35 * math.sin(t * math.tau * .73)
                samples[index * 2] = envelope * (.21 * math.sin(math.tau * (220*t + 8*t*t))
                    + .11 * math.sin(math.tau * 660*t)) + noise
                samples[index * 2 + 1] = envelope * (.19 * math.sin(math.tau * (337*t + 5*t*t))
                    + .09 * math.sin(math.tau * 1011*t)) - noise
            assert lib.sf_writef_float(output, samples, RATE) == RATE, lib.sf_strerror(output)
    finally:
        assert lib.sf_close(output) == 0
    report['files'].append({'format': extension, 'supported': True, 'path': str(path),
        'bytes': path.stat().st_size, 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()})
(OUT / 'manifest.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(report, indent=2))
