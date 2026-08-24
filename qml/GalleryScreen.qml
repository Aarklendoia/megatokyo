import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// Gallery screen: strip thumbnails grouped by chapter/category, with a
// favorites filter — same category/chapter model as ReaderScreen.
Item {
    id: root

    property var api: null
    property var strips: []
    property var chapters: []
    property var favorites: []
    property var openReader: function (number) {}
    property var toggleFavorite: function (number) {}

    readonly property var theme: Theme {}

    // "all" | "favorites" | a chapter's category string
    property string selectedFilter: "all"

    // Main-story chapters are stored under category "C-<n>", including the
    // prologue ("C-0", number 0) — see core::scraper::chapters's doc
    // comment and ReaderScreen's own mainStoryCategories, which hit the
    // same pitfall: filtering on `number > 0` instead of the category
    // prefix misfiles the prologue as bonus content, since it shares
    // number 0 with the genuinely bonus sections (One Shot Episode, Grand
    // Theft Colo, ...). Numbered here and grouped ahead of a "Bonus"
    // divider so the two read apart at a glance, not just by title.
    readonly property var mainChapters: chapters.filter(function (c) {
        return c.category.indexOf("C-") === 0
    }).slice().sort(function (a, b) {
        return a.number - b.number
    })
    readonly property var bonusChapters: chapters.filter(function (c) {
        return c.category.indexOf("C-") !== 0
    })

    // Two separate rows rather than one Flow with an inline divider: a
    // single Flow just wraps on width, so bonus chips could still end up
    // sharing a visual line with main-story ones whenever there was room
    // — a real second row is the only way to keep them apart regardless
    // of window width.
    readonly property var mainRowChips: {
        var items = [{ key: "all", label: I18n.tr("gallery.chipAll") }]
        root.mainChapters.forEach(function (c) {
            items.push({ key: c.category, label: c.number + ". " + c.title })
        })
        items.push({ key: "favorites", label: I18n.tr("gallery.chipFavorites") })
        return items
    }
    readonly property var bonusRowChips: root.bonusChapters.map(function (c) {
        return { key: c.category, label: c.title }
    })

    readonly property var favoriteNumbers: favorites.map(function (f) {
        return f.strip_number
    })

    readonly property var filteredStrips: {
        if (selectedFilter === "all")
            return strips
        if (selectedFilter === "favorites")
            return strips.filter(function (s) {
                return root.favoriteNumbers.indexOf(s.number) !== -1
            })
        return strips.filter(function (s) {
            return s.category === selectedFilter
        })
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 28
        spacing: 16

        Label {
            text: I18n.tr("gallery.title")
            font.family: theme.fontDisplay
            font.pixelSize: 20
            font.bold: true
            color: theme.text
        }

        // One row for "all" + main-story chapters + favorites, a
        // separate row below for bonus chapters — see mainRowChips'/
        // bonusRowChips' own doc comment on why this is two Flows rather
        // than one with an inline divider.
        Flow {
            Layout.fillWidth: true
            spacing: 8

            Repeater {
                model: root.mainRowChips
                delegate: chipDelegate
            }
        }

        Flow {
            Layout.fillWidth: true
            spacing: 8
            visible: root.bonusRowChips.length > 0

            Label {
                height: 28
                verticalAlignment: Text.AlignVCenter
                text: I18n.tr("gallery.sectionBonus")
                font.pixelSize: 11
                font.capitalization: Font.AllUppercase
                color: theme.textFaint
            }
            Repeater {
                model: root.bonusRowChips
                delegate: chipDelegate
            }
        }

        Component {
            id: chipDelegate
            Rectangle {
                id: chip
                required property string key
                required property string label
                height: 28
                width: chipLabel.implicitWidth + 24
                radius: 999
                color: root.selectedFilter === chip.key ? theme.tealDim : "transparent"
                border.color: root.selectedFilter === chip.key ? theme.tealDim : theme.line
                border.width: 1

                Label {
                    id: chipLabel
                    anchors.centerIn: parent
                    text: chip.label
                    font.pixelSize: 12
                    color: root.selectedFilter === chip.key ? theme.teal : theme.textDim
                }
                MouseArea {
                    anchors.fill: parent
                    onClicked: root.selectedFilter = chip.key
                }
            }
        }

        GridView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            cellWidth: 128
            cellHeight: 168
            clip: true
            model: root.filteredStrips

            delegate: Item {
                width: 120
                height: 158

                Rectangle {
                    anchors.fill: parent
                    radius: 9
                    color: theme.panelSunken
                    border.color: theme.lineSoft
                    border.width: 1
                    clip: true

                    Image {
                        anchors.fill: parent
                        source: root.api ? root.api.imageUrl(modelData.number) : ""
                        fillMode: Image.PreserveAspectCrop
                        asynchronous: true
                    }

                    Rectangle {
                        anchors.left: parent.left
                        anchors.bottom: parent.bottom
                        anchors.margins: 6
                        width: numLabel.implicitWidth + 10
                        height: 18
                        radius: 5
                        color: Qt.rgba(0.05, 0.05, 0.07, 0.75)
                        Label {
                            id: numLabel
                            anchors.centerIn: parent
                            text: "#" + modelData.number
                            font.family: theme.fontMono
                            font.pixelSize: 10
                            color: theme.text
                        }
                    }

                    MouseArea {
                        anchors.fill: parent
                        onClicked: root.openReader(modelData.number)
                    }

                    // Favorite toggle — always visible (not just when
                    // already favorited), and placed after the big
                    // MouseArea above so it sits on top and intercepts
                    // clicks in its corner instead of opening the reader.
                    // The only other way to favorite a strip used to be a
                    // small icon buried in the Reader.
                    Rectangle {
                        readonly property bool isFavorite: root.favoriteNumbers.indexOf(modelData.number) !== -1
                        anchors.top: parent.top
                        anchors.right: parent.right
                        anchors.margins: 6
                        width: 24; height: 24; radius: 12
                        color: Qt.rgba(0.05, 0.05, 0.07, 0.65)
                        border.color: isFavorite ? theme.red : theme.line
                        border.width: 1

                        Label {
                            anchors.centerIn: parent
                            text: parent.isFavorite ? "♥" : "♡"
                            color: parent.isFavorite ? theme.red : theme.textDim
                            font.pixelSize: 12
                        }
                        MouseArea {
                            anchors.fill: parent
                            onClicked: root.toggleFavorite(modelData.number)
                        }
                    }
                }
            }
        }
    }
}
