import QtQuick

// A button that takes the keyboard (Tab to reach it, Return or Space to
// press it) as well as the mouse.
Rectangle {
    id: btn
    property Theme theme: Theme {}
    property string text: ""
    property bool primary: false
    signal clicked()

    activeFocusOnTab: true
    opacity: enabled ? 1 : 0.4
    implicitWidth: label.implicitWidth + 28
    implicitHeight: theme.baseSize + 16
    color: primary ? theme.accent : (mouse.containsMouse ? Qt.alpha(theme.foreground, 0.1) : "transparent")
    border.width: 1
    border.color: activeFocus ? theme.accent : Qt.alpha(theme.foreground, 0.3)

    Keys.onPressed: e => {
        if (e.key === Qt.Key_Return || e.key === Qt.Key_Enter || e.key === Qt.Key_Space) {
            btn.clicked();
            e.accepted = true;
        }
    }

    Text {
        id: label
        anchors.centerIn: parent
        text: btn.text
        color: btn.primary ? btn.theme.background : btn.theme.foreground
        font.family: btn.theme.font
        font.pixelSize: btn.theme.baseSize
        font.bold: btn.primary
    }
    MouseArea {
        id: mouse
        anchors.fill: parent
        hoverEnabled: true
        onClicked: {
            btn.forceActiveFocus();
            btn.clicked();
        }
    }
}
