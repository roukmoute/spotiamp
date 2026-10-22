#!/usr/bin/env python3
"""Exercise remembered playback through real X11 controls and process restarts.

All data lives in the isolated window-layout fixture. Synthetic Spotify IDs
are never played: transport resume/seek is checked separately with a fake player.
"""

import argparse
import json
import os
from pathlib import Path
import tempfile
import time

from test_window_layout import Fixture, wait_for


def seed(path):
    rows = [{"uri": f"spotify:track:{index + 1:022d}",
             "title": f"Saved track {index + 1:02d}", "artist": "Restart fixture",
             "duration_ms": 180000, "title_kind": "metadata"}
            for index in range(20)]
    # Same song twice: restoring by URI alone would select the wrong occurrence.
    rows[3] = rows[9].copy()
    value = {"schema": 1, "queue": rows, "current": 9, "position_ms": 42000,
             "playlist": "spotify:playlist:0000000000000000000002",
             "volume": 32768, "shuffle": True, "repeat": True}
    path.write_text(json.dumps(value), encoding="utf-8")
    return value


def read(path):
    return json.loads(path.read_text(encoding="utf-8"))


def run(fixture):
    fixture.seed((60, 60), (620, 60))
    path = fixture.path.parent / "playback-session.json"
    skin = fixture.path.parent / "skin"
    skin.write_text("modern\n")
    language = fixture.path.parent / "language"
    language.write_text("fr\n")
    preserved = {p: (p.read_bytes(), p.stat().st_mtime_ns) for p in (skin, language)}

    for appearance in ("classic", "modern"):
        expected = seed(path)
        before = path.stat().st_mtime_ns
        fixture.start("--skin", appearance)
        time.sleep(0.3)
        fixture.stop()
        assert read(path) == expected, "Opening/closing reset the saved playback"
        assert path.stat().st_mtime_ns == before, "Unchanged playback was rewritten"
        fixture.start("--skin", appearance)
        # The restored tenth occurrence should be centered (fifth visible row).
        # Clicking that physical row then Delete proves both scrolling and
        # occurrence identity; an unscrolled queue would remove row five.
        fixture.command("xdotool", "windowactivate", "--sync", fixture.player,
                        "mousemove", "--window", fixture.player, 100, 329,
                        "click", 1, "key", "--clearmodifiers", "Delete", "sleep", "0.15")
        fixture.stop()
        expected["queue"].pop(9)
        actual = read(path)
        assert actual["queue"] == expected["queue"], "The wrong queue row was restored/removed"
        assert actual["current"] is None and actual["position_ms"] == 0
    print("PASS: both skins restore the exact queue occurrence; real Delete and immediate shutdown save it", flush=True)

    seed(path)
    fixture.start()
    fixture.command("xdotool", "windowactivate", "--sync", fixture.player,
                    "mousemove", "--window", fixture.player, 155, 427,
                    "click", 1, "sleep", "0.15")
    fixture.stop()
    assert read(path)["queue"] == [] and read(path)["playlist"] is None
    empty = read(path)
    fixture.start()
    fixture.stop()
    assert read(path) == empty, "Clearing the queue was undone at restart"
    print("PASS: Clear persists and does not resurrect the previous playlist", flush=True)

    expected = seed(path)
    fixture.start()
    fixture.command("xdotool", "windowactivate", "--sync", fixture.player,
                    "mousemove", "--window", fixture.player, 470, 126, "click", 1)
    wait_for("periodic changed-volume checkpoint", lambda: read(path)["volume"] != expected["volume"], timeout=19)
    checkpoint = read(path)
    assert checkpoint["current"] == 9 and checkpoint["position_ms"] == 42000
    assert checkpoint["queue"] == expected["queue"] and checkpoint["playlist"] == expected["playlist"]
    # Abrupt termination intentionally skips normal shutdown: the next launch
    # must recover the periodic checkpoint, not only an exit-time snapshot.
    fixture.app.kill()
    fixture.app.wait(timeout=5)
    fixture.app = None
    wait_for("killed windows destroyed", lambda: not fixture.windows())
    fixture.player = fixture.library = None
    before = path.stat().st_mtime_ns
    fixture.start()
    fixture.stop()
    assert read(path) == checkpoint and path.stat().st_mtime_ns == before
    assert all((p.read_bytes(), p.stat().st_mtime_ns) == value for p, value in preserved.items())
    assert not list(path.parent.glob(".playback-session-*.tmp"))
    print("PASS: periodic checkpoint survives abrupt termination without changing skin/language preferences", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--executable", type=Path, required=True)
    args = parser.parse_args()
    assert os.environ.get("DISPLAY"), "Run inside a fresh xvfb-run display"
    with tempfile.TemporaryDirectory(prefix="spotiamp-playback-") as directory:
        fixture = Fixture(args.executable.resolve(strict=True), Path(directory))
        try:
            run(fixture)
        except BaseException:
            fixture.log.flush()
            print(fixture.log_path.read_text(errors="replace"), flush=True)
            raise
        finally:
            fixture.cleanup()
    print("PASS: all real-process playback restart checks", flush=True)


if __name__ == "__main__":
    main()
