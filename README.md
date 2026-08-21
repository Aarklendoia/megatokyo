# Megatokyo

[![Tests](https://github.com/Aarklendoia/megatokyo/actions/workflows/test.yml/badge.svg)](https://github.com/Aarklendoia/megatokyo/actions/workflows/test.yml)

A Rust + QML rewrite of the [.NET Megatokyo client](https://github.com/Aarklendoia/Megatokyo-NET): browse Fred Gallagher's [Megatokyo](https://megatokyo.com) webcomic by chapter or strip, read the author's rants (blog posts) with on-demand translation, and get notified when a new strip or rant goes up.

## Architecture

- **`core`** — domain types, the `megatokyo.com` scraper, RSS feed parsing, SQLite storage and DeepL translation caching. Shared library, no I/O policy of its own.
- **`daemon`** (`megatokyo-daemon`) — background service: scrapes and caches strips/rants/chapters, serves them over a small hand-rolled HTTP API. Can run co-located with the GUI (loopback) or on a real server reachable by several clients (see below).
- **`gui`** (`megatokyo-gui`) — a thin Rust launcher that spawns Qt's own `qml6` runtime against `qml/`. No Qt/Rust binding crate: the QML talks to the daemon's HTTP API directly. Also runs in `--background` mode as a login-time notification watcher.
- **`qml/`** — the QML UI itself.

The daemon has no built-in TLS: for a real remote deployment, put it behind a TLS-terminating reverse proxy (or a VPN) rather than exposing it directly. See `daemon`'s doc comments for the API's authentication model.

## Building

```sh
cargo build --workspace
cargo test --workspace
```

Running the GUI requires Qt6's QML runtime (`qml6`) and the base QtQuick modules to be installed.

## Status

Early development. `core` (scraper, feed parsing, storage, translation cache) is done and tested against fixtures of the live site; `daemon` and `gui` are in progress — see open issues for the remaining work.

## License

GPL-3.0-or-later, see [LICENSE](LICENSE).
