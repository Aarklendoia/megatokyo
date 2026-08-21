import QtQuick

// Color/type tokens shared by every screen, matching the mockup reviewed
// with the user before this was built. Instantiated per-file (no qmldir
// singleton registration — see Api.qml's doc comment on staying
// dependency-free) since it only ever holds constants.
QtObject {
    readonly property color ink: "#121319"
    readonly property color panel: "#1a1c26"
    readonly property color panelRaised: "#232634"
    readonly property color panelSunken: "#0d0e13"
    readonly property color line: "#34384a"
    readonly property color lineSoft: "#262a38"
    readonly property color text: "#eef0f6"
    readonly property color textDim: "#9297ab"
    readonly property color textFaint: "#62667a"
    readonly property color teal: "#5eead4"
    readonly property color tealDim: "#2c554e"
    readonly property color red: "#ff5470"

    readonly property string fontDisplay: "Sans Serif"
    readonly property string fontBody: "Sans Serif"
    readonly property string fontMono: "Monospace"
}
