import QtQuick

// Text in the theme's colors and font, wrapped to the width it is given.
Text {
    id: words
    property Theme theme: Theme {}
    property bool dim: false
    property bool bad: false
    color: bad ? theme.red : theme.foreground
    opacity: dim ? 0.65 : 1
    font.family: theme.font
    font.pixelSize: theme.baseSize
    wrapMode: Text.WordWrap
    width: parent ? parent.width : implicitWidth
}
