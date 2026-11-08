pragma ComponentBehavior: Bound
import QtQuick

// Step 5, only when the logbook is chosen: where it lives.
Column {
    id: step
    required property Setup s
    width: parent ? parent.width : 400
    spacing: 14

    Words { theme: step.s.theme; text: "Logbook"; font.bold: true; font.pixelSize: step.s.theme.baseSize + 4 }
    Field {
        theme: step.s.theme
        label: "Logbook folder"
        text: step.s.vault
        problems: step.s.problemsFor("vault")
        onEdited: t => step.s.edit("vault", t)
    }
    Tick {
        theme: step.s.theme
        title: "Keep the logbook in git"
        checked: step.s.keepGit
        onToggled: step.s.edit("git", !step.s.keepGit)
    }
}
