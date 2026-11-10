pragma ComponentBehavior: Bound
import QtQuick

// Step 3: the home port, and the sailing area that follows it.
Column {
    id: step
    required property Setup s
    readonly property var report: s.report
    readonly property var cover: report ? report.coverage : null
    width: parent ? parent.width : 400
    spacing: 14

    Words { theme: step.s.theme; text: "Home and area"; font.bold: true; font.pixelSize: step.s.theme.baseSize + 4 }
    Field {
        theme: step.s.theme
        label: "Home port, as latitude, longitude"
        hint: "37.8667, -122.3134"
        text: step.s.home
        problems: step.s.problemsFor("home")
        onEdited: t => step.s.edit("home", t)
    }
    Words {
        theme: step.s.theme
        visible: step.report !== null && step.report.answers.home !== null
        text: step.report && step.report.answers.home ? "Read as " + step.report.answers.home.show : ""
        dim: true
    }
    Words {
        theme: step.s.theme
        visible: step.s.area !== null
        text: step.s.area ? "Sailing area: " + step.s.area.south + " to " + step.s.area.north + " N, "
            + step.s.area.west + " to " + step.s.area.east + " E. It follows your home port." : ""
    }
    Words {
        theme: step.s.theme
        visible: step.s.problemsFor("area").length > 0
        text: step.s.problemsFor("area").join("; ")
        bad: true
    }
    Column {
        width: parent.width
        spacing: 4
        visible: step.cover !== null
        Words { theme: step.s.theme; text: (step.cover && step.cover.noaa ? "✓" : "✗") + "  NOAA charts" }
        Words { theme: step.s.theme; text: (step.cover && step.cover.noaa ? "✓" : "✗") + "  NOAA tides" }
        Words { theme: step.s.theme; text: (step.cover && step.cover.hrrr ? "✓" : "✗") + "  HRRR wind" }
        Words {
            theme: step.s.theme
            visible: step.cover !== null && !(step.cover.noaa && step.cover.hrrr)
            text: "NOAA and HRRR cover US waters only."
            dim: true
        }
    }
}
