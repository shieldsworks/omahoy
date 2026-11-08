import QtQuick
import Quickshell.Io

// One command at a time. Its stdin is `input`, written by printf in front
// of the command: a Process can write and close its stdin, but a pipe
// closes it the same way every time and needs no timing. Output arrives a
// line at a time on `lines` and `errLines` as it happens, and whole in
// `out` and `err` when the command exits. There is no stop: a probe stops
// the hub's service and starts it again at the end, and an apply changes the
// machine, so each runs to its end even if the window closes.
Item {
    id: runner

    property var argv: []
    property string input: ""
    readonly property bool running: proc.running
    property var lines: []
    property var errLines: []
    property int code: -1
    readonly property string out: lines.join("\n")
    readonly property string err: errLines.join("\n")

    // The exit code, once the last line has arrived.
    signal finished(int code)

    function start() {
        if (proc.running) return;
        lines = [];
        errLines = [];
        code = -1;
        proc.command = ["bash", "-c", "printf %s \"$1\" | \"${@:2}\"", "_", input].concat(argv);
        proc.running = true;
    }

    Process {
        id: proc
        stdout: SplitParser {
            onRead: data => runner.lines = runner.lines.concat([data])
        }
        stderr: SplitParser {
            onRead: data => runner.errLines = runner.errLines.concat([data])
        }
        onExited: (exitCode, exitStatus) => {
            runner.code = exitCode;
            settle.restart();
        }
    }

    // The last line can still be in flight when the exit is reported.
    Timer {
        id: settle
        interval: 40
        onTriggered: runner.finished(runner.code)
    }
}
