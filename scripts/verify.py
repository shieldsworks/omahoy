#!/usr/bin/env python3
from __future__ import annotations

import json
import posixpath
import re
import subprocess
import sys
import traceback
from dataclasses import dataclass
from functools import cached_property
from html.parser import HTMLParser
from pathlib import Path, PurePosixPath
from typing import Callable, Iterable, Iterator, Mapping, Sequence
from urllib.parse import unquote, urlsplit


@dataclass(frozen=True)
class Finding:
    """One violation. path is repo-relative, e.g. "site/omahelm/index.html".
    line is 1-based, or None when the finding is about the whole file."""
    path: str
    line: int | None
    message: str


Report = dict[str, tuple[Finding, ...]]
"""Check name to its findings, in run order, always ending with "clean".
Verified means every value is empty."""


CheckFn = Callable[["Repo"], Iterable[Finding]]


@dataclass(frozen=True)
class Check:
    name: str
    guards: str
    run: CheckFn


CHECKS: dict[str, Check] = {}

CLEAN = "verification created or changed no file (always runs, last)"
USAGE = "usage: scripts/verify.sh [--list | CHECK...]"


def check(guards: str) -> Callable[[CheckFn], CheckFn]:
    def register(fn: CheckFn) -> CheckFn:
        name = fn.__name__.replace("_", "-")
        if name in CHECKS or name == "clean":
            raise ValueError(f"a check named {name} already exists")
        CHECKS[name] = Check(name, guards, fn)
        return fn
    return register


@dataclass(eq=False)
class Element:
    tag: str
    attrs: dict[str, str]          # html.parser lowercases names, so viewBox arrives as viewbox
    line: int
    parent: Element | None
    text: str = ""

    def matches(self, tag: str, **attrs: str) -> bool:
        return self.tag == tag and all(
            value in self.attrs.get(name, "").split() if name == "class" else self.attrs.get(name) == value
            for name, value in attrs.items())

    def within(self, tag: str, **attrs: str) -> Element | None:
        node = self.parent
        while node is not None and not node.matches(tag, **attrs):
            node = node.parent
        return node


@dataclass(frozen=True)
class File:
    path: PurePosixPath


@dataclass(frozen=True)
class Directory:
    path: PurePosixPath


@dataclass(frozen=True)
class External:
    url: str


@dataclass(frozen=True)
class Broken:
    reason: str


Target = File | Directory | External | Broken


@dataclass(frozen=True)
class Ref:
    element: Element
    attr: str
    raw: str
    target: Target


@dataclass(frozen=True)
class Document:
    path: PurePosixPath
    elements: tuple[Element, ...]
    refs: tuple[Ref, ...]
    problems: tuple[Finding, ...]


class Repo:
    def __init__(self, root: Path) -> None:
        self.root = root
        self._documents: dict[PurePosixPath, Document] = {}

    def ls(self, *pathspecs: str) -> tuple[PurePosixPath, ...]:
        def files(*flags: str) -> set[str]:
            return set(_git(self.root, "ls-files", "-z", *flags, "--", *pathspecs).split("\0")) - {""}
        kept = files("--cached", "--others", "--exclude-standard") - files("--deleted")
        return tuple(sorted(map(PurePosixPath, kept)))

    @cached_property
    def site_files(self) -> frozenset[PurePosixPath]:
        return frozenset(self.ls("site/"))

    @cached_property
    def pages(self) -> tuple[Document, ...]:
        return tuple(self.document(path) for path in sorted(self.site_files)
                     if path.suffix == ".html" and not path.is_relative_to("site/vendor"))

    def document(self, path: PurePosixPath | str) -> Document:
        path = PurePosixPath(path)
        if path not in self._documents:
            text = (self.root / path).read_text(encoding="utf-8")
            self._documents[path] = _parse(path, text, self.site_files)
        return self._documents[path]


_VOID = frozenset({"area", "base", "br", "col", "embed", "hr", "img", "input",
                   "link", "meta", "source", "track", "wbr"})
_URL_ATTRS = frozenset({"href", "src", "poster"})
_URL_META = frozenset({"og:image", "og:url", "twitter:image"})
_ORIGIN = "https://omahoy.org/"
_DYNAMIC_IMPORT = re.compile(r"""\bimport\(\s*(['"])(.+?)\1\s*\)""")
_APPS_NAV = {"aria-label": "Apps"}
_WORDMARK = {"class": "wordmark"}


class _Walker(HTMLParser):
    def __init__(self, path: PurePosixPath) -> None:
        super().__init__(convert_charrefs=True)
        self.path = str(path)
        self.elements: list[Element] = []
        self.open: list[Element] = []
        self.problems: list[Finding] = []

    def problem(self, line: int, message: str) -> None:
        self.problems.append(Finding(self.path, line, message))

    def add(self, tag: str, attrs: list[tuple[str, str | None]]) -> Element:
        values: dict[str, str] = {}
        for name, value in attrs:
            values.setdefault(name, value or "")
        element = Element(tag, values, self.getpos()[0], self.open[-1] if self.open else None)
        self.elements.append(element)
        return element

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        element = self.add(tag, attrs)
        if tag not in _VOID:
            self.open.append(element)

    def handle_startendtag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        self.add(tag, attrs)

    def handle_endtag(self, tag: str) -> None:
        line = self.getpos()[0]
        if tag in _VOID:
            self.problem(line, f"</{tag}> ends a void element")
            return
        for depth in range(len(self.open) - 1, -1, -1):
            if self.open[depth].tag == tag:
                for skipped in self.open[depth + 1:]:
                    self.problem(line, f"</{tag}> closes <{skipped.tag}> opened at line {skipped.line}")
                del self.open[depth:]
                return
        self.problem(line, f"</{tag}> has nothing to close")

    def handle_data(self, data: str) -> None:
        if self.open and self.open[-1].tag == "script":
            self.open[-1].text += data


def _parse(path: PurePosixPath, text: str, files: frozenset[PurePosixPath]) -> Document:
    walker = _Walker(path)
    walker.feed(text)
    walker.close()
    for element in walker.open:
        walker.problem(element.line, f"<{element.tag}> is never closed")
    refs: list[Ref] = []
    for element in walker.elements:
        urls = [(name, value) for name, value in element.attrs.items() if name in _URL_ATTRS]
        if element.tag == "meta" and "content" in element.attrs and (
                element.attrs.get("property") in _URL_META or element.attrs.get("name") in _URL_META):
            urls.append(("content", element.attrs["content"]))
        if element.tag == "script" and "src" not in element.attrs:
            if element.attrs.get("type") == "importmap":
                imports = _import_map(element.text)
                if imports is None:
                    walker.problem(element.line, 'import map is not JSON with an "imports" object of strings')
                urls += [("importmap", value) for value in imports or ()]
            else:
                urls += [("import", match[2]) for match in _DYNAMIC_IMPORT.finditer(element.text)]
        refs += [Ref(element, attr, raw, _resolve(path, raw, attr, files)) for attr, raw in urls]
    problems = sorted(walker.problems, key=lambda finding: finding.line or 0)
    return Document(path, tuple(walker.elements), tuple(refs), tuple(problems))


def _import_map(text: str) -> list[str] | None:
    try:
        imports = json.loads(text)["imports"]
    except (ValueError, KeyError, TypeError):
        return None
    if not isinstance(imports, dict) or not all(isinstance(value, str) for value in imports.values()):
        return None
    return list(imports.values())


def _resolve(page: PurePosixPath, raw: str, attr: str, files: frozenset[PurePosixPath]) -> Target:
    url = urlsplit(raw)
    host = url.hostname or ""
    if host == "omahoy.org" or host.endswith(".omahoy.org"):
        if not raw.startswith(_ORIGIN):
            return Broken(f"spell the site's own URLs from {_ORIGIN}")
        base, path = PurePosixPath("site"), url.path[1:]
    elif attr == "content":
        return Broken(f"og: URLs are absolute {_ORIGIN} URLs")
    elif url.scheme or url.netloc:
        return External(raw)
    elif raw.startswith("/"):
        return Broken("root-relative; use a relative URL so the file:// review in site/README.md loads it")
    elif not url.path:
        return File(page)
    else:
        base, path = page.parent, url.path
    target = PurePosixPath(posixpath.normpath(base / unquote(path)))
    if target.parts[:1] != ("site",):
        return Broken("leaves site/")
    if target.name == "index.html":
        return Broken("names index.html; link its directory with a trailing slash")
    if (not path or path.endswith("/")) and attr == "importmap":
        return Directory(target) if any(target in file.parents for file in files) else Broken(f"no file under {target}/")
    if not path or path.endswith("/"):
        index = target / "index.html"
        return File(index) if index in files else Broken(f"no index.html in {target}/")
    if target in files:
        return File(target)
    if target / "index.html" in files:
        return Broken("directory link needs a trailing slash")
    return Broken("names nothing under site/")


def _git(root: Path, *args: str) -> str:
    return subprocess.run(["git", *args], cwd=root, capture_output=True, text=True, check=True).stdout


_LOCATED = re.compile(r"^([^\s:]+):(\d+):(.*)$")


def _script(repo: Repo, owner: str, *argv: str) -> Iterator[Finding]:
    done = subprocess.run(argv, cwd=repo.root, capture_output=True, text=True)
    if done.returncode == 0:
        return
    output = done.stdout.splitlines() + done.stderr.splitlines()
    located = [match for match in map(_LOCATED.match, output) if match]
    if located:
        yield from (Finding(match[1], int(match[2]), match[3].strip()) for match in located)
    else:
        yield Finding(owner, None, "\n".join(output[-40:]) or f"exited {done.returncode}")


@check("every .py file in the tree compiles")
def python(repo: Repo) -> Iterator[Finding]:
    # compile() in memory. py_compile writes __pycache__/ beside the source, and that
    # directory is gitignored, so the clean check would not see it.
    for path in repo.ls("*.py"):
        try:
            compile((repo.root / path).read_bytes(), str(path), "exec", dont_inherit=True)
        except SyntaxError as error:
            yield Finding(str(path), error.lineno, error.msg)


@check("scripts/test_verify.py passes")
def tests(repo: Repo) -> Iterator[Finding]:
    # -B keeps importing verify.py from writing scripts/__pycache__/.
    return _script(repo, "scripts/test_verify.py",
                   sys.executable, "-B", "-m", "unittest", "discover", "-s", "scripts", "-p", "test_*.py")


@check("no apologetic or deferred-work comments (scripts/check-comments.sh)")
def comments(repo: Repo) -> Iterator[Finding]:
    return _script(repo, "scripts/check-comments.sh", str(repo.root / "scripts/check-comments.sh"))


@check("every page's tags open and close, so it can be walked")
def html(repo: Repo) -> Iterator[Finding]:
    for page in repo.pages:
        yield from page.problems


@check("every local href, src, poster, og: URL, import-map entry and import() names a file under site/, "
       "directories with a trailing slash")
def links(repo: Repo) -> Iterator[Finding]:
    for page in repo.pages:
        for ref in page.refs:
            if isinstance(ref.target, Broken):
                yield Finding(str(page.path), ref.element.line, f"{ref.raw}: {ref.target.reason}")


@check("each <video> has a .webp poster named after its clip")
def posters(repo: Repo) -> Iterator[Finding]:
    for page in repo.pages:
        refs = {(ref.element, ref.attr): ref for ref in page.refs}
        for video in (element for element in page.elements if element.tag == "video"):
            src, poster = refs.get((video, "src")), refs.get((video, "poster"))
            if poster is None:
                yield Finding(str(page.path), video.line, "video has no poster")
            elif (src and isinstance(src.target, File) and isinstance(poster.target, File)
                  and poster.target.path != src.target.path.with_suffix(".webp")):
                want = PurePosixPath(src.raw).with_suffix(".webp")
                yield Finding(str(page.path), video.line, f"poster {poster.raw} should be {want}")


_HOME = PurePosixPath("site/index.html")
_WORDMARK_SVG = PurePosixPath("site/wordmark.svg")


def _drawing(document: Document, **attrs: str) -> tuple[int, tuple[str | None, str | None]] | None:
    for svg in document.elements:
        if svg.matches("svg", **attrs):
            path = next((e for e in document.elements if e.tag == "path" and e.within("svg") is svg), None)
            if path is not None:
                return svg.line, (svg.attrs.get("viewbox"), path.attrs.get("d"))
    return None


@check("the wordmark inlined in index.html is wordmark.svg's path, and is inlined nowhere else")
def wordmark(repo: Repo) -> Iterator[Finding]:
    inline = _drawing(repo.document(_HOME), **_WORDMARK) if _HOME in repo.site_files else None
    source = _drawing(repo.document(_WORDMARK_SVG)) if _WORDMARK_SVG in repo.site_files else None
    if inline is None:
        yield Finding(str(_HOME), None, 'no <svg class="wordmark"> with a <path>')
    elif source is None:
        yield Finding(str(_WORDMARK_SVG), None, "no <svg> with a <path>")
    elif inline[1] != source[1]:
        yield Finding(str(_HOME), inline[0], f"inline wordmark differs from {_WORDMARK_SVG}; paste its viewBox and d")
    for page in repo.pages:
        if page.path != _HOME:
            for svg in page.elements:
                if svg.matches("svg", **_WORDMARK):
                    yield Finding(str(page.path), svg.line, f"the wordmark is inlined only in {_HOME}")


_THEME_JS = File(PurePosixPath("site/theme.js"))


@check("every page loads theme.js in <head> as a synchronous classic script")
def theme(repo: Repo) -> Iterator[Finding]:
    why = "the page must paint in its theme from the first frame"
    for page in repo.pages:
        loads = [ref for ref in page.refs
                 if ref.element.tag == "script" and ref.attr == "src" and ref.target == _THEME_JS]
        if not loads:
            yield Finding(str(page.path), None, f"theme.js is never loaded; {why}")
            continue
        script = loads[0].element
        kind = script.attrs.get("type", "")
        problems = [f"has {flag}" for flag in ("async", "defer") if flag in script.attrs]
        problems += [f'has type="{kind}"'] if kind.lower() not in ("", "text/javascript") else []
        problems += ["loads outside <head>"] if script.within("head") is None else []
        for problem in problems:
            yield Finding(str(page.path), script.line, f"{loads[0].raw} {problem}; {why}")
        for ref in loads[1:]:
            yield Finding(str(page.path), ref.element.line, f"{ref.raw} loads theme.js a second time")


@check("every app page's nav lists every app page, in one order, marking itself")
def nav(repo: Repo) -> Iterator[Finding]:
    apps = [page for page in repo.pages if len(page.path.parts) == 3 and page.path.name == "index.html"]
    app_files = {page.path for page in apps}
    first: tuple[str, list[PurePosixPath]] | None = None
    for page in apps:
        where = str(page.path)
        navs = [element for element in page.elements if element.matches("nav", **_APPS_NAV)]
        if not navs:
            yield Finding(where, None, 'no <nav aria-label="Apps">')
            continue
        listed: list[PurePosixPath] = []
        for ref in page.refs:
            if ref.element.tag != "a" or ref.attr != "href" or ref.element.within("nav", **_APPS_NAV) is None:
                continue
            if isinstance(ref.target, Broken):
                continue
            if not isinstance(ref.target, File) or ref.target.path not in app_files:
                yield Finding(where, ref.element.line, f"nav links {ref.raw}, not an app page")
                continue
            listed.append(ref.target.path)
            current = ref.element.attrs.get("aria-current") == "page"
            if current and ref.target.path != page.path:
                yield Finding(where, ref.element.line, f'aria-current="page" is on {ref.raw}, not this page')
            elif not current and ref.target.path == page.path:
                yield Finding(where, ref.element.line, f'{ref.raw} is this page; mark it aria-current="page"')
        for missing in sorted(app_files - set(listed)):
            relative = posixpath.relpath(missing.parent, page.path.parent)
            yield Finding(where, navs[0].line, f"nav is missing {relative}/")
        if first is None:
            first = (where, listed)
        elif [file for file in listed if file in first[1]] != [file for file in first[1] if file in listed]:
            yield Finding(where, navs[0].line, f"nav order differs from {first[0]}")


_VIEWER = PurePosixPath("site/omatiller/viewer.js")
_OMATILLER = PurePosixPath("site/omatiller/index.html")
_MODEL_BINDING = re.compile(r"\b(?:const|let|var)\s+MODEL\s*=\s*([^;\n]*)")
_STRING = re.compile(r"""(['"])([^'"\\]+)\1""")


@check("site/omatiller/viewer.js binds MODEL once, to a string literal whose .glb and .json both exist")
def model(repo: Repo) -> Iterator[Finding]:
    where = str(_VIEWER)
    if _VIEWER not in repo.site_files:
        yield Finding(where, None, f"missing; {_OMATILLER} loads it")
        return
    text = (repo.root / _VIEWER).read_text(encoding="utf-8")
    bindings = [(text.count("\n", 0, match.start()) + 1, match[1].strip())
                for match in _MODEL_BINDING.finditer(text)]
    if not bindings:
        yield Finding(where, None, "no const MODEL = './model/<name>'")
        return
    line, value = bindings[0]
    for again, _ in bindings[1:]:
        yield Finding(where, again, f"MODEL is bound again; keep the one at line {line}")
    literal = _STRING.fullmatch(value)
    if literal is None:
        yield Finding(where, line, f"MODEL is {value}, not a string literal")
        return
    for suffix in (".glb", ".json"):
        # fetch() resolves against the page that loads viewer.js, not against the module.
        target = _resolve(_OMATILLER, literal[2] + suffix, "src", repo.site_files)
        if isinstance(target, Broken):
            yield Finding(where, line, f"{literal[2]}{suffix}: {target.reason}")


_FEATURES = PurePosixPath(".cursor/skills/verify/features")
_HEADINGS = ["Sub-features", "How to get to it (user POV)", "Driving it with <harness>", "Gotchas"]
_HARNESS = "Driving it with "


def _markdown(repo: Repo, path: PurePosixPath) -> Iterator[tuple[int, str]]:
    fenced = False
    for number, line in enumerate((repo.root / path).read_text(encoding="utf-8").splitlines(), 1):
        if line.lstrip().startswith("```"):
            fenced = not fenced
        elif not fenced:
            yield number, line


@check("every feature file is linked from features/README.md and has the four headings in order")
def feature_map(repo: Repo) -> Iterator[Finding]:
    files = [path for path in repo.ls(f"{_FEATURES}/") if path.parent == _FEATURES and path.suffix == ".md"]
    index = _FEATURES / "README.md"
    features = {path.name for path in files} - {index.name}
    linked: set[str] = set()
    if index not in files:
        yield Finding(str(index), None, "missing; it indexes the feature files")
    else:
        for number, line in _markdown(repo, index):
            for name in re.findall(r"\]\(\./([^)\s#]+)", line):
                linked.add(name)
                if name not in features:
                    yield Finding(str(index), number, f"./{name}: no such feature file")
    for name in sorted(features - linked):
        yield Finding(str(index), None, f"does not link ./{name}")
    for path in files:
        if path == index:
            continue
        headings = [(number, line[3:].strip()) for number, line in _markdown(repo, path) if line.startswith("## ")]
        shape = ["Driving it with <harness>" if text.startswith(_HARNESS) and text[len(_HARNESS):].strip() else text
                 for _, text in headings]
        if shape != _HEADINGS:
            found = ", ".join(text for _, text in headings) or "none"
            yield Finding(str(path), headings[0][0] if headings else None,
                          f"## headings are {found}; want {', '.join(_HEADINGS)}")


def run(root: Path, names: Sequence[str] = (), checks: Mapping[str, Check] = CHECKS) -> Report:
    """Run the named checks (all when empty) in definition order against the checkout at root, then clean.
    An exception inside a check becomes one Finding carrying its traceback, so one crash fails
    the run without hiding the other sections. checks exists for the clean test; every other
    caller passes names only."""
    before = _status(root)
    repo = Repo(root)
    report: Report = {}
    for name, entry in checks.items():
        if names and name not in names:
            continue
        try:
            report[name] = tuple(entry.run(repo))
        except Exception:
            report[name] = (Finding("scripts/verify.py", None, traceback.format_exc().rstrip()),)
    report["clean"] = tuple(Finding(line[3:], None, "created or changed by verification")
                            for line in sorted(_status(root) - before))
    return report


def _status(root: Path) -> frozenset[str]:
    return frozenset(_git(root, "status", "--porcelain", "--untracked-files=all").splitlines())


def main(argv: Sequence[str]) -> int:
    """The CLI boundary. Exit 0 verified, 1 findings, 2 bad arguments or no git work tree, with nothing run."""
    root = Path(__file__).resolve().parents[1]
    if list(argv) == ["--list"]:
        width = max(map(len, [*CHECKS, "clean"]))
        for entry in CHECKS.values():
            print(f"{entry.name:<{width}}  {entry.guards}")
        print(f"{'clean':<{width}}  {CLEAN}")
        return 0
    wrong = [f"unknown option {arg}" if arg.startswith("-") else f"unknown check {arg}"
             for arg in argv if arg.startswith("-") or arg not in CHECKS]
    if not wrong:
        inside = subprocess.run(["git", "rev-parse", "--is-inside-work-tree"], cwd=root, capture_output=True, text=True)
        wrong = [] if inside.stdout.strip() == "true" else [f"{root} is not a git work tree"]
    if wrong:
        print("\n".join([*wrong, USAGE]), file=sys.stderr)
        return 2
    report = run(root, argv)
    for name, findings in report.items():
        guards = CHECKS[name].guards if name in CHECKS else CLEAN
        print(f"== {name}  ({guards})" if findings else f"== {name}")
        for finding in findings:
            where = finding.path if finding.line is None else f"{finding.path}:{finding.line}"
            print(f"{where}: {finding.message}")
    failed = [name for name, findings in report.items() if findings]
    print(flush=True)
    if failed:
        print(f"FAILED: {' '.join(failed)}", file=sys.stderr)
        return 1
    print("All verification steps passed.")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
