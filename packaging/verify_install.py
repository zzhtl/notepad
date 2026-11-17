#!/usr/bin/env python3
"""Install, start and reinstall the real package on a disposable GitHub runner."""
import argparse
import hashlib
import os
from pathlib import Path
import sqlite3
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parent.parent
WORK = ROOT / "target/install-check"


def run(*command):
    subprocess.run([str(part) for part in command], check=True)


def launch(binary, database, environment, phase):
    with (WORK / f"{phase}.log").open("wb") as log:
        app = subprocess.Popen([str(binary)], env=environment, stdout=log, stderr=log)
        try:
            # Keep the GUI alive after database initialization, to catch renderer/startup failures.
            for _ in range(40):
                time.sleep(0.2)
                if app.poll() is not None:
                    raise RuntimeError(f"Application exited during {phase}; see target/install-check/{phase}.log")
            assert database.is_file(), f"Database missing at {database}"
            assert not (binary.parent / "notepad.db").exists(), "Database must not be inside installation"
            with sqlite3.connect(database) as connection:
                connection.execute("SELECT COUNT(*) FROM notes").fetchone()
        finally:
            if app.poll() is None:
                app.terminate()
                try:
                    app.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    app.kill()
                    app.wait()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", required=True)
    args = parser.parse_args()
    WORK.mkdir(parents=True, exist_ok=True)
    environment = {**os.environ, "ICED_BACKEND": "tiny-skia"}
    if sys.platform == "win32":
        package = next((ROOT / "target/installers").glob("*.exe"))
        installed = WORK / "Notepad"
        binary = installed / "notepad.exe"
        database = Path(os.environ["LOCALAPPDATA"]) / "notepad/notepad.db"
        install = [package, "/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART", "/SP-",
                   f"/DIR={installed}", f"/LOG={WORK / 'setup.log'}"]
        uninstall = [installed / "unins000.exe", "/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART"]
    elif sys.platform == "darwin":
        package = next((ROOT / "target/installers").glob("*.pkg"))
        binary = Path("/Applications/Notepad.app/Contents/MacOS/notepad")
        database = Path.home() / "Library/Application Support/notepad/notepad.db"
        install = ["sudo", "installer", "-pkg", package, "-target", "/"]
        uninstall = None
    else:
        package = next((ROOT / "target/installers").glob("*.deb"))
        binary = Path("/usr/lib/notepad/notepad")
        environment["XDG_DATA_HOME"] = str(WORK / "data")
        database = WORK / "data/notepad/notepad.db"
        install = ["sudo", "apt-get", "install", "--reinstall", "-y", package]
        uninstall = ["sudo", "apt-get", "remove", "-y", "notepad"]
    assert not database.exists(), "Use a fresh CI runner; do not overwrite existing user data"
    run(*install)
    assert (binary.parent / "notepad.install").is_file()
    expected = (ROOT / "target/package" / f"{args.target}.sha256").read_text().strip()
    assert hashlib.sha256(binary.read_bytes()).hexdigest() == expected
    if sys.platform == "darwin":
        run("codesign", "--verify", "--deep", "--strict", "/Applications/Notepad.app")
    elif sys.platform == "win32":
        shortcut = Path(os.environ["APPDATA"]) / "Microsoft/Windows/Start Menu/Programs/Notepad.lnk"
        assert shortcut.is_file(), "Start Menu shortcut missing"
    else:
        run("desktop-file-validate", "/usr/share/applications/notepad.desktop")
    launch(binary, database, environment, "first-launch")
    # Seed valid data through the real database schema, then reinstall and start again.
    with sqlite3.connect(database) as connection:
        connection.execute("PRAGMA foreign_keys = ON")
        connection.execute("INSERT INTO folders (id,name,created_at,updated_at) VALUES ('installer-folder','Installer test','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')")
        connection.execute("INSERT INTO notes (id,folder_id,title,content,created_at,updated_at) VALUES ('installer-note','installer-folder','Persistent data','Retained after reinstall','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')")
    run(*install)
    launch(binary, database, environment, "after-reinstall")
    if uninstall:
        run(*uninstall)
        assert not binary.exists(), "Uninstaller did not remove application"
    with sqlite3.connect(database) as connection:
        content = connection.execute("SELECT content FROM notes WHERE id='installer-note'").fetchone()
        assert content == ("Retained after reinstall",), "Installation or removal changed user data"
    print(f"Installer verified on {args.target}: install, GUI launch, reinstall, retained note data", flush=True)


if __name__ == "__main__":
    main()
