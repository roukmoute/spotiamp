#!/usr/bin/env python3
"""Exercise real X11 windows and settings across process restarts in Xvfb.

Requires an isolated display, Openbox, xdotool and xprop. All application data
lives in a temporary directory; no Spotify account or network update is used.
"""

import argparse
import os
from pathlib import Path
import subprocess
import tempfile
import time


def wait_for(description, probe, timeout=5):
    deadline = time.monotonic() + timeout
    last = None
    while time.monotonic() < deadline:
        last = probe()
        if last:
            return last
        time.sleep(0.04)
    raise AssertionError(f"Timed out: {description}; last result: {last!r}")


class Fixture:
    def __init__(self, executable, root):
        self.executable = executable
        self.env = os.environ.copy()
        for suffix in ("CONFIG", "CACHE", "DATA", "STATE", "RUNTIME"):
            directory = root / suffix.lower()
            directory.mkdir(mode=0o700)
            self.env[f"XDG_{suffix}_HOME" if suffix != "RUNTIME" else "XDG_RUNTIME_DIR"] = str(directory)
        self.env.update(DBUS_SESSION_BUS_ADDRESS=f"unix:path={root}/no-session-bus",
                        LC_ALL="C", XMODIFIERS="@im=none")
        self.path = root / "config" / "Spotiamp" / "window-layout"
        self.path.parent.mkdir()
        self.log_path = root / "native.log"
        self.log = self.log_path.open("w+")
        self.app = None
        self.manager = None
        self.player = None
        self.library = None
        # A pre-existing WM means this is not the disposable display required
        # by this test. Never replace a user's window manager.
        try:
            assert "window id # 0x" not in self.command("xprop", "-root", "_NET_SUPPORTING_WM_CHECK"), (
                "Use a fresh xvfb-run display without an existing window manager"
            )
            self.manager = subprocess.Popen(["openbox", "--sm-disable"], env=self.env,
                                            stdout=self.log, stderr=self.log)
            wait_for("Openbox startup", lambda: "window id # 0x" in self.command(
                "xprop", "-root", "_NET_SUPPORTING_WM_CHECK"))
        except BaseException:
            self.cleanup()
            raise

    def command(self, *args, check=True):
        result = subprocess.run([str(arg) for arg in args], env=self.env,
                                capture_output=True, text=True, encoding="utf-8", timeout=5)
        if check and result.returncode:
            raise AssertionError(f"{args!r}: {result.stderr.strip()}")
        return result.stdout.strip()

    def windows(self):
        result = {}
        for window in self.command("xdotool", "search", "--name", "^Spotiamp", check=False).splitlines():
            title = self.command("xdotool", "getwindowname", window, check=False)
            if title.startswith("Spotiamp — "):
                result["library"] = window
            elif title.startswith("Spotiamp "):
                result["player"] = window
        return result

    def start(self, *args, library=True):
        assert self.app is None
        self.app = subprocess.Popen([str(self.executable), "--no-update-check", *args],
                                    env=self.env, stdout=self.log, stderr=self.log)

        def ready():
            assert self.app.poll() is None, f"Spotiamp exited during startup ({self.app.returncode})"
            found = self.windows()
            return found if "player" in found and ("library" in found) == library else None

        found = wait_for("application windows", ready)
        self.player, self.library = found["player"], found.get("library")
        # Let queued map/configure events settle before testing persistence.
        time.sleep(0.15)
        assert len(self.windows()) == (2 if library else 1)

    def position(self, window):
        values = dict(line.split("=", 1) for line in self.command(
            "xdotool", "getwindowgeometry", "--shell", window).splitlines())
        return int(values["X"]), int(values["Y"])

    def positions(self, player, library):
        wait_for(f"player at {player}", lambda: self.position(self.player) == player)
        if library is not None:
            wait_for(f"library at {library}", lambda: self.position(self.library) == library)

    def drag(self, window, destination):
        origin = self.position(window)
        self.command("xdotool", "windowactivate", "--sync", window,
                     "mousemove", "--window", window, 30, 9,
                     "mousedown", 1, "sleep", "0.12",
                     "mousemove_relative", "--", destination[0] - origin[0], destination[1] - origin[1],
                     "sleep", "0.12", "mouseup", 1)
        wait_for(f"drag to {destination}", lambda: self.position(window) == destination)
        time.sleep(0.12)

    def toggle_library(self, opened):
        self.command("xdotool", "windowactivate", "--sync", self.player,
                     "key", "--clearmodifiers", "F3")
        wait_for(f"library {'open' if opened else 'closed'}",
                 lambda: ("library" in self.windows()) == opened)
        self.library = self.windows().get("library")
        time.sleep(0.12)

    def stop(self, automatic=False):
        assert self.app is not None
        if not automatic:
            self.command("xdotool", "windowactivate", "--sync", self.player,
                         "key", "--clearmodifiers", "alt+F4")
        assert self.app.wait(timeout=5) == 0, "Spotiamp did not exit cleanly"
        self.app = None
        wait_for("native windows destroyed", lambda: not self.windows())
        self.player = self.library = None

    def seed(self, player, library, attached=True, opened=True):
        assert self.app is None
        self.path.write_text(f"v1 {player[0]} {player[1]} {library[0]} {library[1]} "
                             f"{int(attached)} {int(opened)} 1\n")

    def saved(self, player, library, attached, opened):
        expected = f"v1 {player[0]} {player[1]} {library[0]} {library[1]} {int(attached)} {int(opened)} 1\n"
        actual = self.path.read_text()
        assert actual == expected, f"Saved layout: expected {expected!r}, observed {actual!r}"

    def cleanup(self):
        for process in (self.app, self.manager):
            if process is not None and process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=3)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=3)
        self.log.close()


def run(f):
    f.seed((130, 90), (690, 120))
    f.start("--skin", "classic")
    f.positions((130, 90), (690, 120))
    before = f.path.stat().st_mtime_ns
    f.drag(f.player, (190, 150))
    f.positions((190, 150), (750, 180))
    assert f.path.stat().st_mtime_ns == before, "Layout was written before shutdown"
    f.stop()
    f.saved((190, 150), (750, 180), True, True)
    print("PASS: attached windows move together and save their exact offset at exit", flush=True)

    before = f.path.stat().st_mtime_ns
    f.start("--skin", "classic")
    f.positions((190, 150), (750, 180))
    f.stop()
    assert f.path.stat().st_mtime_ns == before, "Unchanged layout was rewritten"
    print("PASS: a real restart restores attached coordinates without rewriting unchanged settings", flush=True)

    f.start("--skin", "classic")
    f.drag(f.library, (900, 520))
    f.positions((190, 150), (900, 520))
    f.stop()
    f.saved((190, 150), (900, 520), False, True)
    f.start("--skin", "modern")
    f.positions((190, 150), (900, 520))
    f.drag(f.player, (230, 110))
    f.positions((230, 110), (900, 520))
    f.toggle_library(False)
    f.stop()
    f.saved((230, 110), (900, 520), False, False)
    print("PASS: physical detachment survives a restart with the Modern skin", flush=True)

    f.start(library=False)
    f.positions((230, 110), None)
    f.drag(f.player, (270, 160))
    f.toggle_library(True)
    f.positions((270, 160), (900, 520))
    f.toggle_library(False)
    f.stop()
    f.saved((270, 160), (900, 520), False, False)
    print("PASS: closed library stays closed at restart and F3 restores its separate position", flush=True)

    before = f.path.stat().st_mtime_ns
    f.start("--library")
    f.positions((270, 160), (900, 520))
    f.stop()
    f.saved((270, 160), (900, 520), False, False)
    assert f.path.stat().st_mtime_ns == before
    f.seed((130, 90), (690, 120))
    before = f.path.stat().st_mtime_ns
    f.start("--no-library", library=False)
    f.stop()
    f.saved((130, 90), (690, 120), True, True)
    assert f.path.stat().st_mtime_ns == before
    print("PASS: --library and --no-library overrides do not replace saved visibility", flush=True)

    f.start()
    f.toggle_library(False)
    f.drag(f.player, (190, 150))
    f.toggle_library(True)
    f.positions((190, 150), (750, 180))
    f.drag(f.player, (230, 190))
    f.positions((230, 190), (790, 220))
    f.stop()
    f.saved((230, 190), (790, 220), True, True)
    print("PASS: reopening an attached library follows the player displacement while closed", flush=True)

    f.start("--quit-after", "3")
    f.positions((230, 190), (790, 220))
    f.command("xdotool", "windowminimize", f.player)
    wait_for("both windows minimized", lambda: all("Iconic" in f.command(
        "xprop", "-id", window, "WM_STATE") for window in (f.player, f.library)))
    f.stop(automatic=True)
    f.saved((230, 190), (790, 220), True, True)
    f.start()
    f.positions((230, 190), (790, 220))
    f.stop()
    print("PASS: quitting while minimized retains normal coordinates and attachment", flush=True)

    f.seed((-30000, -30000), (-29440, -29970))
    f.start()
    player, library = f.position(f.player), f.position(f.library)
    width, height = map(int, f.command("xdotool", "getdisplaygeometry").split())
    for point in (player, library):
        assert 0 <= point[0] < width - 96 and 0 <= point[1] < height - 18, point
    assert (library[0] - player[0], library[1] - player[1]) == (560, 30)
    f.stop()
    f.saved(player, library, True, True)
    f.start()
    f.positions(player, library)
    f.stop()
    print("PASS: a missing monitor is recovered on screen with the attached offset preserved", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--executable", type=Path, required=True)
    args = parser.parse_args()
    executable = args.executable.resolve(strict=True)
    assert os.environ.get("DISPLAY"), "Run inside xvfb-run"
    with tempfile.TemporaryDirectory(prefix="spotiamp-layout-") as directory:
        fixture = Fixture(executable, Path(directory))
        try:
            run(fixture)
        except BaseException:
            fixture.log.flush()
            print(fixture.log_path.read_text(errors="replace"), flush=True)
            raise
        finally:
            fixture.cleanup()
    print("PASS: all real-process layout restart checks", flush=True)


if __name__ == "__main__":
    main()
