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

    readonly property var theme: Theme {}

    // "all" | "favorites" | a chapter's category string
    property string selectedFilter: "all"

    // Chapters with a non-zero number are the main story; bonus/side
    // sections parse to category number 0 (see core::scraper::chapters,
    // and ReaderScreen's own mainStoryCategories) — numbered here and
    // grouped ahead of a "Bonus" divider so the two are easy to tell
    // apart at a glance, not just by title.
    readonly property var mainChapters: chapters.filter(function (c) {
        return c.number > 0
    }).slice().sort(function (a, b) {
        return a.number - b.number
    })
    readonly property var bonusChapters: chapters.filter(function (c) {
        return c.number === 0
    })

    readonly property var chipModel: {
        var items = [{ key: "all", label: I18n.tr("gallery.chipAll"), kind: "pill" }]
        root.mainChapters.forEach(function (c) {
            items.push({ key: c.category, label: c.number + ". " + c.title, kind: "pill" })
        })
        if (root.bonusChapters.length > 0)
            items.push({ key: "__bonus_divider__", label: I18n.tr("gallery.sectionBonus"), kind: "divider" })
        root.bonusChapters.forEach(function (c) {
            items.push({ key: c.category, label: c.title, kind: "pill" })
        })
        items.push({ key: "favorites", label: I18n.tr("gallery.chipFavorites"), kind: "pill" })
        return items
    }

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

        Flow {
            Layout.fillWidth: true
            spacing: 8

            Repeater {
                model: root.chipModel
                delegate: Item {
                    id: chipDelegate
                    readonly property bool isDivider: modelData.kind === "divider"
                    height: 28
                    width: isDivider ? dividerRow.implicitWidth : (chipLabel.implicitWidth + 24)

                    // A plain, non-interactive section label — not a chip —
                    // so "Bonus" reads as a group heading, not a filter of
                    // its own.
                    Row {
                        id: dividerRow
                        visible: chipDelegate.isDivider
                        height: parent.height
                        spacing: 8
                        Rectangle {
                            width: 1
                            height: 16
                            anchors.verticalCenter: parent.verticalCenter
                            color: theme.line
                        }
                        Label {
                            anchors.verticalCenter: parent.verticalCenter
                            text: modelData.label
                            font.pixelSize: 11
                            font.capitalization: Font.AllUppercase
                            color: theme.textFaint
                        }
                    }

                    Rectangle {
                        visible: !chipDelegate.isDivider
                        anchors.fill: parent
                        radius: 999
                        color: root.selectedFilter === modelData.key ? theme.tealDim : "transparent"
                        border.color: root.selectedFilter === modelData.key ? theme.tealDim : theme.line
                        border.width: 1

                        Label {
                            id: chipLabel
                            anchors.centerIn: parent
                            text: modelData.label
                            font.pixelSize: 12
                            color: root.selectedFilter === modelData.key ? theme.teal : theme.textDim
                        }
                        MouseArea {
                            anchors.fill: parent
                            onClicked: root.selectedFilter = modelData.key
                        }
                    }
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

                    Label {
                        visible: root.favoriteNumbers.indexOf(modelData.number) !== -1
                        anchors.top: parent.top
                        anchors.right: parent.right
                        anchors.margins: 6
                        text: "♥"
                        color: theme.red
                        font.pixelSize: 13
                    }

                    MouseArea {
                        anchors.fill: parent
                        onClicked: root.openReader(modelData.number)
                    }
                }
            }
        }
    }
}
