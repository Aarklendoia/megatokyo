# Changelog

## [0.2.0](https://github.com/Aarklendoia/megatokyo/compare/v0.1.0...v0.2.0) (2026-08-21)


### Features

* app icon and desktop integration ([#13](https://github.com/Aarklendoia/megatokyo/issues/13)) ([4eb782a](https://github.com/Aarklendoia/megatokyo/commit/4eb782a68f466a4a3bea3acaaaef654df20ad54e))
* **core,daemon:** favorites and reading progress ([#10](https://github.com/Aarklendoia/megatokyo/issues/10)) ([576e18b](https://github.com/Aarklendoia/megatokyo/commit/576e18b46c760dafd5fd9620c96ed4bf4f47dca9))
* **core:** DeepL translation with per-language cache ([e2d0de4](https://github.com/Aarklendoia/megatokyo/commit/e2d0de4e9437f1bf5db6856aeff259d0c9f8cc87))
* **core:** image cache for strip images ([dc41296](https://github.com/Aarklendoia/megatokyo/commit/dc41296440adbd28fd8bb403dd3ed8c67c79bee4))
* **core:** parse the RSS feed to detect new strips/rants ([3cf9133](https://github.com/Aarklendoia/megatokyo/commit/3cf9133242056f365ca0c9692f6dad14032cc6a2))
* **core:** scrape chapters, strips and rants off megatokyo.com ([edc918d](https://github.com/Aarklendoia/megatokyo/commit/edc918d7f61e0f9631eb854806fef42418459956))
* **core:** workspace scaffold, domain types, SQLite storage and local_ctrl ([81c050c](https://github.com/Aarklendoia/megatokyo/commit/81c050c3d79db801335f836bf30b4b073a908bdc))
* **daemon:** HTTP API and poll loop ([b9aa3b1](https://github.com/Aarklendoia/megatokyo/commit/b9aa3b13a5a1b4e3c466a36866a0d51cb9722393))
* **daemon:** HTTP API and poll loop ([43f7500](https://github.com/Aarklendoia/megatokyo/commit/43f7500fa2e62a4ce531392c4fb1120e9ae938cb))
* **gui:** windowed launcher and --background notification watcher ([#9](https://github.com/Aarklendoia/megatokyo/issues/9)) ([d839782](https://github.com/Aarklendoia/megatokyo/commit/d839782cce405fd6320b647dd009b09b51e92b82))
* **qml:** dashboard, reader, gallery and rants screens ([#12](https://github.com/Aarklendoia/megatokyo/issues/12)) ([f40c499](https://github.com/Aarklendoia/megatokyo/commit/f40c4997a32652d1c6d468d06e3e4cdb70828a35))


### Bug Fixes

* **core:** keep the RSS entry's link so rants can be fetched by it ([77e3789](https://github.com/Aarklendoia/megatokyo/commit/77e3789ea81571f692642e6d5a146b17b22074ad))
* **core:** satisfy clippy::question_mark in entry_to_item ([3a8c080](https://github.com/Aarklendoia/megatokyo/commit/3a8c080c2ac2f6b533b8859402f4d3182cb58ce1))
