pragma ComponentBehavior: Bound
import QtQuick

// Step 2: GPS and AIS receivers. Each device, and each network source, is
// Used or skipped for now; omahoy says what the probe heard.
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

    Repeater {
        model: step.s.devices
        delegate: Row2 {
            id: dev
            required property var modelData
            required property int index
            Words { theme: step.s.theme; text: dev.modelData.name; font.bold: true }
            Words { theme: step.s.theme; text: dev.modelData.verdict + ": " + dev.modelData.describe; dim: true }
            Row {
                spacing: 8
                visible: dev.modelData.verdict === "ok" || dev.modelData.verdict === "silent"
                Btn {
                    theme: step.s.theme
                    text: dev.modelData.verdict === "silent" ? "Use as AIS at 38400" : "Use"
                    primary: dev.modelData.choice === "use"
                    onClicked: step.s.chooseDevice(dev.index, "use")
                }
                Btn {
                    theme: step.s.theme
                    text: "Skip for now"
                    primary: dev.modelData.choice === "skip"
                    onClicked: step.s.chooseDevice(dev.index, "skip")
                }
            }
            Btn {
                theme: step.s.theme
                visible: dev.modelData.verdict === "denied"
                text: "Allow serial ports"
                enabled: !step.s.busy
                onClicked: step.s.allowSerial()
            }
        }
    }

    Words {
        theme: step.s.theme
        visible: step.s.held.length > 0
        text: "In this boat now, but not heard:"
        font.bold: true
    }
    Repeater {
        model: step.s.held
        delegate: Row2 {
            id: old
            required property var modelData
            required property int index
            Words { theme: step.s.theme; text: old.modelData.source }
            Row {
                spacing: 8
                Btn {
                    theme: step.s.theme
                    text: "Keep"
                    primary: old.modelData.keep
                    onClicked: step.s.chooseHeld(old.index, true)
                }
                Btn {
                    theme: step.s.theme
                    text: "Skip for now"
                    primary: !old.modelData.keep
                    onClicked: step.s.chooseHeld(old.index, false)
                }
            }
        }
    }

    Field {
        theme: step.s.theme
        label: "Network source (host:port), such as a phone app sharing NMEA"
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
    Row2 {
        visible: step.s.netProbe !== null
        Words { theme: step.s.theme; text: step.s.netProbe ? step.s.netProbe.spec : "" ; font.bold: true }
        Words { theme: step.s.theme; text: step.s.netProbe ? step.s.netProbe.verdict + ": " + step.s.netProbe.describe : ""; dim: true }
        Words {
            theme: step.s.theme
            text: "Phone apps share only while they're open."
            dim: true
        }
        Row {
            spacing: 8
            Btn {
                theme: step.s.theme
                text: step.s.netProbe && step.s.netProbe.verdict === "ok" ? "Use" : "Use anyway"
                enabled: step.s.netProbe !== null && !!step.s.netProbe.source
                onClicked: step.s.chooseNet("use")
            }
            Btn {
                theme: step.s.theme
                text: "Skip for now"
                onClicked: step.s.chooseNet("skip")
            }
        }
    }
    Repeater {
        model: step.s.nets
        delegate: Row2 {
            id: net
            required property var modelData
            Words { theme: step.s.theme; text: net.modelData.source + (net.modelData.choice === "use" ? "  (using)" : "  (skipped for now)") }
            Btn { theme: step.s.theme; text: "Remove"; onClicked: step.s.dropNet(net.modelData.spec) }
        }
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
