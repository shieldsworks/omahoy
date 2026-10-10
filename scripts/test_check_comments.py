import hashlib
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "check-comments.sh"

PHRASES = (
    "# TODO",
    "# FIXME",
    "# XXX",
    "# HACK",
    "# workaround",
    "# Workaround",
    "# temporary fix",
    "# temporary hack",
    "# temporary solution",
    "# quick fix",
    "# for the time being",
    "# not ideal",
    "# should be fixed",
    "# sorry",
    "# kludge",
    "# band-aid",
    "# bandaid",
)

CLEAN = (
    "# for now",
    "# temporary file",
    "# workarounds",
    "# bandaids",
    "# TODOs",
    "TODO = 1",
)

MESSAGE = "\nApologetic or deferred-work comments are not allowed (see AGENTS.md).\n"


class Comments(unittest.TestCase):
    def repo(self, files: dict[str, str]) -> Path:
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        root = Path(tmp.name)
        subprocess.run(["git", "init", "-q", str(root)], check=True)
        for name, text in files.items():
            path = root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text)
        subprocess.run(["git", "-C", str(root), "add", "-A"], check=True)
        return root

    def check(self, root: Path, *args: str, script: Path = SCRIPT) -> subprocess.CompletedProcess[str]:
        return subprocess.run([str(script), *args], cwd=root, capture_output=True, text=True)

    def planted(self, root: Path) -> Path:
        dest = root / "tools" / "check-comments.sh"
        dest.parent.mkdir(parents=True)
        dest.write_text(SCRIPT.read_text() + "# TODO planted\n")
        dest.chmod(0o755)
        subprocess.run(["git", "-C", str(root), "add", "-A"], check=True)
        return dest

    def test_each_banned_phrase_is_a_finding(self):
        text = "\n".join(PHRASES) + "\n"
        root = self.repo({"src/notes.py": text})
        done = self.check(root)
        want = "".join(f"src/notes.py:{number}:{line}\n" for number, line in enumerate(PHRASES, 1))
        self.assertEqual(done.returncode, 1)
        self.assertEqual(done.stdout, want)
        self.assertEqual(done.stderr, MESSAGE)

    def test_allowed_phrases_and_non_comments_pass(self):
        root = self.repo({"src/notes.py": "\n".join(CLEAN) + "\n"})
        done = self.check(root)
        self.assertEqual(done.returncode, 0)
        self.assertEqual(done.stdout, "")
        self.assertEqual(done.stderr, "")

    def test_fixtures_and_nested_vendor_are_skipped(self):
        root = self.repo({
            "tests/fixtures/bad.py": "# TODO\n",
            "site/vendor/bad.js": "# TODO\n",
            "src/notes.py": "# for now\n",
        })
        done = self.check(root)
        self.assertEqual(done.returncode, 0)
        self.assertEqual(done.stdout, "")

    def test_a_top_level_vendor_directory_is_scanned(self):
        root = self.repo({"vendor/bad.js": "# TODO\n"})
        done = self.check(root)
        self.assertEqual(done.returncode, 1)
        self.assertEqual(done.stdout, "vendor/bad.js:1:# TODO\n")

    def test_the_agents_pin_matches_the_script(self):
        digest = hashlib.sha256(SCRIPT.read_bytes()).hexdigest()
        line = f"{digest}  scripts/check-comments.sh"
        self.assertIn(line, (ROOT / "AGENTS.md").read_text().splitlines())

    def test_an_empty_tree_passes(self):
        root = self.repo({})
        done = self.check(root)
        self.assertEqual(done.returncode, 0)
        self.assertEqual(done.stdout, "")
        self.assertEqual(done.stderr, "")

    def test_pathspecs_replace_the_default_set(self):
        root = self.repo({
            "notes.md": "<!-- TODO -->\n",
            "src/bad.py": "# FIXME\n",
        })
        done = self.check(root, "*.md")
        self.assertEqual(done.returncode, 1)
        self.assertEqual(done.stdout, "notes.md:1:<!-- TODO -->\n")
        self.assertEqual(done.stderr, MESSAGE)

    def test_the_script_excludes_itself_at_any_path(self):
        root = self.repo({})
        done = self.check(root, script=self.planted(root))
        self.assertEqual(done.returncode, 0)
        self.assertEqual(done.stdout, "")
        self.assertEqual(done.stderr, "")

    def test_a_hit_is_reported_when_the_script_lives_in_the_repo(self):
        root = self.repo({"src/bad.py": "# FIXME\n"})
        done = self.check(root, script=self.planted(root))
        self.assertEqual(done.returncode, 1)
        self.assertEqual(done.stdout, "src/bad.py:1:# FIXME\n")
