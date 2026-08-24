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
    property var strips: []
    property var chapters: []
    property var favorites: []
    property var toggleFavorite: function (number) {}
    property var saveProgress: function (number) {}

    property int currentNumber: -1
    property bool mainStoryOnly: false

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

    readonly property var jumpModel: {
        var items = chapters.map(function (c) {
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
    onMainStoryOnlyChanged: {
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
