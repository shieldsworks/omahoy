// Palettes from Omarchy's own themes (themes/*/colors.toml), shared by every
// page. link, warn and sea are the colors used for text; they match the
// theme's accent, yellow and cyan except where those are too faint to read
// on the theme's background. Loaded in <head>, so a page paints in its theme
// from the first frame.
(function () {
  var THEMES = {
    "tokyo-night": { bg: "#1a1b26", well: "#13141c", fg: "#a9b1d6", bright: "#c0caf5", dim: "#565f89", muted: "#414868", accent: "#7aa2f7", yellow: "#e0af68", green: "#9ece6a", link: "#7aa2f7", warn: "#e0af68", sea: "#449dab", soft: "#7e84a1" },
    "catppuccin":  { bg: "#1e1e2e", well: "#161622", fg: "#cdd6f4", bright: "#cdd6f4", dim: "#6c7086", muted: "#585b70", accent: "#89b4fa", yellow: "#f9e2af", green: "#a6e3a1", link: "#89b4fa", warn: "#f9e2af", sea: "#94e2d5", soft: "#999fb9" },
    "gruvbox":     { bg: "#282828", well: "#1e1e1e", fg: "#d4be98", bright: "#d4be98", dim: "#7c6f64", muted: "#665c54", accent: "#7daea3", yellow: "#d8a657", green: "#a9b665", link: "#7daea3", warn: "#d8a657", sea: "#89b482", soft: "#a09176" },
    "matte-black": { bg: "#121212", well: "#0d0d0d", fg: "#bebebe", bright: "#bebebe", dim: "#555555", muted: "#333333", accent: "#e68e0d", yellow: "#b91c1c", green: "#FFC107", link: "#e68e0d", warn: "#e68e0d", sea: "#bebebe", soft: "#8a8a8a" },
    "rose-pine":   { bg: "#faf4ed", well: "#ede7e1", fg: "#575279", bright: "#575279", dim: "#9893a5", muted: "#cecacd", accent: "#56949f", yellow: "#ea9d34", green: "#286983", link: "#3e6f78", warn: "#955a14", sea: "#3e6f78", soft: "#6f6a8a" },
    "white":       { bg: "#ffffff", well: "#f5f5f5", fg: "#000000", bright: "#000000", dim: "#808080", muted: "#c0c0c0", accent: "#6e6e6e", yellow: "#4a4a4a", green: "#3a3a3a", link: "#6e6e6e", warn: "#4a4a4a", sea: "#3e3e3e", soft: "#4d4d4d" },
    // Omahoy's own: red on black for the helm after dark.
    "night-watch": { bg: "#0c0404", well: "#070202", fg: "#e8503f", bright: "#ff7a66", dim: "#6e2219", muted: "#551a12", accent: "#ff3b2f", yellow: "#ffa28a", green: "#ff6b5a", link: "#ff6b5a", warn: "#ffa28a", sea: "#e0604c", soft: "#d9483a" }
  };
  var NAMES = Object.keys(THEMES), root = document.documentElement, listeners = [];
  var own = function (n) { return Object.prototype.hasOwnProperty.call(THEMES, n); };

  function apply(name, remember) {
    var t = THEMES[name];
    for (var k in t) root.style.setProperty("--" + k, t[k]);
    root.dataset.theme = name;
    var meta = document.querySelector('meta[name="theme-color"]');
    if (meta) meta.setAttribute("content", t.bg);
    if (remember) try { localStorage.setItem("omahoy-theme", name); } catch (e) {}
    listeners.forEach(function (fn) { fn(name, t); });
  }

  // A shared link (?theme=night-watch) wins, then the visitor's last pick.
  var first = new URLSearchParams(location.search).get("theme");
  if (!own(first)) {
    first = null;
    try { first = localStorage.getItem("omahoy-theme"); } catch (e) {}
  }
  if (!own(first)) {
    first = window.matchMedia && matchMedia("(prefers-color-scheme: light)").matches ? "rose-pine" : "tokyo-night";
  }
  apply(first, false);

  window.OMAHOY_THEMES = THEMES;
  window.omahoyTheme = {
    names: NAMES,
    current: function () { return root.dataset.theme; },
    colors: function () { return THEMES[root.dataset.theme]; },
    apply: function (name) { if (own(name)) apply(name, true); },
    onChange: function (fn) { listeners.push(fn); }
  };

  // The picker, wherever a page has an element with id="themes", and T
  // (Shift+T back) to cycle through the themes anywhere.
  document.addEventListener("DOMContentLoaded", function () {
    // The recordings play on their own, except for visitors who'd rather
    // nothing moved: theirs wait, with controls.
    var still = window.matchMedia && matchMedia("(prefers-reduced-motion: reduce)");
    var hold = function () {
      document.querySelectorAll("video[autoplay]").forEach(function (v) {
        if (still.matches) { v.pause(); v.controls = true; }
        else if (v.paused) { v.play().catch(function () {}); }
      });
    };
    if (still) { hold(); still.addEventListener("change", hold); }

    var picker = document.getElementById("themes");
    if (picker) {
      var hint = picker.querySelector(".hint"), buttons = NAMES.map(function (name) {
        var btn = document.createElement("button");
        btn.type = "button"; btn.textContent = name; btn.dataset.theme = name;
        btn.addEventListener("click", function () { apply(name, true); });
        picker.insertBefore(btn, hint);
        return btn;
      });
      var mark = function (name) {
        buttons.forEach(function (btn) { btn.setAttribute("aria-pressed", String(btn.dataset.theme === name)); });
      };
      listeners.push(mark);
      mark(root.dataset.theme);
    }
    document.addEventListener("keydown", function (e) {
      if (e.key !== "t" && e.key !== "T") return;
      if (e.ctrlKey || e.metaKey || e.altKey) return;
      if (e.target.closest && e.target.closest("input, textarea, select, [contenteditable]")) return;
      var n = NAMES.indexOf(root.dataset.theme);
      apply(NAMES[(n + (e.shiftKey ? NAMES.length - 1 : 1)) % NAMES.length], true);
    });
  });
})();
