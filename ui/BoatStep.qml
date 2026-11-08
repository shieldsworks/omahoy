pragma ComponentBehavior: Bound
import QtQuick

// Step 1: the boat's name, depth units, draft and clearance.
Column {
    id: step
    required property Setup s
    width: parent ? parent.width : 400
    spacing: 14

    Words { theme: step.s.theme; text: "The boat"; font.bold: true; font.pixelSize: step.s.theme.baseSize + 4 }
    Field {
        theme: step.s.theme
        label: "Boat name"
        text: step.s.name
        problems: step.s.problemsFor("name")
        onEdited: t => step.s.edit("name", t)
    }
    Words { theme: step.s.theme; text: "Depth units"; dim: true; font.pixelSize: step.s.theme.baseSize - 1 }
    Row {
        spacing: 8
        Repeater {
            model: ["feet", "meters", "fathoms"]
            delegate: Btn {
                required property string modelData
                theme: step.s.theme
                text: modelData
                primary: step.s.units === modelData
                onClicked: step.s.edit("units", modelData)
            }
        }
    }
    Field {
        theme: step.s.theme
        label: "Draft (" + step.s.units + ")"
        text: step.s.draft
        problems: step.s.problemsFor("draft")
        onEdited: t => step.s.edit("draft", t)
    }
    Field {
        theme: step.s.theme
        label: "Clearance wanted under the keel (" + step.s.units + ")"
        text: step.s.clearance
        problems: step.s.problemsFor("clearance")
        onEdited: t => step.s.edit("clearance", t)
    }
    Words {
        theme: step.s.theme
        visible: step.s.report !== null && step.s.report.depths !== null
        text: step.s.report && step.s.report.depths
            ? "The chart shades water shallower than " + step.s.report.depths.safety + " " + step.s.units + "."
            : ""
        dim: true
    }
}
