import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// Settings screen: this daemon's own connection details (read-only,
// copyable — for pasting into another device's "remote daemon" section
// below), a remote-daemon override, DeepL translation + this daemon's poll
// interval (daemon-side, GET/POST /config — see daemon::control), and this
// GUI's own notifications toggle + background poll interval (gui-side,
// GET/POST /gui-config via guiCtrlApi — see gui::control).
Item {
    id: root

    property var api: null
    property var guiCtrlApi: null
    property string localBaseUrl: ""
    property string localToken: ""

    readonly property var theme: Theme {}

    property string remoteBaseUrl: ""
    property string remoteApiToken: ""
    property bool notificationsEnabled: true
    property int guiPollIntervalMinutes: 15

    property string deeplApiKey: ""
    property int daemonPollIntervalMinutes: 15

    property string remoteStatus: ""
    property bool remoteStatusIsError: false
    property string notificationsStatus: ""
    property bool notificationsStatusIsError: false
    property string daemonStatus: ""
    property bool daemonStatusIsError: false

    function loadRemoteSettings() {
        if (!guiCtrlApi)
            return
        guiCtrlApi.get("/gui-config", function (body) {
            if (!body)
                return
            root.remoteBaseUrl = body.remote_base_url || ""
            root.remoteApiToken = body.remote_api_token || ""
            root.notificationsEnabled = body.notifications_enabled !== false
            root.guiPollIntervalMinutes = body.poll_interval_minutes || 15
        })
    }

    function loadDaemonSettings() {
        if (!api)
            return
        api.get("/config", function (body) {
            if (!body)
                return
            root.deeplApiKey = body.deepl_api_key || ""
            root.daemonPollIntervalMinutes = body.poll_interval_minutes || 15
        })
    }

    onGuiCtrlApiChanged: loadRemoteSettings()
    onApiChanged: loadDaemonSettings()
    Component.onCompleted: {
        loadRemoteSettings()
        loadDaemonSettings()
    }

    function saveRemote() {
        root.remoteStatus = ""
        guiCtrlApi.post(
            "/gui-config?remote_base_url=" + encodeURIComponent(remoteUrlField.text)
            + "&remote_api_token=" + encodeURIComponent(remoteTokenField.text),
            function () { root.remoteStatusIsError = false; root.remoteStatus = I18n.tr("settings.saved") },
            function () { root.remoteStatusIsError = true; root.remoteStatus = I18n.tr("settings.saveFailed") }
        )
    }

    function saveNotifications() {
        root.notificationsStatus = ""
        guiCtrlApi.post(
            "/gui-config?notifications_enabled=" + (notificationsCheck.checked ? "true" : "false")
            + "&poll_interval_minutes=" + guiPollField.text,
            function () { root.notificationsStatusIsError = false; root.notificationsStatus = I18n.tr("settings.saved") },
            function () { root.notificationsStatusIsError = true; root.notificationsStatus = I18n.tr("settings.saveFailed") }
        )
    }

    function saveDaemon() {
        root.daemonStatus = ""
        api.post(
            "/config?deepl_api_key=" + encodeURIComponent(deeplField.text)
            + "&poll_interval_minutes=" + daemonPollField.text,
            function () { root.daemonStatusIsError = false; root.daemonStatus = I18n.tr("settings.saved") },
            function () { root.daemonStatusIsError = true; root.daemonStatus = I18n.tr("settings.saveFailed") }
        )
    }

    function copyField(field) {
        field.selectAll()
        field.copy()
        field.deselect()
    }

    ScrollView {
        anchors.fill: parent
        contentWidth: availableWidth

        ColumnLayout {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.margins: 28
            spacing: 20

            Label {
                text: I18n.tr("settings.title")
                font.family: theme.fontDisplay
                font.pixelSize: 20
                font.bold: true
                color: theme.text
            }

            // -- this daemon ------------------------------------------------
            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: thisDaemonColumn.implicitHeight + 32
                radius: 12
                color: theme.panelRaised
                border.color: theme.lineSoft
                border.width: 1

                ColumnLayout {
                    id: thisDaemonColumn
                    // Left/right/top only, not anchors.fill: the Rectangle's
                    // own height is derived FROM this column's
                    // implicitHeight (below) — anchoring height too would
                    // feed that height straight back in, which leaves a
                    // wrapping Label's second line clipped since the
                    // Rectangle never grows to fit it.
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.margins: 16
                    spacing: 10

                    Label {
                        text: I18n.tr("settings.thisDaemonTitle")
                        color: theme.teal
                        font.family: theme.fontMono
                        font.pixelSize: 10
                    }
                    Label {
                        text: I18n.tr("settings.thisDaemonHint")
                        color: theme.textDim
                        font.pixelSize: 12
                        wrapMode: Text.WordWrap
                        Layout.fillWidth: true
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8
                        Rectangle {
                            Layout.fillWidth: true
                            Layout.preferredHeight: 36
                            radius: 8
                            color: theme.panelSunken
                            border.color: theme.line
                            border.width: 1
                            TextField {
                                id: localUrlField
                                anchors.fill: parent
                                anchors.margins: 6
                                background: null
                                color: theme.textDim
                                font.family: theme.fontMono
                                font.pixelSize: 12
                                readOnly: true
                                selectByMouse: true
                                text: root.localBaseUrl
                            }
                        }
                        Button {
                            text: I18n.tr("settings.copy")
                            onClicked: root.copyField(localUrlField)
                        }
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8
                        Rectangle {
                            Layout.fillWidth: true
                            Layout.preferredHeight: 36
                            radius: 8
                            color: theme.panelSunken
                            border.color: theme.line
                            border.width: 1
                            TextField {
                                id: localTokenField
                                anchors.fill: parent
                                anchors.margins: 6
                                background: null
                                color: theme.textDim
                                font.family: theme.fontMono
                                font.pixelSize: 12
                                readOnly: true
                                selectByMouse: true
                                echoMode: TextInput.Password
                                text: root.localToken
                            }
                        }
                        Button {
                            text: I18n.tr("settings.copy")
                            onClicked: root.copyField(localTokenField)
                        }
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        Label {
                            text: I18n.tr("settings.daemonPollIntervalLabel")
                            color: theme.textDim
                            font.pixelSize: 12
                        }
                        Rectangle {
                            Layout.preferredWidth: 70
                            Layout.preferredHeight: 32
                            radius: 8
                            color: theme.panelSunken
                            border.color: theme.line
                            border.width: 1
                            TextField {
                                id: daemonPollField
                                anchors.fill: parent
                                anchors.margins: 6
                                background: null
                                color: theme.text
                                font.pixelSize: 12
                                selectByMouse: true
                                validator: IntValidator { bottom: 1 }
                                text: root.daemonPollIntervalMinutes
                            }
                        }
                        Label {
                            text: I18n.tr("settings.deeplKeyLabel")
                            color: theme.textDim
                            font.pixelSize: 12
                        }
                        Rectangle {
                            Layout.fillWidth: true
                            Layout.preferredHeight: 32
                            radius: 8
                            color: theme.panelSunken
                            border.color: theme.line
                            border.width: 1
                            TextField {
                                id: deeplField
                                anchors.fill: parent
                                anchors.margins: 6
                                background: null
                                color: theme.text
                                font.family: theme.fontMono
                                font.pixelSize: 12
                                selectByMouse: true
                                echoMode: TextInput.Password
                                placeholderText: I18n.tr("settings.deeplKeyPlaceholder")
                                placeholderTextColor: theme.textFaint
                                text: root.deeplApiKey
                            }
                        }
                        Button {
                            id: daemonSaveButton
                            text: I18n.tr("settings.save")
                            onClicked: root.saveDaemon()
                            background: Rectangle { color: theme.teal; radius: 8 }
                            contentItem: Label {
                                text: daemonSaveButton.text
                                color: theme.ink
                                font.bold: true
                                font.pixelSize: 12
                                horizontalAlignment: Text.AlignHCenter
                            }
                        }
                        Label {
                            text: root.daemonStatus
                            color: root.daemonStatusIsError ? theme.red : theme.teal
                            font.pixelSize: 11
                        }
                    }
                }
            }

            // -- remote daemon ------------------------------------------------
            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: remoteColumn.implicitHeight + 32
                radius: 12
                color: theme.panelRaised
                border.color: theme.lineSoft
                border.width: 1

                ColumnLayout {
                    id: remoteColumn
                    // See thisDaemonColumn's own comment on why this isn't
                    // anchors.fill.
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.margins: 16
                    spacing: 10

                    Label {
                        text: I18n.tr("settings.remoteTitle")
                        color: theme.teal
                        font.family: theme.fontMono
                        font.pixelSize: 10
                    }
                    Label {
                        text: I18n.tr("settings.remoteHint")
                        color: theme.textDim
                        font.pixelSize: 12
                        wrapMode: Text.WordWrap
                        Layout.fillWidth: true
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 36
                        radius: 8
                        color: theme.panelSunken
                        border.color: theme.line
                        border.width: 1
                        TextField {
                            id: remoteUrlField
                            anchors.fill: parent
                            anchors.margins: 6
                            background: null
                            color: theme.text
                            font.family: theme.fontMono
                            font.pixelSize: 12
                            selectByMouse: true
                            placeholderText: I18n.tr("settings.remoteUrlPlaceholder")
                            placeholderTextColor: theme.textFaint
                            text: root.remoteBaseUrl
                        }
                    }
                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 36
                        radius: 8
                        color: theme.panelSunken
                        border.color: theme.line
                        border.width: 1
                        TextField {
                            id: remoteTokenField
                            anchors.fill: parent
                            anchors.margins: 6
                            background: null
                            color: theme.text
                            font.family: theme.fontMono
                            font.pixelSize: 12
                            selectByMouse: true
                            echoMode: TextInput.Password
                            placeholderText: I18n.tr("settings.remoteTokenPlaceholder")
                            placeholderTextColor: theme.textFaint
                            text: root.remoteApiToken
                        }
                    }
                    RowLayout {
                        spacing: 10
                        Button {
                            id: remoteSaveButton
                            text: I18n.tr("settings.save")
                            onClicked: root.saveRemote()
                            background: Rectangle { color: theme.teal; radius: 8 }
                            contentItem: Label {
                                text: remoteSaveButton.text
                                color: theme.ink
                                font.bold: true
                                font.pixelSize: 12
                                horizontalAlignment: Text.AlignHCenter
                            }
                        }
                        Label {
                            text: root.remoteStatus
                            color: root.remoteStatusIsError ? theme.red : theme.teal
                            font.pixelSize: 11
                        }
                    }
                }
            }

            // -- notifications ------------------------------------------------
            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: notifColumn.implicitHeight + 32
                radius: 12
                color: theme.panelRaised
                border.color: theme.lineSoft
                border.width: 1

                ColumnLayout {
                    id: notifColumn
                    // See thisDaemonColumn's own comment on why this isn't
                    // anchors.fill.
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.margins: 16
                    spacing: 10

                    Label {
                        text: I18n.tr("settings.notificationsTitle")
                        color: theme.teal
                        font.family: theme.fontMono
                        font.pixelSize: 10
                    }

                    RowLayout {
                        spacing: 16

                        CheckBox {
                            id: notificationsCheck
                            text: I18n.tr("settings.notificationsToggle")
                            checked: root.notificationsEnabled
                            contentItem: Label {
                                text: notificationsCheck.text
                                color: theme.text
                                font.pixelSize: 12
                                leftPadding: notificationsCheck.indicator.width + 6
                                verticalAlignment: Text.AlignVCenter
                            }
                        }

                        Label {
                            text: I18n.tr("settings.notificationsPollIntervalLabel")
                            color: theme.textDim
                            font.pixelSize: 12
                        }
                        Rectangle {
                            Layout.preferredWidth: 70
                            Layout.preferredHeight: 32
                            radius: 8
                            color: theme.panelSunken
                            border.color: theme.line
                            border.width: 1
                            TextField {
                                id: guiPollField
                                anchors.fill: parent
                                anchors.margins: 6
                                background: null
                                color: theme.text
                                font.pixelSize: 12
                                selectByMouse: true
                                validator: IntValidator { bottom: 1 }
                                text: root.guiPollIntervalMinutes
                            }
                        }

                        Button {
                            id: notifSaveButton
                            text: I18n.tr("settings.save")
                            onClicked: root.saveNotifications()
                            background: Rectangle { color: theme.teal; radius: 8 }
                            contentItem: Label {
                                text: notifSaveButton.text
                                color: theme.ink
                                font.bold: true
                                font.pixelSize: 12
                                horizontalAlignment: Text.AlignHCenter
                            }
                        }
                        Label {
                            text: root.notificationsStatus
                            color: root.notificationsStatusIsError ? theme.red : theme.teal
                            font.pixelSize: 11
                        }
                    }
                }
            }

            Item { Layout.preferredHeight: 8 }
        }
    }
}
