pragma ComponentBehavior: Bound
import QtQuick
import Quickshell

// The form's state and every call to `omahoy gui`. The panel holds the
// answers; omahoy decides what they mean (docs/gui.md in the omahoy repo).
// Whatever the panel shows as a problem, an area, a refusal or a plan came
// back from the binary in a report.
Item {
    id: s

    property Theme theme: Theme {}
    readonly property string bin: Quickshell.env("OMAHOY_BIN") || "omahoy"

    // The answers.
    property string name: ""
    property string units: "feet"
    property string draft: ""
    property string clearance: ""
    property string home: ""
    property string vault: "~/Logbook"
    property bool keepGit: true
    property var area: null
    property var apps: []

    // What omahoy last said about them.
    property var report: null
    property bool exists: false
    property bool loaded: false
    property string loadError: ""
    property bool homeEdited: false
    property var boatSources: []

    // Hardware. "keep" leaves the boat's sources as they were; "chosen"
    // builds them from the choices below.
    property string hardwareMode: "keep"
    property var devices: []
    property var held: []
    property var nets: []
    property var netProbe: null
    property bool probed: false
    property string hardwareError: ""
    property bool needLogout: false
    property string probeLog: ""

    // The steps, review last.
    property string step: "boat"
    property int reached: 0
    property var plan: null
    property string planError: ""
    property string applyState: ""
    property var applyLog: []
    property string applyError: ""

    readonly property bool wantsLogbook: apps.indexOf("logbook") >= 0
    readonly property var stepList: wantsLogbook
        ? [{id: "boat", label: "Boat"}, {id: "hardware", label: "Hardware"}, {id: "home", label: "Home and area"},
           {id: "apps", label: "Apps"}, {id: "logbook", label: "Logbook"}, {id: "review", label: "Review"}]
        : [{id: "boat", label: "Boat"}, {id: "hardware", label: "Hardware"}, {id: "home", label: "Home and area"},
           {id: "apps", label: "Apps"}, {id: "review", label: "Review"}]
    readonly property int stepIndex: {
        for (let i = 0; i < stepList.length; i++) if (stepList[i].id === step) return i;
        return 0;
    }
    readonly property bool complete: report !== null && report.complete === true && !refusing
    readonly property bool planning: planner.running
    readonly property bool applying: applier.running
    readonly property bool checking: checker.running || checkTimer.running
    readonly property bool busy: scanner.running || reprober.running || netter.running || joiner.running
        || planner.running || applier.running

    // For checks: the first line of each runner's stderr while it works.
    readonly property string progress: {
        const lines = scanner.running ? scanner.errLines : reprober.running ? reprober.errLines
            : netter.running ? netter.errLines : planner.running ? planner.errLines : [];
        return lines.length ? lines[lines.length - 1] : "";
    }

    function parse(text) {
        try { return JSON.parse(text); } catch (e) { return null; }
    }

    function sourceMatches(source, device) {
        return source === device.source || source === device.spec || source.indexOf(device.spec + ":") === 0;
    }

    function chosenSources() {
        if (hardwareMode === "keep") return boatSources.slice();
        const out = [];
        const add = src => { if (src && out.indexOf(src) < 0) out.push(src); };
        for (const d of devices) if (d.choice === "use") add(d.source);
        for (const h of held) if (h.keep) add(h.source);
        for (const n of nets) if (n.choice === "use") add(n.source);
        return out;
    }
    function sources() {
        const c = chosenSources();
        return c.length ? c : ["demo"];
    }
    readonly property bool onDemo: sources().length === 1 && sources()[0] === "demo"

    // A value that could end its line or open a comment in boat text is
    // refused here, never sent: it could add lines the binary can't see.
    function unsafe(value) {
        return /[\u0000-\u001f\u007f"#]/.test(String(value));
    }
    readonly property var refused: {
        const out = {};
        const fields = {name: name, draft: draft, clearance: clearance, home: home, vault: vault};
        for (const k in fields) {
            if (unsafe(fields[k])) out[k] = k + " can't hold quotes, #, or control characters";
        }
        for (const src of chosenSources()) {
            if (unsafe(src)) out.source = "a source can't hold quotes, #, or control characters";
        }
        return out;
    }
    readonly property bool refusing: Object.keys(refused).length > 0

    function boatText() {
        const L = [];
        if (name.trim() !== "") L.push("name = \"" + name.trim() + "\"");
        L.push("units = " + units);
        if (draft.trim() !== "") L.push("draft = " + draft.trim());
        if (clearance.trim() !== "") L.push("clearance = " + clearance.trim());
        if (home.trim() !== "") L.push("home = " + home.trim());
        if (area) L.push("area = " + [area.south, area.west, area.north, area.east].join(","));
        for (const src of sources()) L.push("source = " + src);
        L.push("apps = " + apps.join(", "));
        L.push("vault = \"" + vault.trim() + "\"");
        L.push("git = " + (keepGit ? "true" : "false"));
        return L.join("\n") + "\n";
    }

    // Problems the report names for a boat.toml key; null is the boat as
    // a whole.
    function problemsFor(key) {
        const out = [];
        if (refused[key]) out.push(refused[key]);
        if (!report) return out;
        const text = boatText().split("\n");
        for (const p of report.problems) {
            if (p.key !== key) continue;
            // A source problem names the line it was found on.
            const row = key === "source" && p.line ? String(text[p.line - 1] || "").replace(/^source\s*=\s*/, "") : "";
            out.push(row ? row + ": " + p.message : p.message);
        }
        return out;
    }
    function stepOf(key) {
        if (key === "source") return "hardware";
        if (key === "home" || key === "area") return "home";
        if (key === "apps") return "apps";
        if (key === "vault" || key === "git") return wantsLogbook ? "logbook" : "review";
        return "boat";
    }
    function problemCount(id) {
        let n = Object.keys(refused).filter(k => stepOf(k) === id).length;
        if (report) n += report.problems.filter(p => stepOf(p.key) === id).length;
        return n;
    }

    // Draft: where the form starts.
    function applyDraft(r) {
        const a = r.answers;
        name = a.name || "";
        units = a.units || "feet";
        draft = a.draft !== null && a.draft !== undefined ? String(a.draft) : "";
        clearance = a.clearance !== null && a.clearance !== undefined ? String(a.clearance) : "";
        home = a.home ? a.home.lat + ", " + a.home.lon : "";
        area = r.suggestedArea || null;
        apps = a.apps ? a.apps.slice() : r.apps.filter(x => !x.refusal).map(x => x.short);
        if (a.vault) vault = a.vault;
        if (a.git !== null && a.git !== undefined) keepGit = a.git;
        boatSources = (a.sources || []).filter(x => x !== "demo");
        report = r;
    }

    function load() {
        loadError = "";
        drafter.argv = [bin, "gui", "draft"];
        drafter.start();
    }

    // Edits. Every one re-checks after a pause.
    function touch() {
        if (loaded) checkTimer.restart();
    }
    function edit(key, value) {
        if (key === "units") { setUnits(String(value)); return; }
        if (key === "name") name = String(value);
        else if (key === "draft") draft = String(value);
        else if (key === "clearance") clearance = String(value);
        else if (key === "home") { home = String(value); homeEdited = true; }
        else if (key === "vault") vault = String(value);
        else if (key === "git") keepGit = value === true || value === "true";
        touch();
    }
    function setUnits(u) {
        if (u === units) return;
        const r = report;
        if (r) {
            if (r.draftIn && r.draftIn[u] !== null && r.draftIn[u] !== undefined) draft = String(r.draftIn[u]);
            const wasDefault = r.defaultClearance && Number(clearance) === r.defaultClearance[units];
            if (wasDefault) clearance = String(r.defaultClearance[u]);
            else if (r.clearanceIn && r.clearanceIn[u] !== null && r.clearanceIn[u] !== undefined) clearance = String(r.clearanceIn[u]);
        }
        units = u;
        touch();
    }
    function toggleApp(short) {
        const i = apps.indexOf(short);
        const next = apps.slice();
        if (i >= 0) next.splice(i, 1); else next.push(short);
        apps = next;
        touch();
    }

    function runCheck() {
        if (refusing) return;
        if (checker.running) { recheck = true; return; }
        checker.input = boatText();
        checker.argv = [bin, "gui", "check"];
        checker.start();
    }
    property bool recheck: false

    // A new report: the area follows the suggested one, and the apps follow
    // the area.
    function takeReport(r) {
        report = r;
        const sa = r.suggestedArea || null;
        if (JSON.stringify(sa) !== JSON.stringify(area)) {
            area = sa;
            if (sa) apps = r.appsAllowed.slice();
            recheck = true;
        }
    }

    // Hardware.
    function listen() {
        hardwareError = "";
        needLogout = false;
        scanner.argv = [bin, "gui", "probe"];
        scanner.start();
    }
    function takeProbe(r) {
        probed = true;
        hardwareMode = "chosen";
        const found = r.devices.map(d => Object.assign({}, d, {
            choice: d.verdict === "ok" && boatSources.some(b => sourceMatches(b, d)) ? "use" : ""
        }));
        devices = found;
        const heard = found.filter(d => d.verdict === "ok");
        held = boatSources.filter(b => !heard.some(d => sourceMatches(b, d)))
            .map(b => ({source: b, keep: true}));
        if (r.uucp && r.uucp.member && !r.uucp.session) needLogout = true;
        const fix = found.find(d => d.fix);
        if (fix && !homeEdited && !(exists && home.trim() !== "")) {
            home = fix.fix.lat.toFixed(4) + ", " + fix.fix.lon.toFixed(4);
        }
        touch();
    }
    function chooseDevice(i, choice) {
        const next = devices.slice();
        next[i] = Object.assign({}, next[i], {choice: choice});
        devices = next;
        hardwareMode = "chosen";
        touch();
    }
    function chooseHeld(i, keep) {
        const next = held.slice();
        next[i] = Object.assign({}, next[i], {keep: keep});
        held = next;
        hardwareMode = "chosen";
        touch();
    }
    function allowSerial() {
        hardwareError = "";
        joiner.argv = [bin, "gui", "join-uucp"];
        joiner.start();
    }
    function deniedSpecs() {
        return devices.filter(d => d.verdict === "denied").map(d => d.spec);
    }
    function takeReprobe(r) {
        const next = devices.slice();
        for (const d of r.devices) {
            const at = next.findIndex(x => x.spec === d.spec);
            const row = Object.assign({}, d, {choice: ""});
            if (at >= 0) next[at] = row; else next.push(row);
        }
        devices = next;
        needLogout = true;
        touch();
    }
    function testNet(hostPort) {
        netProbe = null;
        hardwareError = "";
        if (unsafe(hostPort)) { hardwareError = "A host:port can't hold quotes, #, or control characters."; return; }
        netter.argv = [bin, "gui", "probe", "tcp:" + hostPort.trim()];
        netter.start();
    }
    function chooseNet(choice) {
        if (!netProbe) return;
        const next = nets.filter(n => n.spec !== netProbe.spec);
        next.push({spec: netProbe.spec, source: netProbe.source || netProbe.spec, describe: netProbe.describe, choice: choice});
        nets = next;
        netProbe = null;
        hardwareMode = "chosen";
        touch();
    }
    function dropNet(spec) {
        nets = nets.filter(n => n.spec !== spec);
        touch();
    }
    // The step's own Skip for now: whatever the boat has stays.
    function skipHardware() {
        hardwareMode = "keep";
        next();
    }

    // Steps.
    function goTo(id) {
        const i = stepList.findIndex(x => x.id === id);
        if (i < 0) return false;
        if (checkTimer.running) { checkTimer.stop(); runCheck(); }
        step = id;
        reached = Math.max(reached, i);
        if (id === "review") runPlan();
        return true;
    }
    function next() {
        if (stepIndex < stepList.length - 1) goTo(stepList[stepIndex + 1].id);
    }
    function back() {
        if (stepIndex > 0 && !applier.running) goTo(stepList[stepIndex - 1].id);
    }

    function runPlan() {
        plan = null;
        if (refusing) { planError = "Fix the fields marked above first."; return; }
        planError = "";
        planner.input = boatText();
        planner.argv = [bin, "gui", "plan"];
        planner.start();
    }
    function go() {
        if (!complete || busy || refusing) return;
        applyLog = [];
        applyError = "";
        applyState = "running";
        applier.input = boatText();
        applier.argv = [bin, "gui", "apply"];
        applier.start();
    }
    function openChart() {
        opener.argv = ["omarchy", "shell", "shell", "toggle", "org.omahoy.helm", "{}"];
        opener.start();
    }

    Timer {
        id: checkTimer
        interval: 300
        onTriggered: s.runCheck()
    }

    Runner {
        id: drafter
        onFinished: code => {
            const r = code === 0 ? s.parse(drafter.out) : null;
            if (r === null) {
                s.loadError = code === 0 ? "omahoy gui draft printed something other than JSON"
                    : (drafter.err || "omahoy gui draft failed") + "\nRun omahoy doctor in a terminal to see what is wrong with boat.toml.";
                return;
            }
            s.exists = r.exists === true;
            s.applyDraft(r.report);
            s.loaded = true;
            s.touch();
        }
    }
    Runner {
        id: checker
        onFinished: code => {
            const r = code === 0 ? s.parse(checker.out) : null;
            if (r !== null) s.takeReport(r);
            if (s.recheck) {
                s.recheck = false;
                s.runCheck();
            }
        }
    }
    Runner {
        id: scanner
        onFinished: code => {
            const r = code === 0 ? s.parse(scanner.out) : null;
            if (r === null) s.hardwareError = "Couldn't listen: " + (scanner.err || "omahoy gui probe failed");
            else s.takeProbe(r);
        }
    }
    Runner {
        id: joiner
        onFinished: code => {
            if (code !== 0) {
                s.hardwareError = "Couldn't add you to the uucp group: " + (joiner.err || "dismissed");
                return;
            }
            reprober.argv = [s.bin, "gui", "probe", "--uucp"].concat(s.deniedSpecs());
            reprober.start();
        }
    }
    Runner {
        id: reprober
        onFinished: code => {
            const r = code === 0 ? s.parse(reprober.out) : null;
            if (r === null) s.hardwareError = "Couldn't listen again: " + (reprober.err || "omahoy gui probe failed");
            else s.takeReprobe(r);
        }
    }
    Runner {
        id: netter
        onFinished: code => {
            const r = code === 0 ? s.parse(netter.out) : null;
            if (r === null || r.devices.length === 0) s.hardwareError = "Couldn't test the source: " + (netter.err || "no answer");
            else s.netProbe = r.devices[0];
        }
    }
    Runner {
        id: planner
        onFinished: code => {
            const r = code === 0 ? s.parse(planner.out) : null;
            if (r === null) s.planError = planner.err || "omahoy gui plan failed";
            else s.plan = r;
        }
    }
    Runner {
        id: applier
        onLinesChanged: s.applyLog = applier.lines
        onFinished: code => {
            s.applyLog = applier.lines;
            if (code === 0) s.applyState = "done";
            else if (code === 1) s.applyState = "needs";
            else {
                s.applyState = "failed";
                s.applyError = applier.err || "omahoy gui apply failed";
            }
        }
    }
    Runner {
        id: opener
    }

    Component.onCompleted: load()
}
