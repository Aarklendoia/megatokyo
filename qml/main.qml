import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// Shell: sidebar nav + the four screens reviewed with the user before this
// was built (Home/Reader/Gallery/Rants — Settings' remote-config
// persistence is a separate follow-up, see the project's issues). QML talks
// to the daemon directly over its HTTP API (local or remote, see
// gui::daemon_link), so there's nothing here for a local control server to
// answer — base_url/token just come in as positional arguments from
// gui::launcher.
ApplicationWindow {
    id: window
    visible: true
    width: 1180
    height: 760
    minimumWidth: 720
    minimumHeight: 480
    title: "Megatokyo"

    property string baseUrl: Qt.application.arguments.length > 1 ? Qt.application.arguments[Qt.application.arguments.length - 2] : ""
    property string apiToken: Qt.application.arguments.length > 1 ? Qt.application.arguments[Qt.application.arguments.length - 1] : ""

    property var chapters: []
    property var strips: []
    property var rants: []
    property var favorites: []
    property var status: ({})
    property int progressStrip: -1

    readonly property var theme: Theme {}

    Api {
        id: api
        baseUrl: window.baseUrl
        token: window.apiToken
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
    function refreshAll() {
        refreshChapters()
        refreshStrips()
        refreshRants()
        refreshFavorites()
        refreshStatus()
        refreshProgress()
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

    Component.onCompleted: refreshAll()

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
                    model: [I18n.tr("app.navHome"), I18n.tr("app.navReader"), I18n.tr("app.navGallery"), I18n.tr("app.navRants")]
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
            }
        }
    }
}
