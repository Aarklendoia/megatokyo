# Publishing to Launchpad (PPA)

Step-by-step guide to publish megatokyo as a Launchpad PPA
(`ppa:aarklendoia-edtech/megatokyo`), so users can `apt install` it directly
instead of downloading `.deb` files from GitHub Releases. This mirrors
[linux-hello](https://github.com/Aarklendoia/linux-hello)'s and
[kio-protondrive](https://github.com/Aarklendoia/kio-protondrive)'s process
(see their own `docs/LAUNCHPAD.md`) — same Launchpad account, deliberately a
**separate PPA** so the three unrelated projects don't share one archive.

- Launchpad account: `aarklendoia-edtech` (already exists, reused from
  linux-hello/kio-protondrive)
- Personal signing key (manual uploads): reuse the existing
  `86EB1CE672402B0B104049C3D4251A0893FE3895` (`aarklendoia@proton.me`),
  already confirmed on the account with the Code of Conduct signed — no new
  personal key needed.
- CI signing key (automated uploads): generated and registered on the
  `aarklendoia-edtech` account, fingerprint
  `463A3894A7CAC3F707B93B2E37C96955509787D3` (`aarklendoia+ci@proton.me`,
  UID `megatokyo CI`) — a new, project-specific key, deliberately not
  reusing linux-hello's or kio-protondrive's CI key, so a leaked secret in
  one repo's Actions only compromises that one PPA. RSA 4096, sign-only, no
  passphrase (required for unattended CI signing), expires 2028-08-20.
  Private key added as the `PPA_GPG_PRIVATE_KEY` repository secret.
- PPA: `ppa:aarklendoia-edtech/megatokyo`
  (<https://launchpad.net/~aarklendoia-edtech/+archive/ubuntu/megatokyo>)
- PPA description (Launchpad's own "Change details" field, not something
  `debian/control` or this repo governs — edit it there if it ever needs
  updating):

  ```text
  Unofficial Megatokyo webcomic reader for Linux. A Qt/QML desktop client
  (megatokyo-gui) backed by a scraper/API daemon (megatokyo-daemon) that
  caches strips and Fred Gallagher's rants locally and translates rants
  on demand via DeepL.

  https://github.com/Aarklendoia/megatokyo
  ```

Supported Ubuntu series (one PPA upload each per release):

| Series | Codename | Packaged cargo/rustc |
| ------ | -------- | -------------------- |
| 26.04 LTS | `resolute` | 1.93.1 |
| 26.10 | `stonking` | 1.97.1 |

Adding or dropping a series means updating this table and the job matrix in
[publish-ppa.yml](../.github/workflows/publish-ppa.yml).

Launchpad's build farm has no general internet access, so the plain `cargo
build --release` in `debian/rules` would fail there — see
[Vendoring](#2-vendoring-required-before-every-ppa-upload) for how that's
handled.

## 1. One-time setup (manual, on launchpad.net)

None of this can be automated from a script — it requires a browser and your
Launchpad identity.

1. Create the PPA: your Launchpad profile page
   (`https://launchpad.net/~aarklendoia-edtech`) → "Create a new PPA" → name
   it `megatokyo`, public visibility.
2. Generate a new, dedicated CI signing key (don't reuse the other
   projects'): done — `463A3894A7CAC3F707B93B2E37C96955509787D3`.
3. On your Launchpad profile → "OpenPGP keys" → import the new fingerprint
   as an *additional* key on the same `aarklendoia-edtech` account. Launchpad
   emails a confirmation you must decrypt (`gpg --decrypt`) and follow the
   link in, or `gpg --clearsign` the confirmation text it shows on the page
   directly — done.
4. Install the upload tooling locally, if not already present:

   ```bash
   sudo apt install devscripts dput debhelper lintian gnupg
   ```

5. Private key added as the `PPA_GPG_PRIVATE_KEY` GitHub Actions repository
   secret — done:

   ```bash
   gpg --armor --export-secret-keys 463A3894A7CAC3F707B93B2E37C96955509787D3
   # paste the output into the GitHub repo secret
   ```

## 2. Vendoring (required before every PPA upload)

Launchpad's builders can't reach crates.io during a build. Unlike
linux-hello/kio-protondrive, this workspace is pure Cargo (no
CMake/Corrosion, no build.rs downloading model files) for `megatokyo-daemon`
and `megatokyo-gui` — but `megatokyo-pwa` (a Leptos/WASM bundle trunk
assembles) needs a lot more: `trunk` and `wasm-bindgen-cli` aren't apt
packages, and Ubuntu's own apt-packaged rustc cannot build `std` for
`wasm32-unknown-unknown` at all, prebuilt or from source (see the script's
own comments for the two things that were tried and why both are
fundamentally impossible against Ubuntu's rustc, not just inconvenient).
One command handles everything:

```bash
# Check the target series' packaged cargo version first:
rmadison -u ubuntu cargo | grep -E 'resolute|stonking'

RUST_TOOLCHAIN=1.93.1 ./debian/scripts/prepare-offline-build.sh   # resolute
RUST_TOOLCHAIN=1.97.1 ./debian/scripts/prepare-offline-build.sh   # stonking
```

Vendor again for each series, right before its upload: each series
needs a `vendor/` tree made with its own cargo version.

This vendors Cargo dependencies into `vendor/` (+ `.cargo/config.toml`) for
the main workspace, pre-populates a cargo registry cache for building
`trunk`/`wasm-bindgen-cli` from source, and vendors a complete official
rustc+cargo+std toolchain (host **and** wasm32 target) used only for
`pwa/`'s build — `megatokyo-daemon`/`megatokyo-gui`/`megatokyo-core` still
build with the apt rustc, untouched, and RUST_TOOLCHAIN's version only
matters for *that* vendoring: see the script's comments for **why the
toolchain version matters** (a newer local cargo can vendor a tree an
older, series-packaged cargo can't verify — already hit and documented in
linux-hello's equivalent script). The vendored toolchain used for `pwa/` is
self-contained and never mixed with the apt rustc, so it isn't subject to
that same version-matching requirement.
None of `vendor/`, `.cargo/config.toml`, `debian/build-tools/cargo-home/`,
or `debian/vendor-rust-toolchain/` are git-ignored by accident: they're
regenerated per release, not part of normal `main` history. Since
`debian/source/format` is `3.0 (native)`,
`debuild -S` tars up whatever is physically present at that moment,
`.gitignore` notwithstanding — except files named exactly `*.so`/`*.a`/
`*.o`/`*.la`, which "3.0 (native)" silently drops unconditionally (a
dpkg-source default, confirmed unrelated to `.gitignore` and not
reachable through `--tar-ignore`/`debian/source/include-binaries` for this
format); `prepare-offline-build.sh`'s last step renames every such file it
just vendored to dodge this, and `debian/rules` renames them back before
building.

## 3. Building and uploading a release

One **source-only** upload per target Ubuntu series:

```bash
dch --newversion "0.1.0~ppa1~resolute1" --distribution resolute --urgency medium \
  "Automated PPA build for resolute, release 0.1.0."

debuild -S -sa -k463A3894A7CAC3F707B93B2E37C96955509787D3

dput ppa:aarklendoia-edtech/megatokyo ../megatokyo_0.1.0~ppa1~resolute1_source.changes
```

Repeat with `stonking` in place of `resolute`, after vendoring again with
that series' toolchain (and dropping the resolute `dch` entry, e.g.
`git checkout debian/changelog`). `~ppa1~stonking1` sorts after
`~ppa1~resolute1`, so a system upgraded from 26.04 to 26.10 also upgrades to
the stonking build.

Track build status at
`https://launchpad.net/~aarklendoia-edtech/+archive/ubuntu/megatokyo/+packages`.

## 4. Automated publishing (CI)

[.github/workflows/publish-ppa.yml](../.github/workflows/publish-ppa.yml)
automates the cycle above on every `vX.Y.Z` release tag (same trigger as
`build-debian.yml`), with one matrix job per supported series. It can also
run manually via `workflow_dispatch` for a single series (codename + cargo
version as inputs). Needs the PPA to
exist (step 1 above) before it can succeed.

## 5. Once published

Add the `add-apt-repository ppa:aarklendoia-edtech/megatokyo` /
`apt install megatokyo-daemon megatokyo-gui megatokyo-pwa` instructions to
the README's install section and a Launchpad badge to the badge row.
