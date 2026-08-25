# Changelog

## [0.7.1](https://github.com/Aarklendoia/megatokyo/compare/v0.7.0...v0.7.1) (2026-08-25)


### Bug Fixes

* **core:** reject invalid calendar dates in the scraper's date parser ([#61](https://github.com/Aarklendoia/megatokyo/issues/61)) ([e3f1bf8](https://github.com/Aarklendoia/megatokyo/commit/e3f1bf84919bd53575128192b01a83978519f5d2)), closes [#54](https://github.com/Aarklendoia/megatokyo/issues/54)
* **daemon:** checkpoint the newest new item, not the last one processed ([#58](https://github.com/Aarklendoia/megatokyo/issues/58)) ([32caa8b](https://github.com/Aarklendoia/megatokyo/commit/32caa8b5ffae58896da96b5845d0c382715bd376)), closes [#51](https://github.com/Aarklendoia/megatokyo/issues/51)
* **gui:** write the GUI config file with owner-only permissions ([#57](https://github.com/Aarklendoia/megatokyo/issues/57)) ([d21b704](https://github.com/Aarklendoia/megatokyo/commit/d21b704e51812f310946a13dffdd08d051cd0a71)), closes [#50](https://github.com/Aarklendoia/megatokyo/issues/50)


### Performance Improvements

* **daemon:** reuse one HTTP client for DeepL translation requests ([#59](https://github.com/Aarklendoia/megatokyo/issues/59)) ([be9d2ab](https://github.com/Aarklendoia/megatokyo/commit/be9d2abe87f6a95f4d43b34ba86ef95cbcbe92db)), closes [#52](https://github.com/Aarklendoia/megatokyo/issues/52)


### Code Refactoring

* **gui:** dedupe hand-rolled flat-TOML field parsers ([#60](https://github.com/Aarklendoia/megatokyo/issues/60)) ([7c362c7](https://github.com/Aarklendoia/megatokyo/commit/7c362c7fc2c71ef91b3aa2d51c73ece1d22db2f3)), closes [#53](https://github.com/Aarklendoia/megatokyo/issues/53)

## [0.7.0](https://github.com/Aarklendoia/megatokyo/compare/v0.6.0...v0.7.0) (2026-08-24)


### Features

* **gui:** persist the Reader's "All strips"/"Main story only" toggle ([#45](https://github.com/Aarklendoia/megatokyo/issues/45)) ([f6ec82e](https://github.com/Aarklendoia/megatokyo/commit/f6ec82e195ef708ca2f80c6f67cdbac05480a9b5))

## [0.6.0](https://github.com/Aarklendoia/megatokyo/compare/v0.5.1...v0.6.0) (2026-08-24)


### Features

* **qml:** improve Rants readability — reading width, search, scrollbar ([#43](https://github.com/Aarklendoia/megatokyo/issues/43)) ([45f8fd5](https://github.com/Aarklendoia/megatokyo/commit/45f8fd58b30f5d9bf2e561d829318464f47b9e32))


### Bug Fixes

* **qml:** pin the Rants list panel to a fixed width, keep "Rants" untranslated ([#40](https://github.com/Aarklendoia/megatokyo/issues/40)) ([5229475](https://github.com/Aarklendoia/megatokyo/commit/522947577d6479e3be7116f0d251071e20cd2dfa))

## [0.5.1](https://github.com/Aarklendoia/megatokyo/compare/v0.5.0...v0.5.1) (2026-08-24)


### Bug Fixes

* **core:** send the DeepL key as an Authorization header, not a form field ([#35](https://github.com/Aarklendoia/megatokyo/issues/35)) ([ab66ff0](https://github.com/Aarklendoia/megatokyo/commit/ab66ff044f8f28a49527b1fd1e1d4330bfd79162))
* **daemon:** stop gating the rant backfill on an empty table ([#38](https://github.com/Aarklendoia/megatokyo/issues/38)) ([c2d6794](https://github.com/Aarklendoia/megatokyo/commit/c2d679455af6b56d26c6b365fa71502a41f126b7))

## [0.5.0](https://github.com/Aarklendoia/megatokyo/compare/v0.4.0...v0.5.0) (2026-08-24)


### Features

* **qml:** keyboard and click-to-turn-page navigation in the Reader ([#31](https://github.com/Aarklendoia/megatokyo/issues/31)) ([d79b145](https://github.com/Aarklendoia/megatokyo/commit/d79b14548b334e0a01d3eb18eb1c6d8e19422fb2))
* **qml:** make Gallery thumbnails' favorite badge interactive ([#32](https://github.com/Aarklendoia/megatokyo/issues/32)) ([7917300](https://github.com/Aarklendoia/megatokyo/commit/7917300e8b1e92010c480b97a035f37d980fc102))
* **qml:** put main-story and bonus chips on two separate rows ([#33](https://github.com/Aarklendoia/megatokyo/issues/33)) ([1d20976](https://github.com/Aarklendoia/megatokyo/commit/1d209763290a9f4d934b4eb3971151df9373cb0c))


### Bug Fixes

* **qml:** Reader tab defaults to the last-read strip ([#26](https://github.com/Aarklendoia/megatokyo/issues/26)) ([6be5c10](https://github.com/Aarklendoia/megatokyo/commit/6be5c1020107ce7d4811ca09bcfb7174442fca29))

## [0.4.0](https://github.com/Aarklendoia/megatokyo/compare/v0.3.1...v0.4.0) (2026-08-24)


### Features

* **rants:** backfill every rant, not just the RSS feed's last 5 ([#23](https://github.com/Aarklendoia/megatokyo/issues/23)) ([d3754a8](https://github.com/Aarklendoia/megatokyo/commit/d3754a8a347e85d1e59c0c1a519ed72e272e356f))


### Bug Fixes

* **qml:** stop clipping the tops of letters in TextField boxes ([#25](https://github.com/Aarklendoia/megatokyo/issues/25)) ([3bbab5b](https://github.com/Aarklendoia/megatokyo/commit/3bbab5bb5c0ded6965b82f4c4f03b3d2ea4b4623))

## [0.3.1](https://github.com/Aarklendoia/megatokyo/compare/v0.3.0...v0.3.1) (2026-08-24)


### Bug Fixes

* **qml:** distinguish main-story chapters from bonus in the Gallery ([#20](https://github.com/Aarklendoia/megatokyo/issues/20)) ([9f20b8c](https://github.com/Aarklendoia/megatokyo/commit/9f20b8c94196b494ac232f63f167394b6e061ef7))
* **qml:** show Settings save errors in red, not the same teal as success ([#22](https://github.com/Aarklendoia/megatokyo/issues/22)) ([7d6002e](https://github.com/Aarklendoia/megatokyo/commit/7d6002eeb8020022c479f212f889655ce2c3120a))

## [0.3.0](https://github.com/Aarklendoia/megatokyo/compare/v0.2.0...v0.3.0) (2026-08-24)


### Features

* Settings screen (remote daemon config, DeepL key, notifications) ([#18](https://github.com/Aarklendoia/megatokyo/issues/18)) ([66682af](https://github.com/Aarklendoia/megatokyo/commit/66682af16c5481d19eeb44c2dd59a79fa9a23b15))


### Bug Fixes

* **ci:** grant contents:write so build-debian.yml can attach release assets ([#15](https://github.com/Aarklendoia/megatokyo/issues/15)) ([d90e89b](https://github.com/Aarklendoia/megatokyo/commit/d90e89b0b29d813a5d2181907aada8d0b957a5c9))
* **packaging:** resolve remaining lintian findings from [#6](https://github.com/Aarklendoia/megatokyo/issues/6) ([#17](https://github.com/Aarklendoia/megatokyo/issues/17)) ([2d2fef8](https://github.com/Aarklendoia/megatokyo/commit/2d2fef80eddb9c8702d70c26b24772bb01f7094b))

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
