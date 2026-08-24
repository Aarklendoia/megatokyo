import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// Reader screen: strip-by-strip navigation, "all strips" vs "main story
// only" (chapters under category "C-<n>" — bonus sections get their own
// short codes, see core::scraper::chapters), favorites, and quick-jump to
// a chapter/category or to favorites.
Item {
    id: root

    property var api: null
    property var guiCtrlApi: null
    property var strips: []
    property var chapters: []
    property var favorites: []
    property var toggleFavorite: function (number) {}
    property var saveProgress: function (number) {}

    property int currentNumber: -1
    property bool mainStoryOnly: false
    // Guards the very first mainStoryOnly assignment (from loadMainStoryOnly
    // below) from immediately posting the value straight back to the GUI's
    // own config — it was just read from there, writing it back is a no-op
    // that would only race the initial GET.
    property bool mainStoryOnlyLoaded: false

    readonly property var theme: Theme {}

    // Main-story chapters are stored under category "C-<n>" — including the
    // prologue, "C-0", which is a real part of the story despite parsing to
    // `number: 0` (see core::scraper::chapters's doc comment). Filtering on
    // `number > 0` alone would misfile it as bonus content, along with the
    // genuinely bonus sections (One Shot Episode, Grand Theft Colo, ...)
    // that also parse to number 0 — category prefix is the one signal that
    // actually distinguishes the two.
    readonly property var mainStoryCategories: chapters.filter(function (c) {
        return c.category.indexOf("C-") === 0
    }).map(function (c) {
        return c.category
    })

    readonly property var filteredStrips: {
        if (!mainStoryOnly)
            return strips
        return strips.filter(function (s) {
            return mainStoryCategories.indexOf(s.category) !== -1
        })
    }

    readonly property int currentIndex: filteredStrips.findIndex(function (s) {
        return s.number === currentNumber
    })

    readonly property var currentStrip: strips.find(function (s) {
        return s.number === currentNumber
    })

    readonly property bool isFavorite: favorites.some(function (f) {
        return f.strip_number === currentNumber
    })

    // Mirrors filteredStrips' own scope: with "Main story only" active,
    // jumping to a bonus chapter would immediately be overridden by
    // onMainStoryOnlyChanged-style logic anyway (nothing there matches the
    // filter) — so bonus chapters aren't offered as a jump target at all
    // while that filter is on.
    readonly property var jumpModel: {
        var source = mainStoryOnly ? chapters.filter(function (c) {
            return c.category.indexOf("C-") === 0
        }) : chapters
        var items = source.map(function (c) {
            var isMainStory = c.category.indexOf("C-") === 0
            return {
                category: c.category,
                label: (isMainStory ? I18n.tr("reader.jumpChapter") + " " + c.number : I18n.tr("reader.jumpBonus")) + " — " + c.title
            }
        })
        items.push({ category: "__favorites__", label: I18n.tr("reader.jumpFavorites") })
        return items
    }

    function goTo(delta) {
        var list = filteredStrips
        var idx = currentIndex
        var next = idx + delta
        if (idx === -1 && list.length > 0) {
            currentNumber = list[0].number
            return
        }
        if (next >= 0 && next < list.length)
            currentNumber = list[next].number
    }

    onCurrentNumberChanged: {
        if (currentNumber > 0)
            saveProgress(currentNumber)
    }

    // Switching "All strips" ↔ "Main story only" can leave the strip
    // currently on screen outside the new filter (currentIndex === -1) —
    // jump to the nearest strip at or before it that does match, rather
    // than silently keep showing a strip the active filter excludes.
    // Falls back to the first match if the current one comes before
    // everything in the new list (e.g. mid-prologue bonus content).
    //
    // Also persists the toggle to this GUI's own config (see
    // loadMainStoryOnly below), skipped for the one assignment that
    // *loads* the persisted value in the first place.
    onMainStoryOnlyChanged: {
        if (mainStoryOnlyLoaded && root.guiCtrlApi)
            root.guiCtrlApi.post("/gui-config?main_story_only=" + (mainStoryOnly ? "true" : "false"))

        if (currentIndex !== -1)
            return
        var list = filteredStrips
        if (list.length === 0)
            return
        var candidate = list[0]
        for (var i = 0; i < list.length; i++) {
            if (list[i].number > currentNumber)
                break
            candidate = list[i]
        }
        currentNumber = candidate.number
    }

    // Loads the persisted "All strips"/"Main story only" toggle once
    // guiCtrlApi is ready (see main.qml's verified Component.onCompleted
    // ordering — the window's own onCompleted, which resolves
    // guiCtrlApi's real port/token, runs before this screen's). Sets
    // mainStoryOnlyLoaded first so the onMainStoryOnlyChanged handler
    // above knows this particular assignment came from the load, not a
    // user toggle, and doesn't immediately post it straight back.
    function loadMainStoryOnly() {
        if (!guiCtrlApi)
            return
        guiCtrlApi.get("/gui-config", function (body) {
            if (body && body.main_story_only)
                root.mainStoryOnly = true
            mainStoryOnlyLoaded = true
        }, function () {
            // Couldn't read the persisted value — still let later toggles
            // persist rather than silently never saving for the rest of
            // this session.
            mainStoryOnlyLoaded = true
        })
    }
    Component.onCompleted: loadMainStoryOnly()

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 28
        spacing: 16

        Label {
            text: I18n.tr("reader.title")
            font.family: theme.fontDisplay
            font.pixelSize: 20
            font.bold: true
            color: theme.text
        }

        RowLayout {
            Layout.fillWidth: true

            Row {
                spacing: 2
                Rectangle {
                    width: 260; height: 30; radius: 9
                    color: theme.panelSunken
                    border.color: theme.line
                    border.width: 1

                    Row {
                        anchors.fill: parent
                        anchors.margins: 2

                        Rectangle {
                            width: parent.width / 2; height: parent.height
                            radius: 7
                            color: !root.mainStoryOnly ? theme.panelRaised : "transparent"
                            Label {
                                anchors.centerIn: parent
                                width: parent.width - 8
                                horizontalAlignment: Text.AlignHCenter
                                elide: Text.ElideRight
                                text: I18n.tr("reader.segAll")
                                font.pixelSize: 11
                                color: !root.mainStoryOnly ? theme.text : theme.textDim
                            }
                            MouseArea { anchors.fill: parent; onClicked: root.mainStoryOnly = false }
                        }
                        Rectangle {
                            width: parent.width / 2; height: parent.height
                            radius: 7
                            color: root.mainStoryOnly ? theme.panelRaised : "transparent"
                            Label {
                                anchors.centerIn: parent
                                width: parent.width - 8
                                horizontalAlignment: Text.AlignHCenter
                                elide: Text.ElideRight
                                text: I18n.tr("reader.segMain")
                                font.pixelSize: 11
                                color: root.mainStoryOnly ? theme.text : theme.textDim
                            }
                            MouseArea { anchors.fill: parent; onClicked: root.mainStoryOnly = true }
                        }
                    }
                }
            }

            Item { Layout.fillWidth: true }

            ComboBox {
                Layout.preferredWidth: 220
                model: root.jumpModel
                textRole: "label"
                onActivated: function (index) {
                    var item = root.jumpModel[index]
                    if (item.category === "__favorites__") {
                        if (root.favorites.length > 0)
                            root.currentNumber = root.favorites[0].strip_number
                    } else {
                        var matches = root.strips.filter(function (s) {
                            return s.category === item.category
                        })
                        if (matches.length > 0)
                            root.currentNumber = matches[0].number
                    }
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 14

            ToolButton {
                text: "‹"
                enabled: root.currentIndex > 0
                onClicked: root.goTo(-1)
                contentItem: Label { text: "‹"; color: theme.text; font.pixelSize: 18; horizontalAlignment: Text.AlignHCenter }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.fillHeight: true
                radius: 10
                color: theme.panelSunken
                border.color: theme.line
                border.width: 1
                clip: true

                Image {
                    anchors.fill: parent
                    source: root.currentNumber > 0 && root.api ? root.api.imageUrl(root.currentNumber) : ""
                    fillMode: Image.PreserveAspectFit
                    asynchronous: true
                }

                // Click-to-turn-page zones — the whole left/right half of
                // the strip, not just the small ‹/› buttons, mirroring how
                // most comic readers let you click the page itself.
                // Placed here (after the Image, before the favorite badge
                // and title bar below) so those two stay on top and remain
                // clickable in their own corners despite overlapping these
                // zones.
                MouseArea {
                    anchors.left: parent.left
                    anchors.top: parent.top
                    anchors.bottom: parent.bottom
                    width: parent.width / 2
                    enabled: root.currentIndex > 0
                    cursorShape: enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
                    hoverEnabled: true
                    onClicked: root.goTo(-1)
                    Label {
                        anchors.left: parent.left
                        anchors.verticalCenter: parent.verticalCenter
                        anchors.leftMargin: 8
                        text: "‹"
                        font.pixelSize: 28
                        color: theme.text
                        opacity: parent.containsMouse ? 0.5 : 0
                        Behavior on opacity { NumberAnimation { duration: 120 } }
                    }
                }
                MouseArea {
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.bottom: parent.bottom
                    width: parent.width / 2
                    enabled: root.currentIndex >= 0 && root.currentIndex < root.filteredStrips.length - 1
                    cursorShape: enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
                    hoverEnabled: true
                    onClicked: root.goTo(1)
                    Label {
                        anchors.right: parent.right
                        anchors.verticalCenter: parent.verticalCenter
                        anchors.rightMargin: 8
                        text: "›"
                        font.pixelSize: 28
                        color: theme.text
                        opacity: parent.containsMouse ? 0.5 : 0
                        Behavior on opacity { NumberAnimation { duration: 120 } }
                    }
                }

                Rectangle {
                    anchors.top: parent.top
                    anchors.right: parent.right
                    anchors.margins: 10
                    width: 34; height: 34; radius: 17
                    color: Qt.rgba(0.05, 0.05, 0.07, 0.6)
                    border.color: root.isFavorite ? theme.red : theme.line
                    border.width: 1

                    Label {
                        anchors.centerIn: parent
                        text: root.isFavorite ? "♥" : "♡"
                        color: root.isFavorite ? theme.red : theme.textDim
                        font.pixelSize: 16
                    }
                    MouseArea {
                        anchors.fill: parent
                        onClicked: root.currentNumber > 0 && root.toggleFavorite(root.currentNumber)
                    }
                }

                Rectangle {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.bottom: parent.bottom
                    height: 56
                    gradient: Gradient {
                        GradientStop { position: 0.0; color: Qt.rgba(0.05, 0.05, 0.07, 0.92) }
                        GradientStop { position: 1.0; color: "transparent" }
                    }

                    ColumnLayout {
                        anchors.left: parent.left
                        anchors.bottom: parent.bottom
                        anchors.margins: 12
                        spacing: 1
                        Label {
                            text: root.currentNumber > 0 ? ("#" + root.currentNumber) : ""
                            color: theme.textFaint
                            font.family: theme.fontMono
                            font.pixelSize: 10
                        }
                        Label {
                            text: root.currentStrip ? root.currentStrip.title : ""
                            color: theme.text
                            font.family: theme.fontDisplay
                            font.pixelSize: 15
                            font.bold: true
                        }
                    }
                }
            }

            ToolButton {
                text: "›"
                enabled: root.currentIndex >= 0 && root.currentIndex < root.filteredStrips.length - 1
                onClicked: root.goTo(1)
                contentItem: Label { text: "›"; color: theme.text; font.pixelSize: 18; horizontalAlignment: Text.AlignHCenter }
            }
        }
    }
}
