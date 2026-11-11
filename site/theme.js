// Palettes from Omarchy's own themes (themes/*/colors.toml on the quattro
// branch), via the snapshot at shieldsworks/omarchyyc. link, warn and sea are
// the colors used for text; they match the theme's accent, yellow and cyan
// except where those are too faint to read on the theme's background. The
// last one, night-watch, is Omahoy's own. Loaded in <head>, so a page paints
// in its theme from the first frame.
(function () {
  var THEMES = {
    "catppuccin": { bg: "#1e1e2e", well: "#161622", fg: "#cdd6f4", bright: "#cdd6f4", dim: "#6c7086", muted: "#585b70", accent: "#89b4fa", yellow: "#f9e2af", green: "#a6e3a1", link: "#89b4fa", warn: "#f9e2af", sea: "#94e2d5", soft: "#999fb9" },
    "catppuccin-latte": { bg: "#eff1f5", well: "#e3e4e8", fg: "#4c4f69", bright: "#4c4f69", dim: "#9ca0b0", muted: "#acb0be", accent: "#1e66f5", yellow: "#df8e1d", green: "#40a02b", link: "#1d64ef", warn: "#996214", sea: "#137a80", soft: "#676a80" },
    "ethereal": { bg: "#060B1E", well: "#040816", fg: "#ffcead", bright: "#ffcead", dim: "#6d7db6", muted: "#6d7db6", accent: "#7d82d9", yellow: "#E9BB4F", green: "#92a593", link: "#7d82d9", warn: "#e9bb4f", sea: "#a3bfd1", soft: "#90776e" },
    "everforest": { bg: "#2d353b", well: "#21272c", fg: "#d3c6aa", bright: "#d3c6aa", dim: "#4f585e", muted: "#475258", accent: "#7fbbb3", yellow: "#dbbc7f", green: "#a7c080", link: "#7fbbb3", warn: "#dbbc7f", sea: "#83c092", soft: "#a69f8c" },
    "flexoki-light": { bg: "#FFFCF0", well: "#f2efe4", fg: "#100F0F", bright: "#100F0F", dim: "#878580", muted: "#B7B5AC", accent: "#205EA6", yellow: "#D0A215", green: "#879A39", link: "#205ea6", warn: "#90700f", sea: "#2c817a", soft: "#73716d" },
    "gruvbox": { bg: "#282828", well: "#1e1e1e", fg: "#d4be98", bright: "#d4be98", dim: "#7c6f64", muted: "#665c54", accent: "#7daea3", yellow: "#d8a657", green: "#a9b665", link: "#7daea3", warn: "#d8a657", sea: "#89b482", soft: "#a09176" },
    "hackerman": { bg: "#0B0C16", well: "#080910", fg: "#ddf7ff", bright: "#ddf7ff", dim: "#6a6e95", muted: "#2d3450", accent: "#82FB9C", yellow: "#50f7d4", green: "#4fe88f", link: "#82fb9c", warn: "#50f7d4", sea: "#7cf8f7", soft: "#717f88" },
    "jade-horse": { bg: "#142620", well: "#0f1e19", fg: "#dfede5", bright: "#f0faf4", dim: "#91ad9f", muted: "#64887b", accent: "#8bcbb1", yellow: "#c5cc91", green: "#8bcbb1", link: "#8bcbb1", warn: "#c5cc91", sea: "#86c7c5", soft: "#809088" },
    "kanagawa": { bg: "#1f1f28", well: "#17171e", fg: "#dcd7ba", bright: "#dcd7ba", dim: "#727169", muted: "#54546D", accent: "#dcd7ba", yellow: "#c0a36e", green: "#76946a", link: "#dcd7ba", warn: "#c0a36e", sea: "#6a9589", soft: "#8c8a7d" },
    "last-horizon": { bg: "#0c0b0c", well: "#090809", fg: "#FAFCFB", bright: "#e2dddc", dim: "#584e51", muted: "#584e51", accent: "#b59790", yellow: "#6B5E73", green: "#87a9b0", link: "#b59790", warn: "#807587", sea: "#a5a0b6", soft: "#7c7c7c" },
    "lumon": { bg: "#16242d", well: "#101b21", fg: "#d6e2ee", bright: "#f2fcff", dim: "#4d86b0", muted: "#304860", accent: "#8bc9eb", yellow: "#6fa4c9", green: "#5e95bc", link: "#8bc9eb", warn: "#6fa4c9", sea: "#b4e4f6", soft: "#818e98" },
    "lupine": { bg: "#fafafa", well: "#ececec", fg: "#212121", bright: "#000000", dim: "#757575", muted: "#9e9e9e", accent: "#3264eb", yellow: "#026fde", green: "#4a2fd0", link: "#3264eb", warn: "#026fde", sea: "#0c67de", soft: "#707070" },
    "matte-black": { bg: "#121212", well: "#0d0d0d", fg: "#bebebe", bright: "#bebebe", dim: "#555555", muted: "#333333", accent: "#e68e0d", yellow: "#b91c1c", green: "#FFC107", link: "#e68e0d", warn: "#e68e0d", sea: "#bebebe", soft: "#8a8a8a" },
    "miasma": { bg: "#222222", well: "#191919", fg: "#c2c2b0", bright: "#c2c2b0", dim: "#555555", muted: "#666666", accent: "#78824b", yellow: "#b36d43", green: "#5f875f", link: "#848d5b", warn: "#ba7953", sea: "#c9a554", soft: "#8d8d81" },
    "nord": { bg: "#2e3440", well: "#222730", fg: "#d8dee9", bright: "#d8dee9", dim: "#667080", muted: "#4c566a", accent: "#81a1c1", yellow: "#ebcb8b", green: "#a3be8c", link: "#81a1c1", warn: "#ebcb8b", sea: "#88c0d0", soft: "#9aa0ab" },
    "retro-82": { bg: "#05182e", well: "#031222", fg: "#f6dcac", bright: "#f6dcac", dim: "#3f8f8a", muted: "#2a6b78", accent: "#faa968", yellow: "#e97b3c", green: "#028391", link: "#faa968", warn: "#e97b3c", sea: "#8cbfb8", soft: "#888372" },
    "ristretto": { bg: "#2c2525", well: "#211b1b", fg: "#e6d9db", bright: "#e6d9db", dim: "#72696a", muted: "#72696a", accent: "#f38d70", yellow: "#f9cc6c", green: "#adda78", link: "#f38d70", warn: "#f9cc6c", sea: "#85dacc", soft: "#988e8f" },
    "rose-pine": { bg: "#faf4ed", well: "#ede7e1", fg: "#575279", bright: "#575279", dim: "#9893a5", muted: "#cecacd", accent: "#56949f", yellow: "#ea9d34", green: "#286983", link: "#3e6f78", warn: "#955a14", sea: "#3e6f78", soft: "#6f6a8a" },
    "solitude": { bg: "#101315", well: "#0c0e10", fg: "#cacccc", bright: "#a5aeb4", dim: "#4b4e55", muted: "#4b4e55", accent: "#798186", yellow: "#d9dbdc", green: "#9fa5a9", link: "#798186", warn: "#d9dbdc", sea: "#7d7d7d", soft: "#7e8181" },
    "starship": { bg: "#1c1c1e", well: "#161618", fg: "#f5f5f7", bright: "#ffffff", dim: "#98989d", muted: "#8e8e93", accent: "#ff9f0a", yellow: "#ffc27a", green: "#ff9f0a", link: "#ff9f0a", warn: "#ffc27a", sea: "#a6dcef", soft: "#878789" },
    "tokyo-night": { bg: "#1a1b26", well: "#13141c", fg: "#a9b1d6", bright: "#c0caf5", dim: "#565f89", muted: "#414868", accent: "#7aa2f7", yellow: "#e0af68", green: "#9ece6a", link: "#7aa2f7", warn: "#e0af68", sea: "#449dab", soft: "#7e84a1" },
    "vantablack": { bg: "#000000", well: "#090909", fg: "#ffffff", bright: "#ffffff", dim: "#505050", muted: "#7a7a7a", accent: "#8d8d8d", yellow: "#cecece", green: "#b6b6b6", link: "#8d8d8d", warn: "#cecece", sea: "#b0b0b0", soft: "#787878" },
    "white": { bg: "#ffffff", well: "#f5f5f5", fg: "#000000", bright: "#000000", dim: "#808080", muted: "#c0c0c0", accent: "#6e6e6e", yellow: "#4a4a4a", green: "#3a3a3a", link: "#6e6e6e", warn: "#4a4a4a", sea: "#3e3e3e", soft: "#4d4d4d" },
    "night-watch": { bg: "#0c0404", well: "#070202", fg: "#e8503f", bright: "#ff7a66", dim: "#6e2219", muted: "#551a12", accent: "#ff3b2f", yellow: "#ffa28a", green: "#ff6b5a", link: "#ff6b5a", warn: "#ffa28a", sea: "#e0604c", soft: "#d9483a" }
  };
  var NAMES = Object.keys(THEMES), root = document.documentElement, listeners = [];
  var own = function (n) { return Object.prototype.hasOwnProperty.call(THEMES, n); };
  var LABELS = { "rose-pine": "Rosé Pine", "tokyo-night": "Tokyo Night", "night-watch": "Night Watch" };
  var label = function (name) {
    if (LABELS[name]) return LABELS[name];
    return name.split("-").map(function (w) { return w.charAt(0).toUpperCase() + w.slice(1); }).join(" ");
  };

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
      var hint = picker.querySelector(".hint");
      var select = document.createElement("select");
      select.id = "theme-select";
      select.setAttribute("aria-label", "Theme");
      NAMES.forEach(function (name) {
        var opt = document.createElement("option");
        opt.value = name;
        opt.textContent = label(name);
        select.appendChild(opt);
      });
      select.value = root.dataset.theme;
      select.addEventListener("change", function () { apply(select.value, true); });
      picker.insertBefore(select, hint);
      listeners.push(function (name) { select.value = name; });
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
