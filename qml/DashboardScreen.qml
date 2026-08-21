import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// Home screen: last strip, last rant, resume reading, search by title/number.
Item {
    id: root

    property var api: null
    property var strips: []
    property var rants: []
    property var favorites: []
    property int progressStrip: -1
    property var openReader: function (number) {}
    property var openRant: function (number) {}

    readonly property var theme: Theme {}
    readonly property var lastStrip: strips.length > 0 ? strips[strips.length - 1] : null
    readonly property var lastRant: rants.length > 0 ? rants[0] : null

    readonly property var searchResults: {
        var q = searchField.text.trim().toLowerCase()
        if (q.length === 0)
            return []
        return strips.filter(function (s) {
            return s.title.toLowerCase().indexOf(q) !== -1 || String(s.number) === q
        }).slice(0, 8)
    }

    ScrollView {
        anchors.fill: parent
        contentWidth: availableWidth

        ColumnLayout {
            width: root.width
            spacing: 22
            Layout.margins: 28

            Label {
                text: I18n.tr("dashboard.title")
                font.family: theme.fontDisplay
                font.pixelSize: 20
                font.bold: true
                color: theme.text
            }

            // -- search --------------------------------------------------
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 4

                Rectangle {
                    Layout.preferredWidth: 440
                    Layout.preferredHeight: 40
                    radius: 10
                    color: theme.panelSunken
                    border.color: theme.line
                    border.width: 1

                    TextField {
                        id: searchField
                        anchors.fill: parent
                        anchors.margins: 8
                        background: null
                        color: theme.text
                        placeholderText: I18n.tr("dashboard.searchPlaceholder")
                        placeholderTextColor: theme.textFaint
                        selectByMouse: true
                    }
                }

                Rectangle {
                    visible: searchResults.length > 0
                    Layout.preferredWidth: 440
                    Layout.preferredHeight: Math.min(searchResults.length, 6) * 34
                    color: theme.panelRaised
                    border.color: theme.lineSoft
                    border.width: 1
                    radius: 8

                    ListView {
                        anchors.fill: parent
                        anchors.margins: 4
                        model: searchResults
                        delegate: ItemDelegate {
                            id: resultDelegate
                            width: parent ? parent.width : 0
                            height: 30
                            text: "#" + modelData.number + "  " + modelData.title
                            font.family: theme.fontBody
                            font.pixelSize: 12
                            contentItem: Label {
                                text: resultDelegate.text
                                color: theme.text
                                font: resultDelegate.font
                                verticalAlignment: Text.AlignVCenter
                                leftPadding: 8
                            }
                            background: Rectangle {
                                color: hovered ? theme.panel : "transparent"
                            }
                            onClicked: {
                                searchField.text = ""
                                root.openReader(modelData.number)
                            }
                        }
                    }
                }
            }

            // -- last strip / last rant -----------------------------------
            RowLayout {
                Layout.fillWidth: true
                spacing: 16

                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredWidth: 2
                    Layout.preferredHeight: 130
                    radius: 12
                    color: theme.panelRaised
                    border.color: theme.lineSoft
                    border.width: 1
                    visible: root.lastStrip !== null

                    RowLayout {
                        anchors.fill: parent
                        anchors.margins: 16
                        spacing: 14

                        Rectangle {
                            Layout.preferredWidth: 108
                            Layout.preferredHeight: 78
                            radius: 8
                            color: theme.panelSunken
                            border.color: theme.line
                            border.width: 1
                            clip: true

                            Image {
                                anchors.fill: parent
                                source: root.lastStrip ? api.imageUrl(root.lastStrip.number) : ""
                                fillMode: Image.PreserveAspectCrop
                                asynchronous: true
                            }
                        }

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 6

                            Label {
                                text: I18n.tr("dashboard.lastStripLabel")
                                color: theme.teal
                                font.family: theme.fontMono
                                font.pixelSize: 10
                            }
                            Label {
                                text: root.lastStrip ? ("#" + root.lastStrip.number) : ""
                                color: theme.textFaint
                                font.family: theme.fontMono
                                font.pixelSize: 11
                            }
                            Label {
                                text: root.lastStrip ? root.lastStrip.title : ""
                                color: theme.text
                                font.family: theme.fontDisplay
                                font.pixelSize: 16
                                font.bold: true
                            }
                            Button {
                                id: resumeButton
                                text: I18n.tr("dashboard.resumeReading")
                                enabled: root.progressStrip > 0 || root.lastStrip !== null
                                onClicked: root.openReader(root.progressStrip > 0 ? root.progressStrip : root.lastStrip.number)
                                background: Rectangle {
                                    color: theme.teal
                                    radius: 8
                                }
                                contentItem: Label {
                                    text: resumeButton.text
                                    color: theme.ink
                                    font.bold: true
                                    font.pixelSize: 12
                                    horizontalAlignment: Text.AlignHCenter
                                }
                            }
                        }
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredWidth: 1
                    Layout.preferredHeight: 130
                    radius: 12
                    color: theme.panelRaised
                    border.color: theme.lineSoft
                    border.width: 1
                    visible: root.lastRant !== null

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 16
                        spacing: 6

                        Label {
                            text: I18n.tr("dashboard.lastRantLabel")
                            color: theme.teal
                            font.family: theme.fontMono
                            font.pixelSize: 10
                        }
                        Label {
                            text: root.lastRant ? root.lastRant.title : ""
                            color: theme.text
                            font.family: theme.fontDisplay
                            font.pixelSize: 15
                            font.bold: true
                            Layout.fillWidth: true
                            wrapMode: Text.WordWrap
                        }
                        Label {
                            text: root.lastRant ? (root.lastRant.author + " · #" + root.lastRant.number) : ""
                            color: theme.textFaint
                            font.family: theme.fontMono
                            font.pixelSize: 10
                        }
                        Item { Layout.fillHeight: true }
                        Button {
                            id: readPostButton
                            text: I18n.tr("dashboard.readPost")
                            onClicked: root.lastRant && root.openRant(root.lastRant.number)
                            background: Rectangle {
                                color: "transparent"
                                border.color: theme.line
                                border.width: 1
                                radius: 8
                            }
                            contentItem: Label {
                                text: readPostButton.text
                                color: theme.text
                                font.pixelSize: 12
                                horizontalAlignment: Text.AlignHCenter
                            }
                        }
                    }
                }
            }

            // -- stats ------------------------------------------------------
            RowLayout {
                Layout.fillWidth: true
                spacing: 12

                Repeater {
                    model: [
                        { n: root.strips.length, l: I18n.tr("dashboard.statsStrips") },
                        { n: root.favorites.length, l: I18n.tr("dashboard.statsFavorites") },
                        { n: root.rants.length, l: I18n.tr("dashboard.statsRants") }
                    ]
                    delegate: Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 62
                        radius: 12
                        color: theme.panelRaised
                        border.color: theme.lineSoft
                        border.width: 1

                        ColumnLayout {
                            anchors.fill: parent
                            anchors.margins: 13
                            spacing: 2
                            Label {
                                text: modelData.n
                                color: theme.text
                                font.family: theme.fontMono
                                font.pixelSize: 19
                            }
                            Label {
                                text: modelData.l
                                color: theme.textDim
                                font.pixelSize: 11
                            }
                        }
                    }
                }
            }
        }
    }
}
