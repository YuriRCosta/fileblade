import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile


def snapshot(directory):
    return {str(path.relative_to(directory)): path.read_bytes() for path in directory.rglob("*") if path.is_file()}


def check(repository, package, launcher, installed):
    core = Path(__file__).resolve().parents[1]
    with tempfile.TemporaryDirectory(prefix="fileblade-readonly-") as temporary:
        base = Path(temporary)
        plugins = base / "plugins"
        identifier = json.loads((repository / "manifest.json").read_text())["id"]
        provider = plugins / (identifier if installed else repository.name)
        dependency = plugins / ("data-goblin.fileblade" if installed else "fileblade")
        shutil.copytree(repository / package, provider / package, ignore=shutil.ignore_patterns("__pycache__"))
        shutil.copytree(core / "python", dependency / "python", ignore=shutil.ignore_patterns("__pycache__"))
        (provider / "bin").mkdir()
        shutil.copy2(repository / "bin" / launcher, provider / "bin" / launcher)
        before = snapshot(plugins)
        result = subprocess.run([str(provider / "bin" / launcher), "--help"],
                                env={"PATH": os.environ["PATH"], "HOME": str(base / "home")},
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
        assert result.returncode == 0, result.stderr.decode("utf-8", "replace")
        assert snapshot(plugins) == before, "helper imports modified the plugin tree (including bytecode caches)"


if __name__ == "__main__":
    repository = Path(sys.argv[1]).resolve()
    for installed in (False, True):
        check(repository, sys.argv[2], sys.argv[3], installed)
    print("read-only helper imports: ok (development and installed layouts)")
