import QtQuick

// Thin wrapper around XMLHttpRequest for the daemon's hand-rolled HTTP API
// (see daemon/src/control.rs) — no QML networking module beyond what Qt
// Quick ships with, matching the rest of this project's "no extra deps"
// stance.
QtObject {
    id: root

    property string baseUrl: ""
    property string token: ""

    function request(method, path, onSuccess, onError) {
        var xhr = new XMLHttpRequest()
        xhr.open(method, baseUrl + path)
        xhr.setRequestHeader("x-megatokyo-daemon-token", token)
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

    function del(path, onSuccess, onError) {
        request("DELETE", path, onSuccess, onError)
    }

    // The daemon requires the token as a header on every other route, but
    // QML's `Image` element can't set custom request headers — `/image`
    // alone also accepts it as a query param (see is_authorized in
    // daemon/src/control.rs).
    function imageUrl(number) {
        return baseUrl + "/image?number=" + number + "&token=" + encodeURIComponent(token)
    }
}
