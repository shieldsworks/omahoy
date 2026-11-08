pragma ComponentBehavior: Bound
import QtQuick

// Step 2: GPS and AIS receivers. Each device, and each network source, is
// Used, Kept as the boat has it, or skipped for now; omahoy says what the
// probe heard.
Column {
    id: step
    required property Setup s
    property string netText: ""
    width: parent ? parent.width : 400
    spacing: 10

    component Row2: Rectangle {
        id: row2
        default property alias content: inner.data
        property alias spacingInner: inner.spacing
        width: parent ? parent.width : 400
        height: inner.implicitHeight + 16
        color: Qt.alpha(step.s.theme.foreground, 0.05)
        Column {
            id: inner
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: 8 }
            spacing: 6
        }
    }

    Words { theme: step.s.theme; text: "Hardware"; font.bold: true; font.pixelSize: step.s.theme.baseSize + 4 }
    Words {
        theme: step.s.theme
        text: "Plug in your GPS and AIS receivers, then Listen."
    }
    Row {
        spacing: 10
        Btn {
            theme: step.s.theme
            text: step.s.probed ? "Listen again" : "Listen"
            primary: !step.s.probed
            enabled: !step.s.busy
            onClicked: step.s.listen()
        }
        Btn {
            theme: step.s.theme
            text: "Skip for now"
            enabled: !step.s.busy
            onClicked: step.s.skipHardware()
        }
    }
    Words {
        theme: step.s.theme
        visible: step.s.busy
        text: "Listening. This can take up to 30 seconds per device, and omakeel may need building first.\n" + step.s.progress
        dim: true
    }
    Words { theme: step.s.theme; visible: step.s.hardwareError !== ""; text: step.s.hardwareError; bad: true }
    Words {
        theme: step.s.theme
        visible: step.s.needLogout
        text: "You were added to the uucp group. Log out and back in once so the hub can open the ports on its own."
        dim: true
    }

    // One row per source: the devices the probe heard, the boat's own
    // sources, and the network sources tested.
    Repeater {
        model: step.s.hardware
        delegate: Row2 {
            id: dev
            required property var modelData
            readonly property var row: modelData.row
            readonly property bool offers: !!modelData.offered && modelData.offered !== modelData.boat
            Words { theme: step.s.theme; text: dev.row ? dev.row.name : dev.modelData.boat; font.bold: true }
            Words {
                theme: step.s.theme
                text: dev.row ? dev.row.verdict + ": " + dev.row.describe
                    : dev.modelData.boat ? "In this boat now, but not heard" + (dev.modelData.offered ? " on the last listen" : "")
                    : "Not heard on the last listen"
                dim: true
            }
            Row {
                spacing: 8
                visible: !!dev.modelData.boat || !!dev.modelData.offered
                Btn {
                    theme: step.s.theme
                    visible: !!dev.modelData.boat
                    text: dev.offers ? "Keep " + dev.modelData.boat : "Keep"
                    primary: dev.modelData.choice === "keep"
                    onClicked: step.s.choose(dev.modelData.key, "keep")
                }
                Btn {
                    theme: step.s.theme
                    visible: dev.offers
                    text: dev.row && dev.row.verdict === "silent" ? "Use as AIS at 38400"
                        : dev.row && dev.row.verdict !== "ok" ? "Use anyway"
                        : dev.modelData.boat ? "Use " + dev.modelData.offered : "Use"
                    primary: dev.modelData.choice === "use"
                    onClicked: step.s.choose(dev.modelData.key, "use")
                }
                Btn {
                    theme: step.s.theme
                    text: "Skip for now"
                    primary: dev.modelData.choice === "skip"
                    onClicked: step.s.choose(dev.modelData.key, "skip")
                }
            }
            Btn {
                theme: step.s.theme
                visible: !!dev.row && dev.row.verdict === "denied"
                text: "Allow serial ports"
                enabled: !step.s.busy
                onClicked: step.s.allowSerial()
            }
        }
    }

    Field {
        theme: step.s.theme
        label: "Network source (host:port), such as a phone app sharing NMEA. Phone apps share only while they're open."
        hint: "192.168.1.20:10110"
        text: step.netText
        onEdited: t => step.netText = t
    }
    Btn {
        theme: step.s.theme
        text: "Test"
        enabled: !step.s.busy && step.netText.trim() !== ""
        onClicked: step.s.testNet(step.netText)
    }

    Words {
        theme: step.s.theme
        visible: step.s.problemsFor("source").length > 0
        text: step.s.problemsFor("source").join("; ")
        bad: true
    }
    Words {
        theme: step.s.theme
        text: step.s.onDemo
            ? "Nothing is chosen, so the hub will play the demo sail with invented ships until you add hardware."
            : "The hub will listen to: " + step.s.sources().join(", ")
        dim: true
    }
}
