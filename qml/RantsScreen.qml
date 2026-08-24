import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// Rants screen: list + detail, with a translation toggle that calls
// GET /rant?number=N&lang=xx (cached server-side by the daemon after the
// first translation, see core::translate) — this screen keeps its own
// per-session cache too, so switching back to an already-seen language is
// instant.
//
// No language *picker*: translation only ever offers the system's own
// language (I18n.currentLanguage, already resolved from Qt.locale() at
// startup), and only when a DeepL key is actually configured — DeepL is a
// paid-by-usage API keyed to the user's own account, so there's no sense
// spending it translating into languages nobody in this install reads.
Item {
    id: root

    property var api: null
    property var rants: []
    property int selectedNumber: -1
    property bool deeplConfigured: false

    readonly property var theme: Theme {}

    readonly property string systemLang: I18n.currentLanguage
    readonly property bool canTranslate: root.deeplConfigured && root.systemLang !== "en"
    readonly property var languages: {
        var opts = [{ code: "en", label: "EN" }]
        if (root.canTranslate)
            opts.push({ code: root.systemLang, label: root.systemLang.toUpperCase() })
        return opts
    }
    property string selectedLang: "en"
    property var translationCache: ({})
    property bool loadingTranslation: false

    readonly property var selectedRant: rants.find(function (r) {
        return r.number === selectedNumber
    })

    readonly property string displayedContent: {
        if (!selectedRant)
            return ""
        if (selectedLang === "en")
            return selectedRant.content
        var key = selectedRant.number + "_" + selectedLang
        return translationCache[key] !== undefined ? translationCache[key] : ""
    }

    onRantsChanged: {
        if (selectedNumber === -1 && rants.length > 0)
            selectedNumber = rants[0].number
    }

    // The DeepL key can be cleared (or the app's own system-language
    // detection can't change mid-run, but the key can) from Settings while
    // a translated rant is on screen — fall back to the original rather
    // than keep pointing at a language selectLang() can no longer offer.
    onCanTranslateChanged: {
        if (!canTranslate)
            selectedLang = "en"
    }

    function selectLang(code) {
        selectedLang = code
        if (code === "en" || !selectedRant)
            return
        var key = selectedRant.number + "_" + code
        if (translationCache[key] !== undefined)
            return
        loadingTranslation = true
        api.get("/rant?number=" + selectedRant.number + "&lang=" + code, function (body) {
            var updated = Object.assign({}, translationCache)
            updated[key] = body.content
            translationCache = updated
            loadingTranslation = false
        }, function () {
            loadingTranslation = false
        })
    }

    RowLayout {
        anchors.fill: parent
        anchors.margins: 28
        spacing: 20

        ColumnLayout {
            // minimum/maximum, not just preferred: a RowLayout is free to
            // compress an item below its preferred width when space is
            // tight (narrower window, a long selected title on the right)
            // — pinning all three to 220 makes the list panel genuinely
            // fixed-width rather than just usually-220.
            Layout.preferredWidth: 220
            Layout.minimumWidth: 220
            Layout.maximumWidth: 220
            Layout.fillHeight: true
            spacing: 12

            Label {
                text: I18n.tr("rants.title")
                font.family: theme.fontDisplay
                font.pixelSize: 20
                font.bold: true
                color: theme.text
            }

            ListView {
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                model: root.rants
                delegate: Rectangle {
                    width: ListView.view.width
                    height: 46
                    radius: 9
                    color: modelData.number === root.selectedNumber ? theme.panelRaised : "transparent"
                    border.color: modelData.number === root.selectedNumber ? theme.lineSoft : "transparent"
                    border.width: 1

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 8
                        spacing: 1
                        Label {
                            text: "#" + modelData.number + " · " + modelData.publish_date.slice(0, 10)
                            font.family: theme.fontMono
                            font.pixelSize: 9
                            color: theme.textFaint
                        }
                        Label {
                            text: modelData.title
                            font.pixelSize: 12
                            color: modelData.number === root.selectedNumber ? theme.teal : theme.text
                            elide: Text.ElideRight
                            Layout.fillWidth: true
                        }
                    }
                    MouseArea {
                        anchors.fill: parent
                        onClicked: {
                            root.selectedNumber = modelData.number
                            root.selectedLang = "en"
                        }
                    }
                }
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 8

            Label {
                text: root.selectedRant ? root.selectedRant.title : ""
                font.family: theme.fontDisplay
                font.pixelSize: 18
                font.bold: true
                color: theme.text
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
            }
            Label {
                text: root.selectedRant ? (root.selectedRant.author + " · #" + root.selectedRant.number + " · " + root.selectedRant.publish_date.slice(0, 10)) : ""
                font.family: theme.fontMono
                font.pixelSize: 11
                color: theme.textFaint
            }

            Row {
                spacing: 6
                visible: root.canTranslate
                Repeater {
                    model: root.languages
                    delegate: Rectangle {
                        width: 40; height: 24; radius: 6
                        color: root.selectedLang === modelData.code ? theme.teal : "transparent"
                        border.color: root.selectedLang === modelData.code ? theme.teal : theme.line
                        border.width: 1
                        Label {
                            anchors.centerIn: parent
                            text: modelData.label
                            font.family: theme.fontMono
                            font.pixelSize: 10
                            color: root.selectedLang === modelData.code ? theme.ink : theme.textDim
                        }
                        MouseArea {
                            anchors.fill: parent
                            onClicked: root.selectLang(modelData.code)
                        }
                    }
                }
            }

            ScrollView {
                Layout.fillWidth: true
                Layout.fillHeight: true

                Label {
                    width: parent.width
                    text: root.loadingTranslation ? I18n.tr("rants.loadingTranslation") : root.displayedContent
                    textFormat: Text.RichText
                    wrapMode: Text.WordWrap
                    color: theme.textDim
                    font.pixelSize: 13
                    lineHeight: 1.5
                }
            }
        }
    }
}
