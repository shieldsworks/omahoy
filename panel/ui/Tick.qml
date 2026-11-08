import QtQuick

// A checkbox with a title and a line of explanation. `reason` disables it
// and says why.
Item {
    id: tick
    property Theme theme: Theme {}
    property bool checked: false
    property string title: ""
    property string detail: ""
    property string reason: ""
    signal toggled()

    width: parent ? parent.width : 200
    implicitHeight: Math.max(box.height, texts.implicitHeight) + 10
    activeFocusOnTab: tick.reason === ""
    opacity: reason === "" ? 1 : 0.55

    Keys.onPressed: e => {
        if (e.key === Qt.Key_Space || e.key === Qt.Key_Return || e.key === Qt.Key_Enter) {
            if (tick.reason === "") tick.toggled();
            e.accepted = true;
        }
    }

    Rectangle {
        id: box
        x: 0
        y: 3
        width: tick.theme.baseSize + 4
        height: width
        color: tick.checked && tick.reason === "" ? tick.theme.accent : "transparent"
        border.width: 1
        border.color: tick.activeFocus ? tick.theme.accent : Qt.alpha(tick.theme.foreground, 0.4)
        Text {
            anchors.centerIn: parent
            visible: tick.checked && tick.reason === ""
            text: "✓"
            color: tick.theme.background
            font.pixelSize: tick.theme.baseSize
        }
    }
    Column {
        id: texts
        anchors { left: box.right; leftMargin: 10; right: parent.right; top: parent.top; topMargin: 2 }
        spacing: 2
        Words {
            theme: tick.theme
            text: tick.title
            font.bold: true
        }
        Words {
            theme: tick.theme
            visible: text !== ""
            text: tick.detail
            dim: true
            font.pixelSize: tick.theme.baseSize - 1
        }
        Words {
            theme: tick.theme
            visible: tick.reason !== ""
            text: tick.reason
            bad: true
            font.pixelSize: tick.theme.baseSize - 1
        }
    }
    MouseArea {
        anchors.fill: parent
        enabled: tick.reason === ""
        onClicked: {
            tick.forceActiveFocus();
            tick.toggled();
        }
    }
}
