pragma ComponentBehavior: Bound
import QtQuick

// Step 4: which apps to install. omahoy says which can't work in the area.
Column {
    id: step
    required property Setup s
    width: parent ? parent.width : 400
    spacing: 8

    Words { theme: step.s.theme; text: "Apps"; font.bold: true; font.pixelSize: step.s.theme.baseSize + 4 }
    Words {
        theme: step.s.theme
        visible: step.s.report === null
        text: "Waiting for omahoy..."
        dim: true
    }
    Repeater {
        model: step.s.report ? step.s.report.apps : []
        delegate: Tick {
            required property var modelData
            theme: step.s.theme
            title: modelData.name + "  " + modelData.what
            detail: modelData.caveat || ""
            reason: modelData.refusal || ""
            checked: step.s.apps.indexOf(modelData.short) >= 0
            onToggled: step.s.toggleApp(modelData.short)
        }
    }
    Words {
        theme: step.s.theme
        visible: step.s.problemsFor("apps").length > 0
        text: step.s.problemsFor("apps").join("; ")
        bad: true
    }
}
