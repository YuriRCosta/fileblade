from __future__ import annotations

import json
import re
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOCKET = "data-goblin.fileblade/blade"
HOST_ID = "data-goblin.fileblade"
PLUGIN_ID = re.compile(r"^[a-z0-9_-]+\.[a-z0-9_-]+$")
MODULE_ID = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$")
SETTING_KEY = re.compile(r"^[A-Za-z_][A-Za-z0-9_]{0,63}$")
SETTING_TYPES = {"string", "integer", "number", "boolean", "enum", "path"}


def check(condition: object, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def manifest() -> dict:
    return json.loads((ROOT / "manifest.json").read_text(encoding="utf-8"))


def test_manifest_identity() -> None:
    document = manifest()
    check(document.get("schemaVersion") == 1, "schemaVersion must be 1")
    check(PLUGIN_ID.match(str(document.get("id", ""))), "id must be publisher.name in lowercase")
    check(str(document.get("name", "")).strip(), "name is required")
    check(document.get("version") == "0.1.0", "stay at 0.1.0 until the first release")
    check("service" in document.get("kinds", []), "kinds must include service")
    service = document.get("entryPoints", {}).get("service", "")
    check(service and (ROOT / service).is_file(), "the service entry point must exist")


def test_blade_modules() -> None:
    modules = manifest().get("extensions", {}).get(SOCKET)
    check(isinstance(modules, list) and 1 <= len(modules) <= 128, "declare at least one blade module")
    for module in modules:
        check(MODULE_ID.match(str(module.get("id", ""))), "module id is invalid")
        entry = str(module.get("entry", "Module.qml"))
        check(not entry.startswith("/") and ".." not in entry.split("/"), "entry must be a safe relative path")
        check((ROOT / entry).is_file(), f"entry {entry} must exist")
        check(module.get("hostContract", 1) in (1, 2), "hostContract must be 1 or 2")
        check(0 <= int(module.get("minHeight", 0)) <= 4096, "minHeight must be 0 to 4096")
        settings = module.get("settings")
        if settings is None:
            continue
        schema = settings.get("schema", [])
        keys = [row.get("key") for row in schema]
        check(len(keys) == len(set(keys)) and len(keys) <= 32, "setting keys must be unique and at most 32")
        for row in schema:
            check(SETTING_KEY.match(str(row.get("key", ""))), "setting key is invalid")
            check(row.get("type") in SETTING_TYPES, f"unknown setting type {row.get('type')}")
        for key in settings.get("defaults", {}):
            check(key in keys, f"default {key} has no schema row")


def test_host_guard_targets_fileblade() -> None:
    source = (ROOT / "HostGuard.js").read_text(encoding="utf-8")
    check(f'var HOST_ID = "{HOST_ID}"' in source, "HostGuard.js must name the FileBlade host")
    check("https://github.com/data-goblin/fileblade.git" in source, "HostGuard.js must install FileBlade from its repository")
    check((ROOT / "assets" / "fileblade-logo.png").is_file(), "the host guard needs assets/fileblade-logo.png")


def test_banner_generator() -> None:
    name = str(manifest()["name"])
    name = name[len("FileBlade "):] if name.startswith("FileBlade ") else name
    with tempfile.TemporaryDirectory() as directory:
        completed = subprocess.run(
            [sys.executable, str(ROOT / "scripts" / "fileblade-extension-image.py"), "--manifest", str(ROOT / "manifest.json"), "--out", directory],
            capture_output=True,
            text=True,
            timeout=60,
            check=False,
        )
        check(completed.returncode == 0, completed.stderr)
        svg = (Path(directory) / "fileblade-extension-logo.svg").read_text(encoding="utf-8")
    check('viewBox="0 0 960 272"' in svg, "banner viewBox")
    check('id="fileblade-extension-logo-mask"' in svg, "banner mask id")
    check('fill="#e0af68"' in svg, "banner subtitle colour")
    check(f'aria-label="FileBlade {name.lower()} extension"' in svg, "banner label")


def main() -> int:
    tests = [value for key, value in sorted(globals().items()) if key.startswith("test_") and callable(value)]
    for test in tests:
        test()
    print(f"contract: {len(tests)} checks ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())
