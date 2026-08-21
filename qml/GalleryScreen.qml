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

    readonly property var chipModel: {
        var items = [{ key: "all", label: I18n.tr("gallery.chipAll") }]
        chapters.forEach(function (c) {
            items.push({ key: c.category, label: c.title })
        })
        items.push({ key: "favorites", label: I18n.tr("gallery.chipFavorites") })
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
                delegate: Rectangle {
                    height: 28
                    width: chipLabel.implicitWidth + 24
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
