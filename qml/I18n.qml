pragma Singleton
import QtQuick

// Ported from linux_hello_config's own I18n.qml (same author, same
// pattern): JSON dictionaries under i18n/<lang>.json, loaded via a
// synchronous local-file XHR (requires QML_XHR_ALLOW_FILE_READ=1, set by
// gui::launcher when it spawns qml6 — Qt blocks local-file XHR reads by
// default otherwise). en.json is the single source of truth for English
// strings, not duplicated here.
QtObject {
    id: i18n

    property var translations: ({})
    property string currentLanguage: "en"

    readonly property var languages: ["en", "fr", "es", "de", "ru", "ja", "zh", "ar", "hi", "pt"]
    readonly property var languageNames: ({
        "en": "English",
        "fr": "Français (French)",
        "es": "Español (Spanish)",
        "de": "Deutsch (German)",
        "ru": "Русский (Russian)",
        "ja": "日本語 (Japanese)",
        "zh": "中文 (Chinese)",
        "ar": "العربية (Arabic)",
        "hi": "हिंदी (Hindi)",
        "pt": "Português (Portuguese)"
    })

    function loadLanguage(lang) {
        var qmlPath = Qt.resolvedUrl("./i18n/" + lang + ".json")
        var xhr = new XMLHttpRequest()
        xhr.open("GET", qmlPath, false)
        try {
            xhr.send()
            if (xhr.status === 200) {
                var loaded = JSON.parse(xhr.responseText)
                if (loaded && typeof loaded === 'object') {
                    translations = loaded
                    currentLanguage = lang
                    return true
                }
            }
        } catch (e) {
            // Silent fallback — tr() returns raw keys for anything missing.
        }

        currentLanguage = lang
        return true
    }

    function tr(key) {
        if (!key || key === "")
            return key

        if (key in translations)
            return translations[key]

        var keys = key.split('.')
        var value = translations
        for (var i = 0; i < keys.length; i++) {
            if (value && typeof value === 'object' && keys[i] in value) {
                value = value[keys[i]]
            } else {
                return key
            }
        }
        return typeof value === 'string' ? value : key
    }

    Component.onCompleted: {
        var systemLang = Qt.locale().name.substring(0, 2).toLowerCase()
        if (languages.includes(systemLang))
            loadLanguage(systemLang)
        else
            loadLanguage("en")
    }
}
