pragma ComponentBehavior: Bound
import QtQuick
import Quickshell
import Quickshell.Io

// Omahoy setup, in a window of its own. Run standalone (ui/shell.qml) it
// owns its process; as the shell's panel the shell opens and hides it.
Item {
    id: app

    // Set by the Omarchy shell when loaded as a panel.
    property var shell: null
    property var manifest: null
    property bool standalone: true
    property bool opened: standalone

    // The shell hands open() the payload JSON it was summoned with:
    // {"bin": PATH} from `omahoy panel`, or "{}".
    function open(payload) {
        let p = null;
        try { p = JSON.parse(payload || "{}"); } catch (e) { p = null; }
        setup.begin(p && typeof p.bin === "string" ? p.bin : "");
        opened = true;
        Qt.callLater(() => surface.forceActiveFocus());
    }
    function close() {
        opened = false;
    }
    // Closing never stops a running probe or apply: while one runs the
    // window stays and says so. As a panel, the shell keeps this loaded
    // (keepLoaded in manifest.json), so hiding it leaves the work running.
    // A window closed from outside (its close button) turns visible off;
    // the refusal shows it again by binding visible to opened afresh, which
    // keeps the shell's hide working after.
    function dismiss() {
        if (app.setup.busy) {
            app.setup.closeRefused = true;
            opened = true;
            Qt.callLater(() => win.visible = Qt.binding(() => app.opened));
        } else if (standalone) Qt.quit();
        else if (shell) shell.hide("org.omahoy.setup");
        else opened = false;
    }

    // The shell names the plugin's folder in manifest.__sourceDir;
    // standalone, the folder is the one above ui/shell.qml.
    property Setup setup: Setup {
        applyScript: app.manifest && app.manifest.__sourceDir ? app.manifest.__sourceDir + "/scripts/apply.sh"
            : Quickshell.shellPath("../scripts/apply.sh")
    }
    readonly property Theme theme: setup.theme

    // Quickshell keeps a process alive after its last window closes.
    Connections {
        target: Quickshell
        function onLastWindowClosed() { if (app.standalone && !app.setup.busy) Qt.quit(); }
    }

    readonly property bool onReview: setup.step === "review"
    readonly property bool lastStep: setup.stepIndex === setup.stepList.length - 1

    function advance() {
        if (lastStep) setup.go();
        else setup.next();
    }

    // Alt+Left and Alt+Right move between steps and Ctrl+Return goes on;
    // text fields keep every other key.
    function key(e) {
        if (e.modifiers & Qt.AltModifier && e.key === Qt.Key_Right) setup.next();
        else if (e.modifiers & Qt.AltModifier && e.key === Qt.Key_Left) setup.back();
        else if (e.modifiers & Qt.ControlModifier && (e.key === Qt.Key_Return || e.key === Qt.Key_Enter)) app.advance();
        else if (e.key === Qt.Key_Escape) app.dismiss();
        else return;
        e.accepted = true;
    }

    // For checks, where no keyboard can be driven:
    //   quickshell ipc -p ui/shell.qml call omahoysetup status
    IpcHandler {
        target: "omahoysetup"
        function status(): string {
            return JSON.stringify({
                step: app.setup.step,
                stepIndex: app.setup.stepIndex,
                complete: app.setup.complete,
                ready: app.setup.ready,
                name: app.setup.name,
                units: app.setup.units,
                wantUnits: app.setup.wantUnits,
                draft: app.setup.draft,
                clearance: app.setup.clearance,
                closeRefused: app.setup.closeRefused,
                refusing: app.setup.refusing,
                useUucp: app.setup.useUucp,
                planCurrent: app.setup.planCurrent,
                busy: app.setup.busy,
                listening: app.setup.listening,
                hardware: app.setup.hardware.map(e => ({key: e.key, boat: e.boat, offered: e.offered, choice: e.choice,
                                                         heard: e.heard, verdict: e.row ? e.row.verdict : null})),
                hardwareError: app.setup.hardwareError,
                applyError: app.setup.applyError,
                loaded: app.setup.loaded,
                sources: app.setup.sources(),
                apps: app.setup.apps,
                problems: app.setup.report ? app.setup.report.problems.length : -1,
                problemList: app.setup.report ? app.setup.report.problems : [],
                applyState: app.setup.applyState,
                opened: app.opened,
                visible: win.visible
            });
        }
        function next(): void { app.setup.next(); }
        function back(): void { app.setup.back(); }
        function goTo(step: string): bool {
            const n = Number(step);
            const id = step !== "" && !isNaN(n) && app.setup.stepList[n] ? app.setup.stepList[n].id : step;
            return app.setup.goTo(id);
        }
        function set(key: string, value: string): void { app.setup.edit(key, value); }
        function probe(): void { app.setup.listen(); }
        function skipHardware(): void { app.setup.skipHardware(); }
        function go(): void { app.setup.go(); }
        function close(): void { app.dismiss(); }
        function allowSerial(): void { app.setup.allowSerial(); }
        function choose(key: string, choice: string): bool { return app.setup.choose(key, choice); }
        function testNet(hostPort: string): void { app.setup.testNet(hostPort); }
        // What the shell's hide does.
        function hide(): void { app.close(); }
        // What the shell's summon does, with its payload.
        function open(payload: string): void { app.open(payload); }
    }

    FloatingWindow {
        id: win
        title: "Omahoy setup"
        visible: app.opened
        onVisibleChanged: {
            if (!visible && app.opened) app.dismiss();
            else if (visible) Qt.callLater(() => surface.forceActiveFocus());
        }
        implicitWidth: Number(Quickshell.env("OMAHOYSETUP_WIDTH")) || 900
        implicitHeight: Number(Quickshell.env("OMAHOYSETUP_HEIGHT")) || 680
        color: app.theme.background

        Item {
            id: surface
            anchors.fill: parent
            focus: true
            Keys.onPressed: e => app.key(e)

            Column {
                id: side
                anchors { left: parent.left; top: parent.top; bottom: parent.bottom; margins: 16 }
                width: 190
                spacing: 6
                Words {
                    theme: app.theme
                    text: "OMAHOY SETUP"
                    color: app.theme.accent
                    font.bold: true
                    font.pixelSize: app.theme.baseSize - 1
                }
                Item { width: 1; height: 8 }
                Repeater {
                    model: app.setup.stepList
                    delegate: Rectangle {
                        id: entry
                        required property var modelData
                        required property int index
                        readonly property bool current: app.setup.step === modelData.id
                        readonly property int problems: index <= app.setup.reached ? app.setup.problemCount(modelData.id) : 0
                        width: side.width
                        height: app.theme.baseSize + 14
                        color: current ? Qt.alpha(app.theme.accent, 0.18) : "transparent"
                        Words {
                            anchors { left: parent.left; leftMargin: 8; right: parent.right; rightMargin: 8; verticalCenter: parent.verticalCenter }
                            theme: app.theme
                            width: undefined
                            elide: Text.ElideRight
                            wrapMode: Text.NoWrap
                            text: (entry.index + 1) + "  " + entry.modelData.label + (entry.problems ? "  !" : "")
                            bad: entry.problems > 0
                            font.bold: entry.current
                        }
                        MouseArea {
                            anchors.fill: parent
                            onClicked: app.setup.goTo(entry.modelData.id)
                        }
                    }
                }
            }

            Rectangle {
                anchors { left: side.right; leftMargin: 8; top: parent.top; bottom: parent.bottom; topMargin: 16; bottomMargin: 16 }
                width: 1
                color: Qt.alpha(app.theme.foreground, 0.18)
            }

            Words {
                id: loadError
                anchors { left: side.right; right: parent.right; top: parent.top; margins: 28 }
                theme: app.theme
                visible: app.setup.loadError !== ""
                text: app.setup.loadError
                bad: true
            }

            Flickable {
                id: flick
                anchors { left: side.right; right: parent.right; top: parent.top; bottom: footer.top; leftMargin: 28; rightMargin: 20; topMargin: 16 }
                visible: app.setup.loaded
                clip: true
                contentWidth: width
                contentHeight: pane.implicitHeight + 20
                boundsBehavior: Flickable.StopAtBounds

                Loader {
                    id: pane
                    width: flick.width - 12
                    sourceComponent: {
                        switch (app.setup.step) {
                        case "hardware": return hardwareStep;
                        case "home": return homeStep;
                        case "apps": return appsStep;
                        case "logbook": return logbookStep;
                        case "review": return reviewStep;
                        default: return boatStep;
                        }
                    }
                }
                Component { id: boatStep; BoatStep { s: app.setup } }
                Component { id: hardwareStep; HardwareStep { s: app.setup } }
                Component { id: homeStep; HomeStep { s: app.setup } }
                Component { id: appsStep; AppsStep { s: app.setup } }
                Component { id: logbookStep; LogbookStep { s: app.setup } }
                Component { id: reviewStep; ReviewStep { s: app.setup; onJump: id => app.setup.goTo(id) } }
            }

            Rectangle {
                id: footer
                anchors { left: side.right; right: parent.right; bottom: parent.bottom }
                height: app.theme.baseSize + 36
                color: app.theme.background
                Rectangle { anchors { left: parent.left; right: parent.right; top: parent.top } height: 1; color: Qt.alpha(app.theme.foreground, 0.18) }
                Btn {
                    id: backButton
                    anchors { left: parent.left; leftMargin: 28; verticalCenter: parent.verticalCenter }
                    theme: app.theme
                    text: "Back"
                    enabled: app.setup.stepIndex > 0 && !app.setup.applying
                    onClicked: app.setup.back()
                }
                Words {
                    anchors { left: backButton.right; leftMargin: 14; right: nextButton.left; rightMargin: 14; verticalCenter: parent.verticalCenter }
                    theme: app.theme
                    width: undefined
                    elide: Text.ElideRight
                    wrapMode: Text.NoWrap
                    dim: true
                    text: app.setup.closeRefused ? "Setup is working and can't be closed until it finishes."
                        : app.setup.busy ? "Working..." : app.setup.checking ? "Checking..."
                        : app.onReview && !app.setup.complete ? "Fix the problems above to go on." : ""
                }
                Btn {
                    id: nextButton
                    anchors { right: parent.right; rightMargin: 20; verticalCenter: parent.verticalCenter }
                    theme: app.theme
                    primary: true
                    text: app.lastStep ? "Go" : "Next"
                    enabled: app.lastStep
                        ? app.setup.ready && !app.setup.busy && app.setup.applyState !== "done"
                        : true
                    onClicked: app.advance()
                }
            }
        }
    }

    Component.onCompleted: {
        if (standalone) setup.begin(Quickshell.env("OMAHOY_BIN") || "");
        Qt.callLater(() => surface.forceActiveFocus());
    }
}
