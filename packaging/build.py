#!/usr/bin/env python3
"""Build native installers using the release binary produced by GitHub CI."""
import argparse
import hashlib
import os
from pathlib import Path
import plistlib
import shutil
import struct
import subprocess
import tempfile
import re

ROOT = Path(__file__).resolve().parent.parent
IDENTIFIER = "io.github.zzhtl.notepad"


def run(*args):
    subprocess.run([str(arg) for arg in args], check=True)


def copy(source, destination):
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)


def mark_installed(directory):
    (directory / "notepad.install").write_text("Notepad installer layout\n", encoding="utf-8")


def linux(binary, stage, output, version):
    payload = stage / "root"
    executable = payload / "usr/lib/notepad/notepad"
    copy(binary, executable)
    executable.chmod(0o755)
    mark_installed(executable.parent)
    (payload / "usr/bin").mkdir(parents=True)
    (payload / "usr/bin/notepad").symlink_to("../lib/notepad/notepad")
    copy(ROOT / "packaging/notepad.desktop", payload / "usr/share/applications/notepad.desktop")
    copy(ROOT / "assets/icon.png", payload / "usr/share/icons/hicolor/256x256/apps/notepad.png")
    copy(ROOT / "LICENSE", payload / "usr/share/doc/notepad/copyright")
    copy(ROOT / "README.md", payload / "usr/share/doc/notepad/README.md")
    control = payload / "DEBIAN/control"
    control.parent.mkdir()
    size = sum(file.stat().st_size for file in payload.rglob("*") if file.is_file() and not file.is_symlink()) // 1024 + 1
    control.write_text(
        f"Package: notepad\nVersion: {version}\nArchitecture: amd64\n"
        "Maintainer: zzhtl <zclgudu1224@126.com>\n"
        "Section: editors\nPriority: optional\n"
        f"Installed-Size: {size}\n"
        "Depends: libc6 (>= 2.35), libgcc-s1, libxkbcommon0, libwayland-client0, "
        "libx11-6, libxcb1, libxcb-render0, libxcb-shape0, libxcb-xfixes0, "
        "libegl1, libgl1, xdg-desktop-portal, "
        "xdg-desktop-portal-gtk | xdg-desktop-portal-kde\n"
        "Homepage: https://github.com/zzhtl/notepad\n"
        "Description: Local Markdown notebook\n"
        " Desktop notes with folders, live preview and Markdown directory import/export.\n",
        encoding="utf-8",
    )
    run("desktop-file-validate", payload / "usr/share/applications/notepad.desktop")
    package = output / f"notepad-v{version}-linux-x86_64.deb"
    run("dpkg-deb", "--root-owner-group", "--build", payload, package)
    run("dpkg-deb", "--info", package)
    return executable


def windows(binary, stage, output, version):
    copy(binary, stage / "notepad.exe")
    mark_installed(stage)
    for document in ("LICENSE", "README.md"):
        copy(ROOT / document, stage / document)
    # ICO supports the existing 256px PNG directly; no resampling or new artwork.
    png = (ROOT / "assets/icon.png").read_bytes()
    header = struct.pack("<HHH", 0, 1, 1)
    entry = struct.pack("<BBBBHHII", 0, 0, 0, 0, 1, 32, len(png), 22)
    (stage / "notepad.ico").write_bytes(header + entry + png)
    compiler = shutil.which("ISCC.exe")
    if not compiler:
        compiler = Path(os.environ["ProgramFiles(x86)"]) / "Inno Setup 6/ISCC.exe"
    run(compiler, f"/DAppVersion={version}", f"/DSourceDir={stage}",
        f"/DDestinationDir={output}", ROOT / "packaging/windows.iss")
    return stage / "notepad.exe"


def macos(binary, stage, output, version, target):
    payload = stage / "root"
    app = payload / "Applications/Notepad.app"
    executable = app / "Contents/MacOS/notepad"
    copy(binary, executable)
    executable.chmod(0o755)
    mark_installed(executable.parent)
    resources = app / "Contents/Resources"
    resources.mkdir(parents=True)
    for document in ("LICENSE", "README.md"):
        copy(ROOT / document, resources / document)
    iconset = stage / "notepad.iconset"
    iconset.mkdir()
    for name, size in (("16x16", 16), ("16x16@2x", 32), ("32x32", 32),
                       ("32x32@2x", 64), ("128x128", 128),
                       ("128x128@2x", 256), ("256x256", 256)):
        run("sips", "-z", size, size, ROOT / "assets/icon.png", "--out", iconset / f"icon_{name}.png")
    run("iconutil", "-c", "icns", iconset, "-o", resources / "notepad.icns")
    info = {
        "CFBundleIdentifier": IDENTIFIER,
        "CFBundleName": "Notepad",
        "CFBundleDisplayName": "Notepad",
        "CFBundleExecutable": "notepad",
        "CFBundleIconFile": "notepad.icns",
        "CFBundlePackageType": "APPL",
        "CFBundleVersion": version,
        "CFBundleShortVersionString": version,
        "LSMinimumSystemVersion": "11.0",
        "NSHighResolutionCapable": True,
        "NSPrincipalClass": "NSApplication",
    }
    (app / "Contents/Info.plist").write_bytes(plistlib.dumps(info))
    run("plutil", "-lint", app / "Contents/Info.plist")
    # Ad-hoc signing permits execution on Apple Silicon; it is not Developer ID notarization.
    run("codesign", "--force", "--sign", "-", app)
    run("codesign", "--verify", "--deep", "--strict", app)
    component_plist = stage / "components.plist"
    component_plist.write_bytes(plistlib.dumps([{
        "RootRelativeBundlePath": "Applications/Notepad.app",
        "BundleIsRelocatable": False,
        "BundleIsVersionChecked": True,
        "BundleHasStrictIdentifier": True,
        "BundleOverwriteAction": "upgrade",
    }]))
    component = stage / "notepad-component.pkg"
    run("pkgbuild", "--root", payload, "--identifier", IDENTIFIER, "--version", version,
        "--install-location", "/", "--component-plist", component_plist, component)
    arch = "arm64" if target.startswith("aarch64") else "x86_64"
    requirements = stage / "requirements.plist"
    requirements.write_bytes(plistlib.dumps({"arch": [arch], "os": ["11.0"]}))
    package = output / f"notepad-v{version}-macos-{arch}.pkg"
    run("productbuild", "--product", requirements, "--package", component, package)
    run("pkgutil", "--expand-full", package, stage / "expanded")
    assert list((stage / "expanded").rglob("notepad.install")), "Installer must include installed-layout marker"
    return executable


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", required=True, choices=(
        "x86_64-unknown-linux-gnu", "x86_64-pc-windows-msvc",
        "aarch64-apple-darwin", "x86_64-apple-darwin"))
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--output", type=Path, default=ROOT / "target/installers")
    args = parser.parse_args()
    version = re.search(r'^version = "([^"]+)"', (ROOT / "Cargo.toml").read_text(), re.M)[1]
    filename = "notepad.exe" if "windows" in args.target else "notepad"
    binary = (args.binary or ROOT / "target" / args.target / "release" / filename).resolve(strict=True)
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    staging = ROOT / "target/package"
    staging.mkdir(parents=True, exist_ok=True)
    stage = Path(tempfile.mkdtemp(prefix=args.target + "-", dir=staging))
    if "linux" in args.target:
        packaged_binary = linux(binary, stage, output, version)
    elif "windows" in args.target:
        packaged_binary = windows(binary, stage, output, version)
    else:
        packaged_binary = macos(binary, stage, output, version, args.target)
    (staging / f"{args.target}.sha256").write_text(hashlib.sha256(packaged_binary.read_bytes()).hexdigest())


if __name__ == "__main__":
    main()
