#!/usr/bin/env python3
"""Isolated, interleaved Linux GUI measurements. No Spotify, network or audio.

Warm file-cache start, not cold boot: versions and hashes are read beforehand.
First-paint latency is measured using Xlib (5ms polling), not process startup alone.
Only the app's CPU/memory are sampled; Xvfb/Openbox are excluded and held constant.
"""
import argparse
import ctypes as C
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import platform
import random
import select
import shutil
import statistics
import subprocess
import time

ROOT = Path(__file__).resolve().parent
WORK = ROOT.parent
WM = WORK / 'window-test-wm/root'


def fields(path):
    result = {}
    for line in path.read_text().splitlines():
        key, _, text = line.partition(':')
        parts = text.split()
        if parts and parts[0].isdigit():
            result[key] = int(parts[0]) * (1024 if parts[-1] == 'kB' else 1)
    return result


def sample(pid, start):
    proc = Path('/proc') / str(pid)
    status, rollup = fields(proc / 'status'), fields(proc / 'smaps_rollup')
    stat = (proc / 'stat').read_text().rsplit(')', 1)[1].split()
    return {'elapsed_seconds': time.monotonic() - start,
        'rss_bytes': rollup['Rss'], 'pss_bytes': rollup['Pss'],
        'private_resident_bytes': rollup.get('Private_Clean', 0) + rollup.get('Private_Dirty', 0),
        'cpu_seconds': (int(stat[11]) + int(stat[12])) / os.sysconf('SC_CLK_TCK'),
        'threads': status['Threads']}


class X11:
    def __init__(self, display):
        self.lib = C.CDLL('libX11.so.6')
        signatures = {
            'XOpenDisplay': (C.c_void_p, [C.c_char_p]),
            'XCloseDisplay': (C.c_int, [C.c_void_p]),
            'XInternAtom': (C.c_ulong, [C.c_void_p, C.c_char_p, C.c_int]),
            'XDefaultRootWindow': (C.c_ulong, [C.c_void_p]),
            'XFetchName': (C.c_int, [C.c_void_p, C.c_ulong, C.POINTER(C.c_void_p)]),
            'XFree': (C.c_int, [C.c_void_p]),
            'XGetWindowProperty': (C.c_int, [C.c_void_p, C.c_ulong, C.c_ulong, C.c_long,
                C.c_long, C.c_int, C.c_ulong, C.POINTER(C.c_ulong), C.POINTER(C.c_int),
                C.POINTER(C.c_ulong), C.POINTER(C.c_ulong), C.POINTER(C.c_void_p)]),
            'XGetImage': (C.c_void_p, [C.c_void_p, C.c_ulong, C.c_int, C.c_int,
                C.c_uint, C.c_uint, C.c_ulong, C.c_int]),
            'XGetPixel': (C.c_ulong, [C.c_void_p, C.c_int, C.c_int]),
            'XDestroyImage': (C.c_int, [C.c_void_p]),
        }
        for name, (result, arguments) in signatures.items():
            function = getattr(self.lib, name)
            function.restype, function.argtypes = result, arguments
        self.display = self.lib.XOpenDisplay(display.encode())
        assert self.display, 'Cannot connect to isolated Xvfb'
        self.root = self.lib.XDefaultRootWindow(self.display)
        self.atoms = {}

    def prop(self, window, name):
        if name not in self.atoms:
            self.atoms[name] = self.lib.XInternAtom(self.display, name.encode(), 0)
        atom = self.atoms[name]
        actual, fmt, count, remaining, data = C.c_ulong(), C.c_int(), C.c_ulong(), C.c_ulong(), C.c_void_p()
        status = self.lib.XGetWindowProperty(self.display, window, atom, 0, 1024, 0, 0,
            C.byref(actual), C.byref(fmt), C.byref(count), C.byref(remaining), C.byref(data))
        try:
            if status or not data or fmt.value != 32:
                return []
            return list(C.cast(data, C.POINTER(C.c_ulong))[:count.value])
        finally:
            if data:
                self.lib.XFree(data)

    def first_paint(self, pid):
        for window in self.prop(self.root, '_NET_CLIENT_LIST'):
            # minifb does not publish _NET_WM_PID. This display has exactly one
            # application at a time; confirm its title and our Popen is alive.
            title = C.c_void_p()
            try:
                if not self.lib.XFetchName(self.display, window, C.byref(title)) or not title:
                    continue
                if not C.string_at(title).startswith(b'Spotiamp 0.1.5 '):
                    continue
            finally:
                if title:
                    self.lib.XFree(title)
            state = self.prop(window, 'WM_STATE')
            if not state or state[0] != 1:  # Normal, managed and mapped.
                continue
            # The clock colon is a solid green 3x4 rectangle at (87,49),
            # independent of focus, login state, text, quality and updater.
            image = self.lib.XGetImage(self.display, window, 87, 49, 1, 1, C.c_ulong(-1), 2)
            if image:
                try:
                    if self.lib.XGetPixel(image, 0, 0) & 0xffffff == 0x5bff35:
                        return window
                finally:
                    self.lib.XDestroyImage(image)
        return None

    def close(self):
        self.lib.XCloseDisplay(self.display)


def stats(values):
    values = sorted(values)
    return {'min': values[0], 'median': statistics.median(values),
            'p95': values[max(0, (95 * len(values) + 99) // 100 - 1)], 'max': values[-1]}


def info(path, env):
    path = Path(path).resolve(strict=True)
    version = subprocess.check_output([str(path), '--help'], env=env, text=True, timeout=5).splitlines()[0]
    assert version.startswith('Spotiamp 0.1.5'), version
    return {'path': str(path), 'version': version, 'bytes': path.stat().st_size,
            'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', action='append', required=True, help='NAME=/absolute/path')
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--runs', type=int, default=8)
    parser.add_argument('--idle-repeats', type=int, default=2)
    parser.add_argument('--warmup', type=float, default=20)
    parser.add_argument('--seconds', type=float, default=30)
    parser.add_argument('--wm-root', type=Path, default=WM if WM.exists() else None,
                        help='Optional extracted Openbox package tree; otherwise use system Openbox')
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    binaries = dict(item.split('=', 1) for item in args.binary)
    env = os.environ.copy()
    for variable, directory in [('XDG_CONFIG_HOME', 'config'), ('XDG_CACHE_HOME', 'cache'),
            ('XDG_DATA_HOME', 'data'), ('XDG_STATE_HOME', 'state'), ('XDG_RUNTIME_DIR', 'run')]:
        path = out / directory
        path.mkdir(mode=0o700)
        env[variable] = str(path)
    env.update({'DBUS_SESSION_BUS_ADDRESS': 'unix:path=' + str(out / 'run/absent-bus'),
        'XMODIFIERS': '@im=none', 'LC_ALL': 'C.UTF-8'})
    if args.wm_root:
        wm_root = args.wm_root.resolve()
        manager_binary, manager_config = wm_root / 'usr/bin/openbox', wm_root / 'etc/xdg/openbox/rc.xml'
        env.update({'XDG_DATA_DIRS': str(wm_root / 'usr/share') + ':/usr/local/share:/usr/share',
            'XDG_CONFIG_DIRS': str(wm_root / 'etc/xdg'),
            'LD_LIBRARY_PATH': str(wm_root / 'usr/lib/x86_64-linux-gnu')})
    else:
        manager_binary, manager_config = shutil.which('openbox'), '/etc/xdg/openbox/rc.xml'
        assert manager_binary, 'Install Openbox or supply --wm-root'
    binaries = {name: info(path, env) for name, path in binaries.items()}
    report = {'time_utc': dt.datetime.now(dt.timezone.utc).isoformat(),
        'platform': platform.platform(), 'cpu': platform.processor(), 'binaries': binaries,
        'cpu_count': os.cpu_count(), 'load_average_start': os.getloadavg(),
        'scope': 'Disconnected GUI, one 560x470 window, updater disabled, no audio; app process only',
        'cache': 'Warm filesystem cache; version/hash were read before launch',
        'isolation': 'Private Xvfb/Openbox, XDG directories and absent D-Bus address; no personal credentials',
        'startup_poll_seconds': .005, 'warmup_seconds': args.warmup,
        'idle_seconds': args.seconds, 'startup': [], 'idle': [], 'success': False}
    processes, logs = [], []
    connection = None

    def launch(command, label):
        log = (out / (label + '.log')).open('wb')
        logs.append(log)
        process = subprocess.Popen(command, env=env, stdout=log, stderr=log)
        processes.append(process)
        return process

    def command(*items):
        return subprocess.check_output(items, env=env, timeout=10)

    def terminate(process):
        if process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=5)

    def launch_app(name, label):
        start = time.perf_counter()
        process = launch([binaries[name]['path'], '--no-update-check'], label)
        while time.perf_counter() - start < 10:
            assert process.poll() is None, f'{label} exited {process.returncode}'
            window = connection.first_paint(process.pid)
            if window:
                return process, window, (time.perf_counter() - start) * 1000
            time.sleep(.005)
        raise TimeoutError(f'{label} first paint')

    def save():
        (out / 'report.json').write_text(json.dumps(report, indent=2) + '\n')

    try:
        read_fd, write_fd = os.pipe()
        log = (out / 'xvfb.log').open('wb'); logs.append(log)
        xvfb = subprocess.Popen(['Xvfb', '-displayfd', str(write_fd), '-screen', '0',
            '1000x800x24', '-nolisten', 'tcp'], pass_fds=(write_fd,), stdout=log, stderr=log)
        processes.append(xvfb)
        os.close(write_fd)
        assert select.select([read_fd], [], [], 10)[0], 'Xvfb timeout'
        env['DISPLAY'] = ':' + os.read(read_fd, 32).decode().strip()
        os.close(read_fd)
        connection = X11(env['DISPLAY'])
        marker = '_SPOTIAMP_BENCH_READY_' + str(os.getpid())
        manager = launch([str(manager_binary), '--sm-disable', '--config-file',
            str(manager_config), '--startup',
            f'xprop -root -f {marker} 8s -set {marker} ready'], 'openbox')
        deadline = time.monotonic() + 10
        while b'= "ready"' not in command('xprop', '-root', marker):
            assert manager.poll() is None and time.monotonic() < deadline, 'Openbox startup'
            time.sleep(.05)
        command('xdotool', 'mousemove', '950', '750')
        order = list(binaries)
        randomizer = random.Random(20261022)
        for iteration in range(args.runs):
            randomizer.shuffle(order)
            for name in order:
                process, window, elapsed = launch_app(name, f'startup-{iteration}-{name}')
                report['startup'].append({'binary': name, 'iteration': iteration,
                    'first_managed_paint_ms': elapsed})
                # Shutdown latency is not part of this benchmark. Terminate
                # only our isolated, disconnected child after recording.
                terminate(process)
                time.sleep(.08)
            save()
            print(f'Startup round {iteration + 1}/{args.runs} complete', flush=True)
        for iteration in range(args.idle_repeats):
            randomizer.shuffle(order)
            for name in order:
                process, window, elapsed = launch_app(name, f'idle-{iteration}-{name}')
                command('xdotool', 'windowactivate', '--sync', str(window))
                start = time.monotonic()
                print(f'Idle {name} repeat{iteration + 1}: {args.warmup}s + {args.seconds}s', flush=True)
                time.sleep(args.warmup)
                measured = time.monotonic()
                samples = [sample(process.pid, start)]
                for index in range(1, int(args.seconds * 2) + 1):
                    time.sleep(max(0, measured + index / 2 - time.monotonic()))
                    samples.append(sample(process.pid, start))
                first, last = samples[0], samples[-1]
                seconds = last['elapsed_seconds'] - first['elapsed_seconds']
                report['idle'].append({'binary': name, 'iteration': iteration, 'samples': samples,
                    'elapsed_seconds': seconds,
                    'cpu_percent_one_core': (last['cpu_seconds'] - first['cpu_seconds']) * 100 / seconds,
                    'memory': {key: stats([row[key] for row in samples]) for key in
                        ['rss_bytes', 'pss_bytes', 'private_resident_bytes', 'threads']}})
                terminate(process)
                save()
        report['startup_summary_ms'] = {name: stats([row['first_managed_paint_ms']
            for row in report['startup'] if row['binary'] == name]) for name in binaries}
        report['success'] = True
        report['load_average_end'] = os.getloadavg()
    finally:
        save()
        if connection:
            connection.close()
        for process in reversed(processes):
            terminate(process)
        for log in logs:
            log.close()
    print(json.dumps(report['startup_summary_ms'], indent=2))


if __name__ == '__main__':
    main()
