import io
import re
import subprocess
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path

from verify import CHECKS, Check, Finding, main, run

ROOT = Path(__file__).resolve().parents[1]
PAINT = "the page must paint in its theme from the first frame"


def page(body: str, head: str = '<script src="../theme.js"></script>') -> str:
    return f"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
{head}
</head>
<body>
{body}
</body>
</html>
"""


class SiteCase(unittest.TestCase):
    def site(self, files: dict[str, str]) -> Path:
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        root = Path(tmp.name)
        subprocess.run(["git", "init", "-q", str(root)], check=True)
        for name, text in files.items():
            (root / name).parent.mkdir(parents=True, exist_ok=True)
            (root / name).write_text(text)
        return root


class Python(SiteCase):
    def test_a_syntax_error_names_its_line(self):
        root = self.site({"scripts/bad.py": "def (:\n", "scripts/good.py": "x = 1\n"})
        self.assertEqual(run(root, ["python"])["python"], (Finding("scripts/bad.py", 1, "invalid syntax"),))


class Html(SiteCase):
    def test_a_misnested_tag_names_the_one_left_open(self):
        root = self.site({
            "site/a/index.html": page("<div><span></div>"),
            "site/b/index.html": page("<div><span></span></div>"),
        })
        self.assertEqual(run(root, ["html"])["html"], (
            Finding("site/a/index.html", 8, "</div> closes <span> opened at line 8"),
        ))

    def test_unclosed_stray_and_void_end_tags(self):
        root = self.site({"site/a/index.html": page("<p>open\n</em>\n<br></br>\n<svg><path d=\"M0 0\"/></svg>")})
        self.assertEqual(run(root, ["html"])["html"], (
            Finding("site/a/index.html", 9, "</em> has nothing to close"),
            Finding("site/a/index.html", 10, "</br> ends a void element"),
            Finding("site/a/index.html", 12, "</body> closes <p> opened at line 8"),
        ))

    def test_an_import_map_must_be_json(self):
        root = self.site({
            "site/a/index.html": page("", head='<script type="importmap">{"imports": </script>'),
            "site/b/index.html": page("", head='<script type="importmap">{"imports": {}}</script>'),
        })
        self.assertEqual(run(root, ["html"])["html"], (
            Finding("site/a/index.html", 5, 'import map is not JSON with an "imports" object of strings'),
        ))


class Links(SiteCase):
    def test_each_bad_url_is_one_finding(self):
        body = "\n".join([
            '<a href="missing.html">x</a>',
            '<a href="a">x</a>',
            '<a href="../x">x</a>',
            '<a href="/theme.js">x</a>',
            '<a href="a/index.html">x</a>',
            '<a href="http://omahoy.org/">x</a>',
            '<a href="https://www.omahoy.org/">x</a>',
            '<a href="https://docs.omahoy.org/">x</a>',
            '<a href="nope/">x</a>',
            '<a href="a/">ok</a> <a href="./">ok</a> <a href="?theme=night-watch">ok</a>',
            '<a href="https://github.com/">ok</a> <a href="https://omahoy.org/a/">ok</a> <img src="a/b.webp">',
        ])
        root = self.site({
            "site/index.html": page(body, head='<script src="theme.js"></script>'),
            "site/theme.js": "", "site/a/index.html": page(""), "site/a/b.webp": "",
        })
        self.assertEqual(run(root, ["links"])["links"], (
            Finding("site/index.html", 8, "missing.html: names nothing under site/"),
            Finding("site/index.html", 9, "a: directory link needs a trailing slash"),
            Finding("site/index.html", 10, "../x: leaves site/"),
            Finding("site/index.html", 11, "/theme.js: root-relative; use a relative URL so the file:// review "
                                           "in site/README.md loads it"),
            Finding("site/index.html", 12, "a/index.html: names index.html; link its directory with a trailing slash"),
            Finding("site/index.html", 13, "http://omahoy.org/: spell the site's own URLs from https://omahoy.org/"),
            Finding("site/index.html", 14, "https://www.omahoy.org/: spell the site's own URLs from https://omahoy.org/"),
            Finding("site/index.html", 15, "https://docs.omahoy.org/: spell the site's own URLs from https://omahoy.org/"),
            Finding("site/index.html", 16, "nope/: no index.html in site/nope/"),
        ))

    def test_og_urls_are_absolute(self):
        root = self.site({
            "site/og.png": "",
            "site/a/index.html": page("", head='<meta property="og:image" content="../og.png">'),
            "site/b/index.html": page("", head='<meta property="og:image" content="https://omahoy.org/og.png">'),
        })
        self.assertEqual(run(root, ["links"])["links"], (
            Finding("site/a/index.html", 5, "../og.png: og: URLs are absolute https://omahoy.org/ URLs"),
        ))

    def test_import_map_prefixes_name_directories_and_inline_imports_name_files(self):
        imports = '{"imports":{"three":"../vendor/three.js","three/addons/":"../vendor/addons/","gone/":"../vendor/gone/"}}'
        body = "\n".join([
            "<script type=\"module\">import('./viewer.js');</script>",
            "<script type=\"module\">import(\"./missing.js\");</script>",
            '<a href="../vendor/addons/">x</a>',
        ])
        root = self.site({
            "site/t/index.html": page(body, head=f'<script type="importmap">{imports}</script>'),
            "site/t/viewer.js": "", "site/vendor/three.js": "", "site/vendor/addons/controls/orbit.js": "",
        })
        self.assertEqual(run(root, ["links"])["links"], (
            Finding("site/t/index.html", 5, "../vendor/gone/: no file under site/vendor/gone/"),
            Finding("site/t/index.html", 9, "./missing.js: names nothing under site/"),
            Finding("site/t/index.html", 10, "../vendor/addons/: no index.html in site/vendor/addons/"),
        ))


class Posters(SiteCase):
    def test_poster_shares_the_clip_name(self):
        root = self.site({
            "site/a/index.html": page('<video src="../media/a.mp4" poster="../media/b.webp"></video>\n'
                                      '<video src="../media/a.mp4"></video>'),
            "site/b/index.html": page('<video src="../media/a.mp4" poster="../media/a.webp"></video>'),
            "site/media/a.mp4": "", "site/media/a.webp": "", "site/media/b.webp": "",
        })
        self.assertEqual(run(root, ["posters"])["posters"], (
            Finding("site/a/index.html", 8, "poster ../media/b.webp should be ../media/a.webp"),
            Finding("site/a/index.html", 9, "video has no poster"),
        ))


SVG = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 2 1"><path d="M0 0h1v1h-1z"/></svg>\n'


def inline(d: str, view: str = "0 0 2 1") -> str:
    return f'<svg class="wordmark" viewBox="{view}" role="img"><path fill="currentColor" d="{d}"/></svg>'


class Wordmark(SiteCase):
    def test_the_inline_copy_is_the_file_and_the_only_copy(self):
        home = '<script src="theme.js"></script>'
        drifted = self.site({
            "site/wordmark.svg": SVG,
            "site/index.html": page(inline("M1 0h1v1h-1z"), head=home),
            "site/a/index.html": page(inline("M0 0h1v1h-1z")),
        })
        reframed = self.site({"site/wordmark.svg": SVG, "site/index.html": page(inline("M0 0h1v1h-1z", "0 0 3 1"), head=home)})
        same = self.site({"site/wordmark.svg": SVG, "site/index.html": page(inline("M0 0h1v1h-1z"), head=home)})
        differs = "inline wordmark differs from site/wordmark.svg; paste its viewBox and d"
        self.assertEqual(run(drifted, ["wordmark"])["wordmark"], (
            Finding("site/index.html", 8, differs),
            Finding("site/a/index.html", 8, "the wordmark is inlined only in site/index.html"),
        ))
        self.assertEqual(run(reframed, ["wordmark"])["wordmark"], (Finding("site/index.html", 8, differs),))
        self.assertEqual(run(same, ["wordmark"])["wordmark"], ())


class Theme(SiteCase):
    def test_theme_js_is_a_synchronous_classic_script_in_head(self):
        root = self.site({
            "site/theme.js": "",
            "site/a/index.html": page("", head='<script src="../theme.js" defer></script>'),
            "site/b/index.html": page('<script src="../theme.js"></script>', head=""),
            "site/c/index.html": page("", head='<script type="module" src="../theme.js"></script>'),
            "site/d/index.html": page(""),
            "site/e/index.html": page("", head=""),
        })
        self.assertEqual(run(root, ["theme"])["theme"], (
            Finding("site/a/index.html", 5, f"../theme.js has defer; {PAINT}"),
            Finding("site/b/index.html", 8, f"../theme.js loads outside <head>; {PAINT}"),
            Finding("site/c/index.html", 5, f'../theme.js has type="module"; {PAINT}'),
            Finding("site/e/index.html", None, f"theme.js is never loaded; {PAINT}"),
        ))


def apps_nav(*links: str) -> str:
    return '<nav aria-label="Apps">' + "".join(links) + "</nav>"


class Nav(SiteCase):
    def test_every_nav_lists_every_app_page_in_one_order(self):
        root = self.site({
            "site/index.html": page('<a href="a/">a</a>', head='<script src="theme.js"></script>'),
            "site/a/index.html": page(apps_nav('<a href="./" aria-current="page">a</a>', '<a href="../b/">b</a>',
                                               '<a href="../c/">c</a>')),
            "site/b/index.html": page(apps_nav('<a href="../a/">a</a>', '<a href="../b/" aria-current="page">b</a>')),
            "site/c/index.html": page(apps_nav('<a href="../a/">a</a>', '<a href="../c/" aria-current="page">c</a>',
                                               '<a href="../b/">b</a>')),
        })
        self.assertEqual(run(root, ["nav"])["nav"], (
            Finding("site/b/index.html", 8, "nav is missing ../c/"),
            Finding("site/c/index.html", 8, "nav order differs from site/a/index.html"),
        ))

    def test_aria_current_marks_this_page(self):
        root = self.site({
            "site/a/index.html": page(apps_nav('<a href="../a/">a</a>', '<a href="../b/" aria-current="page">b</a>')),
            "site/b/index.html": page(apps_nav('<a href="../a/">a</a>', '<a href="./" aria-current="page">b</a>')),
        })
        self.assertEqual(run(root, ["nav"])["nav"], (
            Finding("site/a/index.html", 8, '../a/ is this page; mark it aria-current="page"'),
            Finding("site/a/index.html", 8, 'aria-current="page" is on ../b/, not this page'),
        ))


VIEWER = "site/omatiller/viewer.js"


class Model(SiteCase):
    def test_the_named_model_files_exist(self):
        whole = self.site({VIEWER: "const MODEL = './model/m';\n",
                           "site/omatiller/model/m.glb": "", "site/omatiller/model/m.json": ""})
        half = self.site({VIEWER: 'import * as THREE from "three";\nconst MODEL = "./model/m";\n',
                          "site/omatiller/model/m.glb": ""})
        self.assertEqual(run(whole, ["model"])["model"], ())
        self.assertEqual(run(half, ["model"])["model"], (
            Finding(VIEWER, 2, "./model/m.json: names nothing under site/"),
        ))

    def test_model_is_bound_once_to_a_literal(self):
        files = {"site/omatiller/model/m.glb": "", "site/omatiller/model/m.json": ""}
        twice = self.site({VIEWER: "const MODEL = './model/m';\nlet MODEL = './model/m';\n", **files})
        computed = self.site({VIEWER: "const MODEL = base + '/m';\n", **files})
        self.assertEqual(run(twice, ["model"])["model"], (
            Finding(VIEWER, 2, "MODEL is bound again; keep the one at line 1"),
        ))
        self.assertEqual(run(computed, ["model"])["model"], (
            Finding(VIEWER, 1, "MODEL is base + '/m', not a string literal"),
        ))


FEATURES = ".cursor/skills/verify/features/"
FOUR = ("Sub-features", "How to get to it (user POV)", "Driving it with headless Chromium", "Gotchas")


def feature(*headings: str) -> str:
    return "# A feature\n\nWhat a visitor sees.\n\n" + "".join(f"## {heading}\n\nText.\n\n" for heading in headings)


class FeatureMap(SiteCase):
    def test_the_index_links_each_feature_and_each_has_four_headings(self):
        root = self.site({
            FEATURES + "README.md": "# Map\n\n- [Home](./home.md)\n- [Theme](./theme.md)\n- [Gone](./gone.md)\n",
            FEATURES + "home.md": feature(*FOUR),
            FEATURES + "theme.md": feature("Gotchas", *FOUR[:3]),
            FEATURES + "x.md": feature(*FOUR),
        })
        self.assertEqual(run(root, ["feature-map"])["feature-map"], (
            Finding(FEATURES + "README.md", 5, "./gone.md: no such feature file"),
            Finding(FEATURES + "README.md", None, "does not link ./x.md"),
            Finding(FEATURES + "theme.md", 5, "## headings are Gotchas, Sub-features, How to get to it (user POV), "
                                              "Driving it with headless Chromium; want Sub-features, "
                                              "How to get to it (user POV), Driving it with <harness>, Gotchas"),
        ))


class Clean(SiteCase):
    def test_a_check_that_writes_fails_clean(self):
        def writes(repo):
            (repo.root / "site/x.txt").write_text("")
            return ()
        checks = {"writes": Check("writes", "writes a file", writes), "reads": Check("reads", "reads", lambda repo: ())}
        dirty = self.site({"site/old.txt": ""})
        tidy = self.site({"site/old.txt": ""})
        self.assertEqual(run(dirty, ["writes"], checks=checks), {
            "writes": (), "clean": (Finding("site/x.txt", None, "created or changed by verification"),),
        })
        self.assertEqual(run(tidy, ["reads"], checks=checks), {"reads": (), "clean": ()})


class Run(SiteCase):
    def test_a_crash_is_one_finding_and_the_other_checks_still_run(self):
        def crashes(repo):
            raise ValueError("boom")
        checks = {"crashes": Check("crashes", "", crashes), "reads": Check("reads", "", lambda repo: ())}
        report = run(self.site({}), checks=checks)
        crash = report["crashes"][0]
        self.assertEqual(list(report), ["crashes", "reads", "clean"])
        self.assertEqual((crash.path, crash.line, crash.message.splitlines()[-1]),
                         ("scripts/verify.py", None, "ValueError: boom"))


class Cli(unittest.TestCase):
    def test_bad_arguments_exit_2_and_run_nothing(self):
        usage = "usage: scripts/verify.sh [--list | CHECK...]"
        for argv, error in ((["nope"], "unknown check nope"), (["--skip"], "unknown option --skip"),
                            (["clean"], "unknown check clean"), (["links", "--list"], "unknown option --list")):
            out, err = io.StringIO(), io.StringIO()
            with redirect_stdout(out), redirect_stderr(err):
                code = main(argv)
            self.assertEqual((code, out.getvalue(), err.getvalue()), (2, "", f"{error}\n{usage}\n"), argv)


class Coverage(unittest.TestCase):
    def test_every_check_has_a_test_class(self):
        classes = {name for name, value in globals().items()
                   if isinstance(value, type) and issubclass(value, unittest.TestCase)}
        wanted = {name.title().replace("-", "") for name in CHECKS} - {"Comments", "Tests"}
        self.assertEqual(sorted(wanted - classes), [])


class Surface(unittest.TestCase):
    def test_the_shell_and_ci_list_no_checks(self):
        def listed(text: str) -> list[str]:
            names = [name for name in CHECKS if re.search(rf"\b{re.escape(name)}\b", text, re.IGNORECASE)]
            return names + (["VERIFY_SKIP"] if "VERIFY_SKIP" in text else [])
        self.assertEqual(listed("VERIFY_SKIP=1 scripts/verify.sh links feature-map"),
                         ["links", "feature-map", "VERIFY_SKIP"])
        for path in ("scripts/verify.sh", ".github/workflows/site.yml"):
            self.assertEqual(listed((ROOT / path).read_text()), [], path)


if __name__ == "__main__":
    unittest.main()
