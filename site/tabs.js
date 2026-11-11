// Install-path tabs on pages with [data-tabs]. Without this file both
// panels stay stacked, and the tab links are ordinary #panel / #terminal
// anchors. With it, the matching hash selects a tab, arrow keys move
// between them, and the default (no hash) is the Setup panel.
(function () {
  function ready(fn) {
    if (document.readyState === "loading") {
      document.addEventListener("DOMContentLoaded", fn);
    } else {
      fn();
    }
  }

  ready(function () {
    var root = document.querySelector("[data-tabs]");
    if (!root) return;
    var tabs = Array.prototype.slice.call(root.querySelectorAll('[role="tab"]'));
    var panels = Array.prototype.slice.call(root.querySelectorAll('[role="tabpanel"]'));
    if (!tabs.length || tabs.length !== panels.length) return;

    var ids = panels.map(function (p) { return p.id; });
    var DEFAULT = ids.indexOf("panel") >= 0 ? "panel" : ids[0];

    function fromHash() {
      var h = (location.hash || "").replace(/^#/, "");
      return ids.indexOf(h) >= 0 ? h : DEFAULT;
    }

    function select(id, record) {
      tabs.forEach(function (tab) {
        var on = tab.getAttribute("aria-controls") === id;
        tab.setAttribute("aria-selected", on ? "true" : "false");
        tab.tabIndex = on ? 0 : -1;
      });
      panels.forEach(function (panel) {
        var on = panel.id === id;
        panel.classList.toggle("is-active", on);
        if (on) panel.removeAttribute("hidden");
        else panel.setAttribute("hidden", "");
      });
      if (record) {
        var want = "#" + id;
        if (location.hash !== want) {
          if (history.pushState) history.pushState(null, "", want);
          else location.hash = id;
        }
      }
    }

    root.classList.add("tabs-js");
    select(fromHash(), false);

    window.addEventListener("hashchange", function () { select(fromHash(), false); });
    window.addEventListener("popstate", function () { select(fromHash(), false); });

    tabs.forEach(function (tab) {
      tab.addEventListener("click", function (e) {
        e.preventDefault();
        select(tab.getAttribute("aria-controls"), true);
      });
      tab.addEventListener("keydown", function (e) {
        var i = tabs.indexOf(tab);
        var next = i;
        if (e.key === "ArrowRight" || e.key === "ArrowDown") next = (i + 1) % tabs.length;
        else if (e.key === "ArrowLeft" || e.key === "ArrowUp") next = (i - 1 + tabs.length) % tabs.length;
        else if (e.key === "Home") next = 0;
        else if (e.key === "End") next = tabs.length - 1;
        else if (e.key === " " || e.key === "Spacebar") {
          e.preventDefault();
          select(tab.getAttribute("aria-controls"), true);
          return;
        } else return;
        e.preventDefault();
        var id = tabs[next].getAttribute("aria-controls");
        select(id, true);
        tabs[next].focus();
      });
    });
  });
})();
