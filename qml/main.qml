import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// Shell: sidebar nav + the five screens reviewed with the user before this
// was built (Home/Reader/Gallery/Rants/Settings). QML talks to the daemon
// directly over its HTTP API (local or remote, see gui::daemon_link) for
// everything but this GUI's own config — base_url/token/guiRuntimeDir come
// in as positional arguments from gui::launcher, the latter pointing at the
// discovery files for gui::control's local control server (see
// loadGuiCtrl below), which Settings uses to write this GUI's own
// remote-daemon/notifications config back to disk.
ApplicationWindow {
    id: window
    visible: true
    width: 1180
    height: 760
    minimumWidth: 720
    minimumHeight: 480
    title: "Megatokyo"

    // gui::launcher passes base_url/token/runtime_dir as the last three
    // positional arguments — runtime_dir is where the local control
    // server's port/token discovery files live (see loadGuiCtrl below and
    // gui::control's own doc comment).
    property string baseUrl: Qt.application.arguments.length > 2 ? Qt.application.arguments[Qt.application.arguments.length - 3] : ""
    property string apiToken: Qt.application.arguments.length > 2 ? Qt.application.arguments[Qt.application.arguments.length - 2] : ""
    property string guiRuntimeDir: Qt.application.arguments.length > 2 ? Qt.application.arguments[Qt.application.arguments.length - 1] : ""
    property string guiCtrlPort: ""
    property string guiCtrlToken: ""

    property var chapters: []
    property var strips: []
    property var rants: []
    property var favorites: []
    property var status: ({})
    property int progressStrip: -1
    property bool deeplConfigured: false

    readonly property var theme: Theme {}

    Api {
        id: api
        baseUrl: window.baseUrl
        token: window.apiToken
    }

    GuiCtrlApi {
        id: guiCtrlApi
        baseUrl: "http://127.0.0.1:" + window.guiCtrlPort
        token: window.guiCtrlToken
    }

    // Synchronous, one-shot reads of the two discovery files
    // gui::launcher::run writes before spawning qml6 — same pattern as
    // kio-protondrive-wizard's own control-server discovery. Must happen
    // before guiCtrlApi is used for anything.
    function loadGuiCtrl() {
        if (window.guiRuntimeDir === "")
            return
        var portXhr = new XMLHttpRequest()
        portXhr.open("GET", "file://" + window.guiRuntimeDir + "/megatokyo-gui-ctrl.port", false)
        portXhr.send()
        if (portXhr.responseText !== "")
            window.guiCtrlPort = portXhr.responseText.trim()

        var tokenXhr = new XMLHttpRequest()
        tokenXhr.open("GET", "file://" + window.guiRuntimeDir + "/megatokyo-gui-ctrl.token", false)
        tokenXhr.send()
        if (tokenXhr.responseText !== "")
            window.guiCtrlToken = tokenXhr.responseText.trim()
    }

    function refreshChapters() {
        api.get("/chapters", function (body) { window.chapters = body || [] })
    }
    function refreshStrips() {
        api.get("/strips", function (body) { window.strips = body || [] })
    }
    function refreshRants() {
        api.get("/rants", function (body) { window.rants = body || [] })
    }
    function refreshFavorites() {
        api.get("/favorites", function (body) { window.favorites = body || [] })
    }
    function refreshStatus() {
        api.get("/status", function (body) { window.status = body || {} })
    }
    function refreshProgress() {
        api.get("/progress", function (body) {
            window.progressStrip = body && body.strip_number ? body.strip_number : -1
        })
    }
    // Only whether a key is set, not its value — RantsScreen just needs to
    // know whether to offer translation at all (see its own doc comment).
    // Re-called after Settings saves a new key so the Rants screen picks it
    // up immediately, no restart needed.
    function refreshDeeplConfigured() {
        api.get("/config", function (body) {
            window.deeplConfigured = !!(body && body.deepl_api_key)
        })
    }
    function refreshAll() {
        refreshChapters()
        refreshStrips()
        refreshRants()
        refreshFavorites()
        refreshStatus()
        refreshProgress()
        refreshDeeplConfigured()
    }

    function isFavorite(number) {
        return window.favorites.some(function (f) { return f.strip_number === number })
    }

    function toggleFavorite(number) {
        if (isFavorite(number))
            api.del("/favorites?number=" + number, refreshFavorites)
        else
            api.post("/favorites?number=" + number, refreshFavorites)
    }

    function saveProgress(number) {
        window.progressStrip = number
        api.post("/progress?number=" + number)
    }

    function openReader(number) {
        readerScreen.currentNumber = number
        nav.currentIndex = 1
    }

    function openRant(number) {
        rantsScreen.selectedNumber = number
        nav.currentIndex = 3
    }

    Component.onCompleted: {
        loadGuiCtrl()
        refreshAll()
    }

    background: Rectangle { color: theme.ink }

    RowLayout {
        anchors.fill: parent
        spacing: 0

        // -- sidebar ------------------------------------------------------
        Rectangle {
            Layout.preferredWidth: 208
            Layout.fillHeight: true
            color: theme.panelSunken

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 12
                spacing: 2

                Repeater {
                    id: nav
                    property int currentIndex: 0
                    model: [I18n.tr("app.navHome"), I18n.tr("app.navReader"), I18n.tr("app.navGallery"), I18n.tr("app.navRants"), I18n.tr("app.navSettings")]
                    delegate: Rectangle {
                        Layout.fillWidth: true
                        height: 36
                        radius: 9
                        color: nav.currentIndex === index ? theme.tealDim : "transparent"

                        Label {
                            anchors.left: parent.left
                            anchors.verticalCenter: parent.verticalCenter
                            anchors.leftMargin: 12
                            text: modelData
                            font.pixelSize: 13
                            color: nav.currentIndex === index ? theme.teal : theme.textDim
                        }
                        MouseArea {
                            anchors.fill: parent
                            onClicked: nav.currentIndex = index
                        }
                    }
                }

                Item { Layout.fillHeight: true }

                Label {
                    Layout.fillWidth: true
                    text: I18n.tr(window.status.backfilling ? "app.statusBackfilling" : "app.statusUpToDate")
                    font.family: theme.fontMono
                    font.pixelSize: 10
                    color: window.status.backfilling ? theme.textDim : theme.teal
                    elide: Text.ElideRight
                }
            }
        }

        // -- content --------------------------------------------------------
        StackLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            currentIndex: nav.currentIndex

            DashboardScreen {
                api: api
                strips: window.strips
                rants: window.rants
                favorites: window.favorites
                progressStrip: window.progressStrip
                openReader: window.openReader
                openRant: window.openRant
            }

            ReaderScreen {
                id: readerScreen
                api: api
                strips: window.strips
                chapters: window.chapters
                favorites: window.favorites
                toggleFavorite: window.toggleFavorite
                saveProgress: window.saveProgress
                // Initial default only: landing on this tab straight from
                // the sidebar (not via Dashboard's own "resume reading",
                // which already calls openReader explicitly) should still
                // pick up where the user left off. Any later manual
                // navigation (goTo, the jump dropdown, openReader) assigns
                // currentNumber directly, which breaks this binding for
                // the rest of the session — exactly what we want, since a
                // stale progressStrip refresh shouldn't yank the user back
                // to an old strip mid-navigation.
                currentNumber: window.progressStrip
            }

            GalleryScreen {
                api: api
                strips: window.strips
                chapters: window.chapters
                favorites: window.favorites
                openReader: window.openReader
            }

            RantsScreen {
                id: rantsScreen
                api: api
                rants: window.rants
                deeplConfigured: window.deeplConfigured
            }

            SettingsScreen {
                api: api
                guiCtrlApi: guiCtrlApi
                localBaseUrl: window.baseUrl
                localToken: window.apiToken
                daemonConfigSaved: window.refreshDeeplConfigured
            }
        }
    }
}
