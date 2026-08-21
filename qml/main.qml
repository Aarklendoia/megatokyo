import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// Minimal placeholder: proves the launcher -> qml6 -> daemon API pipeline
// end to end (spawn, arguments, XHR, auth header). The real screens
// (strip gallery, rant reader with translation, settings) are tracked
// separately — see the project's open issues.
ApplicationWindow {
    id: window
    visible: true
    width: 480
    height: 360
    title: "Megatokyo"

    // Passed positionally after `--` by gui/src/launcher.rs, since QML
    // doesn't reliably read process environment variables.
    property string baseUrl: Qt.application.arguments.length > 1 ? Qt.application.arguments[Qt.application.arguments.length - 2] : ""
    property string apiToken: Qt.application.arguments.length > 1 ? Qt.application.arguments[Qt.application.arguments.length - 1] : ""

    property string statusText: "Connecting…"
    property int chapterCount: 0

    function refresh() {
        var statusRequest = new XMLHttpRequest()
        statusRequest.open("GET", baseUrl + "/status")
        statusRequest.setRequestHeader("x-megatokyo-daemon-token", apiToken)
        statusRequest.onreadystatechange = function () {
            if (statusRequest.readyState !== XMLHttpRequest.DONE)
                return
            if (statusRequest.status === 200) {
                var status = JSON.parse(statusRequest.responseText)
                statusText = status.backfilling
                    ? "Backfilling…"
                    : "Last strip #" + status.last_strip_number + ", last rant #" + status.last_rant_number
            } else {
                statusText = "Could not reach the daemon at " + baseUrl + " (HTTP " + statusRequest.status + ")"
            }
        }
        statusRequest.send()

        var chaptersRequest = new XMLHttpRequest()
        chaptersRequest.open("GET", baseUrl + "/chapters")
        chaptersRequest.setRequestHeader("x-megatokyo-daemon-token", apiToken)
        chaptersRequest.onreadystatechange = function () {
            if (chaptersRequest.readyState !== XMLHttpRequest.DONE)
                return
            if (chaptersRequest.status === 200)
                chapterCount = JSON.parse(chaptersRequest.responseText).length
        }
        chaptersRequest.send()
    }

    Component.onCompleted: refresh()

    ColumnLayout {
        anchors.centerIn: parent
        spacing: 12

        Label {
            text: "Megatokyo"
            font.pointSize: 20
            Layout.alignment: Qt.AlignHCenter
        }
        Label {
            text: statusText
            Layout.alignment: Qt.AlignHCenter
        }
        Label {
            text: chapterCount + " chapters known"
            Layout.alignment: Qt.AlignHCenter
        }
        Button {
            text: "Refresh"
            Layout.alignment: Qt.AlignHCenter
            onClicked: refresh()
        }
    }
}
