# Megatokyo

[![Tests](https://github.com/Aarklendoia/megatokyo/actions/workflows/test.yml/badge.svg)](https://github.com/Aarklendoia/megatokyo/actions/workflows/test.yml)

A Rust + QML desktop client for Fred Gallagher's [Megatokyo](https://megatokyo.com) webcomic: browse strips by chapter, keep reading where you left off, read the author's rants (blog posts) with on-demand translation, and get notified when a new strip or rant goes up.

## Features

- **Reader** — full-screen strip view with keyboard (arrow keys) and mouse (click the left/right edge of the page) navigation, a favorites toggle, a jump-to-chapter/bonus/favorites dropdown, and an "All strips" / "Main story only" filter that's remembered between runs. Reopening the app resumes on the last strip you read.
- **Gallery** — thumbnail grid filterable by chapter, with the main story and bonus chapters kept on separate rows and favorites toggleable straight from each thumbnail.
- **Rants** — the full rant archive (not just the last few from the RSS feed), with a search box, a fixed-width list, and a reading-width-capped single-column view. If a DeepL API key is configured, rants are automatically offered in your system language, translated once and cached.
- **Notifications** — an optional background watcher (`megatokyo-gui --background`, installed as a login-time service) sends a desktop notification when a new strip or rant is published.
- **Local or remote daemon** — the scraper/cache/API backend can run on the same machine as the GUI (zero-config) or on a server you host yourself, shared by several clients.

## Install

### Ubuntu/Debian (PPA)

```sh
sudo add-apt-repository ppa:aarklendoia-edtech/megatokyo
sudo apt update
sudo apt install megatokyo-daemon megatokyo-gui
```

### From a GitHub Release

Download the `megatokyo-daemon` and `megatokyo-gui` `.deb` files for your architecture from the [Releases page](https://github.com/Aarklendoia/megatokyo/releases), then:

```sh
sudo apt install ./megatokyo-daemon_*.deb ./megatokyo-gui_*.deb
```

(`apt install ./file.deb` rather than `dpkg -i` so apt resolves the Qt/QML dependencies automatically.)

### From source

```sh
cargo build --workspace --release
```

Running the GUI requires Qt6's QML runtime (`qml6`) and the base QtQuick modules (`qml6-module-qtcore`, `qml6-module-qtquick`, `qml6-module-qtquick-controls`, `qml6-module-qtquick-layouts`) to be installed — see [debian/control](debian/control) for the exact package list on Debian/Ubuntu.

## Usage

### First run

Launch **Megatokyo** from your applications menu (or run `megatokyo-gui`). If no remote daemon is configured in Settings, the GUI starts a local `megatokyo-daemon` for you automatically. On its very first run the daemon backfills its whole local cache (all chapters, strips and rants) in the background — the sidebar shows "Backfilling…" until that finishes; already-cached data is browsable in the meantime.

### Settings

- **Remote daemon** — point the client at a daemon running elsewhere (base URL + API token) instead of the automatic local one. Useful if you're sharing one daemon across several machines.
- **DeepL API key** — required for rant translation. Get a free or paid key from [DeepL](https://www.deepl.com/pro-api); once set, the Rants screen offers a translation into your system's language.
- **Poll interval** — how often the daemon checks for new strips/rants.
- **Notifications** — enable/disable the desktop notification sent by the background watcher.

### Reader

- **←/→** or the on-screen arrows/click zones to move between strips.
- The heart button (top-right of the strip) toggles it as a favorite.
- The segmented control switches between all strips and main-story-only; the dropdown jumps straight to a chapter, a bonus section, or your favorites.

### Gallery

Click a thumbnail to open it in the Reader. Filter chips at the top narrow the grid to one chapter or your favorites; the heart badge on each thumbnail toggles that strip as a favorite without leaving the gallery.

### Rants

Use the search box to filter by title or rant number. If a DeepL key is set, a language toggle appears once a rant is open, offering a translation into your system language (cached after the first translation, so it's only ever translated once).

### Background notifications

Installed as a `systemctl --user` service (`megatokyo-gui-background.service`), enabled by default. To check or change it:

```sh
systemctl --user status megatokyo-gui-background
systemctl --user disable --now megatokyo-gui-background   # to turn it off
```

## Architecture

- **`core`** — domain types, the `megatokyo.com` scraper, RSS feed parsing, SQLite storage and DeepL translation caching. Shared library, no I/O policy of its own.
- **`daemon`** (`megatokyo-daemon`) — background service: scrapes and caches strips/rants/chapters, serves them over a small hand-rolled HTTP API. Can run co-located with the GUI (loopback) or on a real server reachable by several clients.
- **`gui`** (`megatokyo-gui`) — a thin Rust launcher that spawns Qt's own `qml6` runtime against `qml/`. No Qt/Rust binding crate: the QML talks to the daemon's HTTP API directly. Also runs in `--background` mode as a login-time notification watcher.
- **`qml/`** — the QML UI itself.

The daemon has no built-in TLS: for a real remote deployment, put it behind a TLS-terminating reverse proxy (or a VPN) rather than exposing it directly. See `daemon`'s doc comments for the API's authentication model.

## Development

```sh
cargo build --workspace
cargo test --workspace
```

## License

GPL-3.0-or-later, see [LICENSE](LICENSE).
