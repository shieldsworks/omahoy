pragma ComponentBehavior: Bound
import QtQuick

// The last step: what apply will do, the problems that stop it, and the
// progress of the apply itself.
Column {
    id: step
    required property Setup s
    readonly property var problems: s.report ? s.report.problems : []
    signal jump(string id)
    width: parent ? parent.width : 400
    spacing: 8

    Words { theme: step.s.theme; text: "Review"; font.bold: true; font.pixelSize: step.s.theme.baseSize + 4 }

    Words {
        theme: step.s.theme
        visible: step.s.onDemo
        text: "No hardware is chosen, so the hub will play the demo sail with invented ships until you add hardware."
        dim: true
    }

    Repeater {
        model: Object.keys(step.s.refused)
        delegate: Words {
            required property string modelData
            theme: step.s.theme
            text: step.s.refused[modelData]
            bad: true
        }
    }
    Repeater {
        model: step.problems
        delegate: Item {
            id: prob
            required property var modelData
            width: step.width
            height: Math.max(msg.implicitHeight, link.implicitHeight) + 4
            Words {
                id: msg
                theme: step.s.theme
                width: parent.width - link.width - 12
                text: (prob.modelData.key ? prob.modelData.key + ": " : "") + prob.modelData.message
                bad: true
            }
            Btn {
                id: link
                anchors.right: parent.right
                theme: step.s.theme
                text: "Fix"
                onClicked: step.jump(step.s.stepOf(prob.modelData.key))
            }
        }
    }

    Words { theme: step.s.theme; visible: step.s.planning; text: "Working out the plan..."; dim: true }
    Words { theme: step.s.theme; visible: step.s.planError !== ""; text: step.s.planError; bad: true }
    Repeater {
        model: step.s.plan && step.problems.length === 0 ? step.s.plan.steps : []
        delegate: Words {
            required property string modelData
            theme: step.s.theme
            text: "•  " + modelData
        }
    }
    Repeater {
        model: step.s.plan && step.problems.length === 0 ? step.s.plan.findings : []
        delegate: Words {
            required property string modelData
            theme: step.s.theme
            text: "Note: " + modelData
            dim: true
        }
    }

    Words {
        theme: step.s.theme
        visible: step.s.applyState !== ""
        text: step.s.applyState === "running" ? "Applying..."
            : step.s.applyState === "done" ? "Done."
            : step.s.applyState === "needs" ? "Needs you: the lines below say what is left."
            : "Failed."
        bad: step.s.applyState === "failed"
        font.bold: true
    }
    Repeater {
        model: step.s.applyLog
        delegate: Words {
            required property string modelData
            theme: step.s.theme
            text: modelData
            font.family: "monospace"
            dim: true
        }
    }
    Words { theme: step.s.theme; visible: step.s.applyError !== ""; text: step.s.applyError; bad: true }
    Btn {
        theme: step.s.theme
        visible: step.s.applyState === "done" && step.s.apps.indexOf("helm") >= 0
        text: "Open the chart"
        primary: true
        onClicked: step.s.openChart()
    }
}
