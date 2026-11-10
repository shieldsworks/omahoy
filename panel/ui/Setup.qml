pragma ComponentBehavior: Bound
import QtQuick
import Quickshell

// The form's state and every call to `omahoy gui`. The panel holds the
// answers; omahoy decides what they mean (docs/gui.md).
// Whatever the panel shows as a problem, an area, a refusal or a plan came
// back from the binary in a report.
Item {
    id: s

    property Theme theme: Theme {}
    // The omahoy binary every call runs. `omahoy panel` passes its own path
    // in the open payload; standalone it is OMAHOY_BIN, else omahoy on PATH.
    property string bin: ""

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

    // The boat text is the revision: every request carries the text it was
    // made from, and an answer counts only while the form still says the
    // same. A report or plan for older text is dropped.
    readonly property string boat: boatText()
    property string reportText: ""
    property string checkSent: ""
    property string planText: ""
    property string planSent: ""
    property string applySent: ""
    // The units the sailor picked, until a report converts the numbers.
    property string wantUnits: ""
    readonly property var unitList: ["feet", "meters", "fathoms"]

    // Hardware: one entry per source, keyed by what it is (hwKey), never by
    // how it was heard. An entry holds the boat's own value (`boat`), the
    // value the last probe offered (`offered`), the sailor's `choice` ("keep"
    // the boat's value, "use" the offered one, "skip", or "" for none yet),
    // whether the last listen heard it (`heard`) and the probe's `row`.
    // Probes add entries and update offered, heard and row; only the sailor
    // changes a choice. The boat's sources are read from the choices.
    property var hw: ({})
    readonly property var hardware: Object.keys(hw).map(k => hw[k])
    property bool probed: false
    property string hardwareError: ""
    property bool needLogout: false
    property bool useUucp: false

    // The steps, review last.
    property string step: "boat"
    property int reached: 0
    property var plan: null
    property string planError: ""
    property string applyState: ""
    property var applyLog: []
    property string applyError: ""
    property bool closeRefused: false

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
    readonly property bool reportCurrent: report !== null && reportText === boat
    readonly property bool planCurrent: plan !== null && planText === boat
    readonly property bool complete: reportCurrent && report.complete === true && !refusing
    readonly property bool ready: complete && planCurrent && wantUnits === ""
    readonly property bool planning: planner.running
    readonly property bool applying: applyState === "running" || launcher.running
    readonly property bool listening: scanner.running || reprober.running || netter.running || joiner.running
    readonly property bool checking: checker.running || checkTimer.running
    onBusyChanged: if (!busy) closeRefused = false
    readonly property bool busy: listening || planner.running || applying

    // For checks: the first line of each runner's stderr while it works.
    readonly property string progress: {
        const lines = scanner.running ? scanner.errLines : reprober.running ? reprober.errLines
            : netter.running ? netter.errLines : planner.running ? planner.errLines : [];
        return lines.length ? lines[lines.length - 1] : "";
    }

    function parse(text) {
        try { return JSON.parse(text); } catch (e) { return null; }
    }

    // A serial source is its device path, whatever the baud; a tcp source
    // is its host and port.
    function hwKey(source) {
        const src = String(source).trim();
        if (src.indexOf("serial:") === 0) {
            const m = /^(.*):(4800|9600|19200|38400|57600|115200)$/.exec(src.slice(7));
            return "serial:" + (m ? m[1] : src.slice(7));
        }
        const tcp = /^tcp:(.+):(\d+)$/.exec(src);
        if (tcp) return "tcp:" + tcp[1].toLowerCase() + ":" + Number(tcp[2]);
        return src;
    }
    function sourceOf(e) {
        return e.choice === "use" ? e.offered : e.choice === "keep" ? e.boat : null;
    }
    function chosenSources() {
        const out = [];
        for (const e of hardware) {
            const src = sourceOf(e);
            if (src && out.indexOf(src) < 0) out.push(src);
        }
        return out;
    }
    function sources() {
        const c = chosenSources();
        return c.length ? c : ["demo"];
    }
    readonly property bool onDemo: sources().length === 1 && sources()[0] === "demo"

    // A value that could end its line or open a comment in boat text is
    // refused here, never sent: it could add lines the binary can't see.
    // Line ends include U+0085, U+2028 and U+2029 as well as C0, DEL and C1.
    function unsafe(value) {
        return /[\u0000-\u001f\u007f-\u009f\u2028\u2029"#]/.test(String(value));
    }
    readonly property var refused: {
        const out = {};
        const fields = {name: name, units: units, draft: draft, clearance: clearance, home: home, vault: vault};
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
        const text = reportText.split("\n");
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
        const m = {};
        for (const src of a.sources || []) {
            if (src === "demo") continue;
            const k = hwKey(src);
            if (!m[k]) m[k] = {key: k, boat: src, offered: null, choice: "keep", heard: false, row: null};
        }
        hw = m;
        report = r;
        reportText = "";
    }

    // Starts the form once, the first time the panel is opened.
    function begin(path) {
        if (path) bin = path;
        if (!bin) bin = "omahoy";
        if (!loaded && !drafter.running) load();
    }
    // An apply already in runDir (running, or finished while no panel was
    // loaded) is picked up: the form is the text it applies, on review.
    function load() {
        loadError = "";
        if (runDir === "") draftFrom("");
        else pollApply();
    }
    function draftFrom(text) {
        adoptText = text;
        drafter.input = text;
        drafter.argv = text === "" ? [bin, "gui", "draft"] : [bin, "gui", "check"];
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
    // A change between the three units waits, as wantUnits, for a report
    // on the text as it stands, with nothing refused, that converts every
    // number in it. Until then units, draft and clearance stay as typed, so
    // a number is never read in units it wasn't typed in.
    function setUnits(u) {
        if (unitList.indexOf(u) < 0 || unitList.indexOf(units) < 0) {
            // No conversion to or from a unit omahoy doesn't know; the
            // refusal check or the binary answers for it.
            wantUnits = "";
            units = u;
            touch();
            return;
        }
        wantUnits = u === units ? "" : u;
        convert();
    }
    function convert() {
        const u = wantUnits;
        if (u === "" || !reportCurrent || refusing) return;
        const r = report;
        const a = r.answers;
        // "same" for an empty field; null while a field holds something the
        // report has no conversion for, such as "2m", which would otherwise
        // be read in the new units once it is fixed.
        const to = (text, table) => text.trim() === "" ? "same"
            : table && table[u] !== null && table[u] !== undefined ? String(table[u]) : null;
        const d = to(draft, r.draftIn);
        const c = to(clearance, r.clearanceIn);
        if (d === null || c === null) return;
        wantUnits = "";
        if (d !== "same") draft = d;
        if (c !== "same") {
            const wasDefault = r.defaultClearance && Number(a.clearance) === r.defaultClearance[units];
            clearance = wasDefault ? String(r.defaultClearance[u]) : c;
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
        if (refusing || checker.running) return;
        checkSent = boat;
        checker.input = checkSent;
        checker.argv = [bin, "gui", "check"];
        checker.start();
    }

    // A report for the current answers: the area follows the suggested one,
    // and the apps follow the area. A changed area changes the text, so the
    // check runs again.
    function takeReport(r, text) {
        report = r;
        reportText = text;
        convert();
        const sa = r.suggestedArea || null;
        if (JSON.stringify(sa) !== JSON.stringify(area)) {
            area = sa;
            if (sa) apps = r.appsAllowed.slice();
        }
    }

    // Hardware.
    function listen() {
        if (busy) return;
        hardwareError = "";
        needLogout = false;
        scanner.argv = [bin, "gui", "probe"].concat(useUucp ? ["--uucp"] : []);
        scanner.start();
    }

    // Takes probe rows into the entries. `all` is a scan of every serial
    // device, so a serial entry it didn't report wasn't heard.
    function hear(devices, all) {
        const m = Object.assign({}, hw);
        if (all) {
            for (const k in m) {
                if (k.indexOf("serial:") === 0 && !devices.some(d => hwKey(d.spec) === k))
                    m[k] = Object.assign({}, m[k], {heard: false, row: null});
            }
        }
        const errors = [];
        for (const d of devices) {
            const k = hwKey(d.spec);
            if (d.verdict === "error" && !m[k]) { errors.push(d.spec + ": " + d.describe); continue; }
            const was = m[k] || {key: k, boat: null, offered: null, choice: "", heard: false, row: null};
            m[k] = Object.assign({}, was, {heard: d.verdict === "ok", row: d, offered: d.source || was.offered});
        }
        hw = m;
        if (errors.length) hardwareError = errors.join("\n");
    }
    function takeProbe(r) {
        probed = true;
        hear(r.devices, true);
        if (r.uucp && r.uucp.member && !r.uucp.session) { needLogout = true; useUucp = true; }
        const fix = r.devices.find(d => d.fix);
        if (fix && !homeEdited && !(exists && home.trim() !== "")) {
            home = fix.fix.lat.toFixed(4) + ", " + fix.fix.lon.toFixed(4);
        }
        touch();
    }
    // The sailor's choice for one entry: "use" needs a value the probe
    // offered, "keep" a value the boat has.
    function choose(key, choice) {
        const e = hw[key];
        if (!e || ["use", "keep", "skip"].indexOf(choice) < 0) return false;
        if ((choice === "use" && !e.offered) || (choice === "keep" && !e.boat)) return false;
        const m = Object.assign({}, hw);
        m[key] = Object.assign({}, e, {choice: choice});
        hw = m;
        touch();
        return true;
    }
    function allowSerial() {
        if (busy) return;
        hardwareError = "";
        joiner.argv = [bin, "gui", "join-uucp"];
        joiner.start();
    }
    function deniedSpecs() {
        return hardware.filter(e => e.row && e.row.verdict === "denied").map(e => e.row.spec);
    }
    function takeReprobe(r) {
        hear(r.devices, false);
        needLogout = true;
        touch();
    }
    function testNet(hostPort) {
        if (busy) return;
        hardwareError = "";
        if (unsafe(hostPort)) { hardwareError = "A host:port can't hold quotes, #, or control characters."; return; }
        netter.argv = [bin, "gui", "probe", "tcp:" + hostPort.trim()];
        netter.start();
    }
    function takeNet(r) {
        const d = r.devices[0];
        if (!d.source) { hardwareError = "Couldn't test " + d.spec + ": " + d.describe; return; }
        hear([d], false);
        touch();
    }
    // The step's own Skip for now: the boat's sources stay as they are.
    function skipHardware() {
        const m = {};
        for (const k in hw) m[k] = Object.assign({}, hw[k], {choice: hw[k].boat ? "keep" : ""});
        hw = m;
        touch();
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
        if (stepIndex > 0 && !applying) goTo(stepList[stepIndex - 1].id);
    }

    // One plan at a time; a plan for older text is dropped and, on the
    // review step, made again when the running one ends.
    function runPlan() {
        if (planner.running) return;
        plan = null;
        if (refusing) { planError = "Fix the fields marked above first."; return; }
        planError = "";
        planSent = boat;
        planner.input = planSent;
        planner.argv = [bin, "gui", "plan"];
        planner.start();
    }
    // Apply runs detached (scripts/apply.sh) with its files in runDir, so
    // it outlives the panel: the shell reloads every panel when a plugin is
    // added mid-apply. The panel follows the files while it runs.
    readonly property string runDir: {
        const d = Quickshell.env("XDG_RUNTIME_DIR");
        return d ? d + "/omahoy-setup" : "";
    }
    // Set by the window: the plugin's folder holds scripts/apply.sh.
    property string applyScript: ""
    property string adoptText: ""

    // Go applies the text the check and the plan were made for.
    function go() {
        if (!ready || busy || refusing) return;
        applyLog = [];
        applyError = "";
        if (runDir === "") {
            applyState = "failed";
            applyError = "XDG_RUNTIME_DIR is not set, so apply has nowhere to keep its progress. Run omahoy apply in a terminal.";
            return;
        }
        applyState = "running";
        applySent = boat;
        launcher.input = applySent;
        launcher.argv = ["bash", applyScript, "start", runDir, bin];
        launcher.start();
    }
    function pollApply() {
        if (watcher.running) return;
        watcher.argv = ["bash", applyScript, "status", runDir];
        watcher.start();
    }
    // What apply.sh status said: a state line, then prefixed out, err and
    // in lines.
    function takeApply(lines) {
        const state = lines.length ? lines[0] : "none";
        const part = p => lines.filter(l => l.indexOf(p) === 0).map(l => l.slice(2));
        if (!loaded) {
            draftFrom(state === "none" ? "" : part("i ").join("\n") + "\n");
            return;
        }
        if (state === "none") {
            if (applyState === "running") {
                applyState = "failed";
                applyError = "The apply's record is gone, so how it ended is unknown. Run omahoy doctor to see.";
            }
            return;
        }
        applyLog = part("o ");
        if (state === "running") {
            applyState = "running";
            applyTimer.restart();
            return;
        }
        clearer.argv = ["bash", applyScript, "clear", runDir];
        clearer.start();
        const code = state.indexOf("code ") === 0 ? Number(state.slice(5)) : -1;
        if (code === 0 && applySent !== boat) {
            applyState = "";
            applyError = "Applied the answers as they were when you pressed Go; later edits are not applied yet.";
        } else if (code === 0) applyState = "done";
        else if (code === 1) applyState = "needs";
        else {
            applyState = "failed";
            applyError = state === "lost" ? "The apply stopped before it finished. Run omahoy apply in a terminal to finish it."
                : part("e ").join("\n") || "omahoy gui apply failed";
        }
    }
    // Answers that change after an apply no longer match what it did.
    onBoatChanged: {
        if (applyState !== "" && applyState !== "running") { applyState = ""; applyLog = []; applyError = ""; }
        if (step === "review" && loaded) runPlan();
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
            const adopted = s.adoptText !== "";
            s.exists = adopted || r.exists === true;
            s.applyDraft(adopted ? r : r.report);
            s.loaded = true;
            if (adopted) {
                s.applySent = s.boat;
                s.goTo("review");
                s.pollApply();
            }
            s.touch();
        }
    }
    Runner {
        id: checker
        onFinished: code => {
            const r = code === 0 ? s.parse(checker.out) : null;
            if (r !== null && s.checkSent === s.boat) s.takeReport(r, s.checkSent);
            if (s.checkSent !== s.boat) s.runCheck();
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
            s.useUucp = true;
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
            else s.takeNet(r);
        }
    }
    Runner {
        id: planner
        onFinished: code => {
            const r = code === 0 ? s.parse(planner.out) : null;
            if (s.planSent !== s.boat) {
                if (s.step === "review") s.runPlan();
            } else if (r === null) s.planError = planner.err || "omahoy gui plan failed";
            else {
                s.plan = r;
                s.planText = s.planSent;
            }
        }
    }
    Runner {
        id: launcher
        onFinished: code => {
            if (code === 0) s.pollApply();
            else if (code === 3) {
                // Another window's apply holds the run; its result is not
                // these answers'.
                s.applyState = "";
                s.applyError = "Another apply is running, so these answers were not applied. Press Go again when it has finished.";
            } else {
                s.applyState = "failed";
                s.applyError = launcher.err || "Couldn't start omahoy gui apply";
            }
        }
    }
    Runner {
        id: watcher
        onFinished: code => s.takeApply(code === 0 ? watcher.lines : [])
    }
    Timer {
        id: applyTimer
        interval: 400
        onTriggered: s.pollApply()
    }
    Runner {
        id: clearer
    }
    Runner {
        id: opener
    }
}
