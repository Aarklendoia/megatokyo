import QtQuick

// Thin wrapper around XMLHttpRequest for this GUI's own local control
// server (see gui::control) — same shape as Api.qml, but for the
// gui-ctrl-token header the launcher generates fresh each run, not the
// daemon's own token.
QtObject {
    id: root

    property string baseUrl: ""
    property string token: ""

    function request(method, path, onSuccess, onError) {
        var xhr = new XMLHttpRequest()
        xhr.open(method, baseUrl + path)
        xhr.setRequestHeader("x-megatokyo-gui-ctrl-token", token)
        xhr.onreadystatechange = function () {
            if (xhr.readyState !== XMLHttpRequest.DONE)
                return
            if (xhr.status >= 200 && xhr.status < 300) {
                var body = null
                if (xhr.responseText.length > 0) {
                    try {
                        body = JSON.parse(xhr.responseText)
                    } catch (e) {
                        body = null
                    }
                }
                if (onSuccess)
                    onSuccess(body)
            } else if (onError) {
                onError(xhr.status, xhr.responseText)
            }
        }
        xhr.send()
    }

    function get(path, onSuccess, onError) {
        request("GET", path, onSuccess, onError)
    }

    function post(path, onSuccess, onError) {
        request("POST", path, onSuccess, onError)
    }
}
