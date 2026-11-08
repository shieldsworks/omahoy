import QtQuick

// A labelled one-line text field with the problems omahoy found on it.
Item {
    id: field
    property Theme theme: Theme {}
    property string label: ""
    property string text: ""
    property string hint: ""
    property var problems: []
    signal edited(string text)

    width: parent ? parent.width : 200
    implicitHeight: col.implicitHeight

    Column {
        id: col
        width: parent.width
        spacing: 4
        Words {
            theme: field.theme
            text: field.label
            dim: true
            font.pixelSize: field.theme.baseSize - 1
        }
        Rectangle {
            width: parent.width
            height: field.theme.baseSize + 16
            color: Qt.alpha(field.theme.foreground, 0.06)
            border.width: 1
            border.color: input.activeFocus ? field.theme.accent
                : field.problems.length ? field.theme.red : Qt.alpha(field.theme.foreground, 0.25)
            TextInput {
                id: input
                anchors { fill: parent; leftMargin: 8; rightMargin: 8 }
                verticalAlignment: TextInput.AlignVCenter
                color: field.theme.foreground
                selectionColor: field.theme.accent
                selectedTextColor: field.theme.background
                font.family: field.theme.font
                font.pixelSize: field.theme.baseSize
                clip: true
                activeFocusOnTab: true
                onTextEdited: field.edited(text)
                // Ctrl+Return goes on in the window, so it passes through.
                Keys.onReturnPressed: e => {
                    if (e.modifiers & Qt.ControlModifier) e.accepted = false;
                    else nextItemInFocusChain().forceActiveFocus();
                }
                Binding on text { value: field.text }
                Text {
                    anchors.fill: parent
                    verticalAlignment: Text.AlignVCenter
                    visible: input.text === "" && field.hint !== ""
                    text: field.hint
                    color: field.theme.foreground
                    opacity: 0.4
                    font: input.font
                }
            }
        }
        Words {
            theme: field.theme
            visible: field.problems.length > 0
            text: field.problems.join("; ")
            bad: true
            font.pixelSize: field.theme.baseSize - 1
        }
    }
}
