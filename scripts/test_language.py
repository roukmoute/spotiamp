#!/usr/bin/env python3
"""Test language changes through real X11 controls and process restarts.

Run in a fresh xvfb-run display. Reuses the isolated Openbox fixture from the
window-layout smoke test; no account, system keyring or update service is used.
"""

import argparse
import os
from pathlib import Path
import tempfile

from test_window_layout import Fixture, wait_for


def assert_titles(fixture, language):
    reader = "lecteur Spotify" if language == "fr" else "Spotify player"
    library = "Bibliothèque Spotify" if language == "fr" else "Spotify Library"

    def matches():
        title = fixture.command("xdotool", "getwindowname", fixture.player)
        if not title.startswith("Spotiamp ") or not title.endswith(f" — {reader}"):
            return False
        if fixture.library is not None:
            title = fixture.command("xdotool", "getwindowname", fixture.library)
            if title != f"Spotiamp — {library}":
                return False
        return True

    wait_for(f"{language} native reader and library titles", matches)


def choose(fixture, preference):
    # The Player tab is already open. Exercise actual pointer press/release,
    # including the controls' hit-testing and settings write path.
    x = {"auto": 216, "fr": 328, "en": 440}[preference]
    fixture.command("xdotool", "windowactivate", "--sync", fixture.player,
                    "mousemove", "--window", fixture.player, x, 220,
                    "click", 1)
    path = fixture.path.parent / "language"
    wait_for(f"saved {preference} choice", lambda: path.exists() and path.read_text() == f"{preference}\n")


def run(fixture):
    preference = fixture.path.parent / "language"
    skin = fixture.path.parent / "skin"
    skin.write_text("modern\n")
    skin_before = (skin.read_bytes(), skin.stat().st_mtime_ns)
    fixture.seed((60, 60), (620, 60))

    for language in ("fr", "en"):
        fixture.start("--language", language, "--about", "player")
        assert_titles(fixture, language)
        fixture.stop()
        assert not preference.exists(), "A CLI language override was saved"
    print("PASS: French and English CLI overrides update both titles without saving a preference", flush=True)

    fixture.start("--about", "player")
    assert_titles(fixture, "en")  # Isolated fixture uses LC_ALL=C.
    process, windows = fixture.app.pid, (fixture.player, fixture.library)
    choose(fixture, "fr")
    assert_titles(fixture, "fr")
    assert fixture.app.poll() is None and fixture.app.pid == process and fixture.windows() == {
        "player": windows[0], "library": windows[1]
    }, "Switching language recreated the process or native windows"
    fixture.stop()
    fixture.start("--about", "player")
    assert_titles(fixture, "fr")
    print("PASS: a real About click updates both live windows and persists across process restart", flush=True)

    choose(fixture, "en")
    assert_titles(fixture, "en")
    fixture.command("xdotool", "key", "--clearmodifiers", "Escape")
    fixture.toggle_library(False)
    fixture.command("xdotool", "key", "--clearmodifiers", "F2", "sleep", "0.12")
    choose(fixture, "fr")
    assert_titles(fixture, "fr")
    fixture.command("xdotool", "key", "--clearmodifiers", "Escape")
    fixture.toggle_library(True)
    assert_titles(fixture, "fr")
    fixture.stop()
    print("PASS: a library reopened after a language change uses the new language", flush=True)

    saved = (preference.read_bytes(), preference.stat().st_mtime_ns)
    fixture.start("--language", "en")
    assert_titles(fixture, "en")
    fixture.stop()
    assert (preference.read_bytes(), preference.stat().st_mtime_ns) == saved, (
        "A temporary CLI override replaced the user's saved preference"
    )
    fixture.start("--about", "player")
    assert_titles(fixture, "fr")
    choose(fixture, "auto")
    assert_titles(fixture, "en")
    fixture.stop()
    print("PASS: temporary overrides preserve a saved choice and Auto immediately restores the system language", flush=True)

    saved = (preference.read_bytes(), preference.stat().st_mtime_ns)
    for system_locale, expected in (("fr_CA.UTF-8", "fr"), ("en_GB.UTF-8", "en"), ("de_DE.UTF-8", "en")):
        fixture.env["LC_ALL"] = system_locale
        fixture.start()
        assert_titles(fixture, expected)
        fixture.stop()
        assert (preference.read_bytes(), preference.stat().st_mtime_ns) == saved, (
            "Automatic language detection rewrote the preference"
        )
    assert (skin.read_bytes(), skin.stat().st_mtime_ns) == skin_before, (
        "Changing language modified the remembered skin"
    )
    assert not list(preference.parent.glob(".language-*.tmp")), "Temporary settings files were left behind"
    print("PASS: Auto is remembered, detects French/English, falls back to English, and preserves the skin", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--executable", type=Path, required=True)
    args = parser.parse_args()
    executable = args.executable.resolve(strict=True)
    assert os.environ.get("DISPLAY"), "Run inside xvfb-run"
    with tempfile.TemporaryDirectory(prefix="spotiamp-language-") as directory:
        fixture = Fixture(executable, Path(directory))
        try:
            run(fixture)
        except BaseException:
            fixture.log.flush()
            print(fixture.log_path.read_text(errors="replace"), flush=True)
            raise
        finally:
            fixture.cleanup()
    print("PASS: all real-process language restart checks", flush=True)


if __name__ == "__main__":
    main()
