import hashlib
import os
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

PATHSPEC_VARS = (
    "GIT_LITERAL_PATHSPECS",
    "GIT_GLOB_PATHSPECS",
    "GIT_NOGLOB_PATHSPECS",
    "GIT_ICASE_PATHSPECS",
)


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

    def check(
        self,
        root: Path,
        *args: str,
        script: Path = SCRIPT,
        env: dict[str, str] | None = None,
    ) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [str(script), *args], cwd=root, capture_output=True, text=True, env=env,
        )

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

    def test_a_non_ascii_path_does_not_hide_a_hit(self):
        root = self.repo({
            "src/café.py": "# TODO\n",
            "src/notes.py": "# FIXME\n",
        })
        done = self.check(root)
        self.assertEqual(done.returncode, 1)
        self.assertEqual(done.stdout, "src/café.py:1:# TODO\nsrc/notes.py:1:# FIXME\n")
        self.assertEqual(done.stderr, MESSAGE)

    def test_a_newline_in_a_path_does_not_hide_a_hit(self):
        root = self.repo({})
        path = root / "src" / "a\nb.py"
        path.parent.mkdir(parents=True)
        path.write_text("# TODO\n")
        subprocess.run(["git", "-C", str(root), "add", "-A"], check=True)
        done = self.check(root)
        self.assertEqual(done.returncode, 1)
        self.assertEqual(done.stdout, "src/a\nb.py:1:# TODO\n")
        self.assertEqual(done.stderr, MESSAGE)

    def test_pathspec_variables_do_not_hide_a_hit(self):
        root = self.repo({
            "src/notes.py": "# TODO\n",
            "src/Notes.PY": "# FIXME\n",
        })
        want = "src/notes.py:1:# TODO\n"
        for var in PATHSPEC_VARS:
            env = os.environ.copy()
            env[var] = "1"
            done = self.check(root, env=env)
            self.assertEqual(done.returncode, 1, var)
            self.assertEqual(done.stdout, want, var)
            self.assertEqual(done.stderr, MESSAGE, var)

    def test_pathspec_variables_do_not_hide_an_explicit_pathspec(self):
        root = self.repo({
            "docs/notes.md": "<!-- TODO -->\n",
            "src/bad.py": "# FIXME\n",
        })
        want = "docs/notes.md:1:<!-- TODO -->\n"
        for var in PATHSPEC_VARS:
            env = os.environ.copy()
            env[var] = "1"
            done = self.check(root, "*.md", env=env)
            self.assertEqual(done.returncode, 1, var)
            self.assertEqual(done.stdout, want, var)
            self.assertEqual(done.stderr, MESSAGE, var)

    def test_pathspec_variables_do_not_fail_a_tree_with_nothing_to_scan(self):
        root = self.repo({"README.md": "hello\n"})
        for var in PATHSPEC_VARS:
            env = os.environ.copy()
            env[var] = "1"
            done = self.check(root, env=env)
            self.assertEqual(done.returncode, 0, var)
            self.assertEqual(done.stdout, "", var)
            self.assertEqual(done.stderr, "", var)

    def test_slash_comments_are_findings(self):
        # The markers are split so this line is not itself a finding.
        line = "/" + "/ TODO"
        block = "/" + "* FIXME */"
        root = self.repo({"src/notes.js": line + "\n" + block + "\n"})
        done = self.check(root)
        self.assertEqual(done.returncode, 1)
        self.assertEqual(done.stdout, "src/notes.js:1:" + line + "\nsrc/notes.js:2:" + block + "\n")
        self.assertEqual(done.stderr, MESSAGE)

    def test_a_search_error_does_not_hide_a_hit(self):
        root = self.repo({"src/notes.py": "# TODO\n"})
        (root / "src" / "dangling.py").symlink_to("missing")
        subprocess.run(["git", "-C", str(root), "add", "-A"], check=True)
        done = self.check(root)
        self.assertEqual(done.returncode, 2)
        self.assertEqual(done.stdout, "src/notes.py:1:# TODO\n")
        self.assertEqual(done.stderr, "grep: src/dangling.py: No such file or directory\n")

    def test_a_search_error_on_an_empty_hit_list_is_not_a_pass(self):
        root = self.repo({})
        (root / "src").mkdir()
        (root / "src" / "dangling.py").symlink_to("missing")
        subprocess.run(["git", "-C", str(root), "add", "-A"], check=True)
        done = self.check(root)
        self.assertEqual(done.returncode, 2)
        self.assertEqual(done.stdout, "")
        self.assertEqual(done.stderr, "grep: src/dangling.py: No such file or directory\n")
