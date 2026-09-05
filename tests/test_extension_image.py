from __future__ import annotations

import importlib.util
import json
import re
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "fileblade-extension-image.py"
COMMANDS = re.compile(r"^([MLHVQCZ][-0-9. ]*)+$")


def load():
    spec = importlib.util.spec_from_file_location("fileblade_extension_image", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


def check(condition: object, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def test_glyph_table(image) -> None:
    check(set(image.GLYPHS) == set("ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 -.&+/"), "alphabet")
    for char, path in image.GLYPHS.items():
        check(path == "" or COMMANDS.match(path), f"glyph {char!r} is not an absolute SVG path")
    check(image.GLYPHS[" "] == "", "space has no outline")
    check(image.MAX_SUBTITLE == 23, "23 characters fit under the blade")


def test_geometry(image) -> None:
    check(image.subtitle_text("Agent Skills") == "AGENT SKILLS EXTENSION", "subtitle text")
    check(image.subtitle_text("  git  ") == "GIT EXTENSION", "subtitle whitespace")
    check(image.subtitle_x("GIT EXTENSION") == 506.39, "git banner x")
    check(image.subtitle_x("AGENT SKILLS EXTENSION") == 290.39, "skills banner x")
    check(image.shifted("M1.0 2.0 3.0 4.0H5.0V6.0Q7.0 8.0 9.0 10.0Z", 10) == "M11.0 2.0 13.0 4.0H15.0V6.0Q17.0 8.0 19.0 10.0Z", "x shift")
    check(image.number(-0.0) == "0.0" and image.number(70.65) == "70.65" and image.number(3) == "3.0", "number format")
    first = image.subtitle_path("AA")
    single = image.subtitle_path("A")
    check(first.startswith(single + " ") and "M" + image.number(60 + float(re.findall(r"M(-?[0-9.]+)", single)[0])) in first, "second glyph advances 60")


def test_banner(image) -> None:
    svg = image.banner_svg("Weather")
    check(svg.startswith('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 960 272" width="960" height="272"'), "banner header")
    check('aria-label="FileBlade weather extension"' in svg, "banner label")
    check(svg.count('id="fileblade-extension-logo-mask"') == 1 and "fileblade-logo-mask" not in svg.replace("fileblade-extension-logo-mask", ""), "banner mask id")
    check('translate(410.39,243.20) scale(0.4)' in svg and 'fill="#e0af68"' in svg, "banner subtitle placement")
    check(svg.count('fill="#7aa2f7"') == 3, "logo colour")
    check(svg.endswith("</svg>\n"), "banner end")
    custom = image.banner_svg("Weather", "RAIN & SUN")
    check('translate(578.39,243.20)' in custom, "custom subtitle placement")
    logo = image.host_logo_svg()
    check('viewBox="0 0 960 240"' in logo and 'id="fileblade-logo-mask"' in logo and "#e0af68" not in logo, "host logo")
    label = image.banner_svg('We "ather" <&>', "WEATHER EXTENSION")
    check('aria-label="FileBlade we &quot;ather&quot; &lt;&amp;&gt; extension"' in label, "label escaping")


def test_refusals(image) -> None:
    for text_value in ["ÜBER EXTENSION", "", "   ", "A" * 24]:
        try:
            image.subtitle_path(text_value)
        except ValueError:
            continue
        raise AssertionError(f"{text_value!r} must be refused")


def test_command_line() -> None:
    with tempfile.TemporaryDirectory() as directory:
        manifest = Path(directory) / "manifest.json"
        manifest.write_text(json.dumps({"name": "FileBlade Agent Skills"}), encoding="utf-8")
        out = Path(directory) / "assets"
        completed = subprocess.run(
            [sys.executable, str(SCRIPT), "--manifest", str(manifest), "--out", str(out)],
            capture_output=True,
            text=True,
            timeout=60,
            check=False,
            env={"PYTHONDONTWRITEBYTECODE": "1", "PATH": ""},
        )
        check(completed.returncode == 0, completed.stderr)
        check(completed.stdout.strip() == str(out / "fileblade-extension-logo.svg"), "written path is printed")
        svg = (out / "fileblade-extension-logo.svg").read_text(encoding="utf-8")
        check('translate(290.39,243.20)' in svg, "manifest name drives the subtitle")
        png = subprocess.run(
            [sys.executable, str(SCRIPT), "--name", "Weather", "--out", str(out), "--png"],
            capture_output=True,
            text=True,
            timeout=60,
            check=False,
            env={"PYTHONDONTWRITEBYTECODE": "1", "PATH": ""},
        )
        check(png.returncode == 1 and "rsvg-convert" in png.stderr, "png without rsvg-convert is refused clearly")
        missing = subprocess.run(
            [sys.executable, str(SCRIPT), "--manifest", str(Path(directory) / "none.json"), "--out", str(out)],
            capture_output=True,
            text=True,
            timeout=60,
            check=False,
            env={"PYTHONDONTWRITEBYTECODE": "1"},
        )
        check(missing.returncode == 1 and "cannot read" in missing.stderr, "missing manifest is reported")
        stdout = subprocess.run(
            [sys.executable, str(SCRIPT), "--name", "Git", "--stdout"],
            capture_output=True,
            text=True,
            timeout=60,
            check=False,
            env={"PYTHONDONTWRITEBYTECODE": "1"},
        )
        check(stdout.returncode == 0 and stdout.stdout.startswith("<svg") and 'translate(506.39,243.20)' in stdout.stdout, "stdout mode")


def main() -> int:
    image = load()
    test_glyph_table(image)
    test_geometry(image)
    test_banner(image)
    test_refusals(image)
    test_command_line()
    print("extension image: ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())
