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
CMake/Corrosion, no build.rs downloading model files), so there is exactly
one thing to vendor:

```bash
# Check the target series' packaged cargo version first:
rmadison -u ubuntu cargo | grep resolute

RUST_TOOLCHAIN=1.93.1 ./debian/scripts/prepare-offline-build.sh
```

This vendors Cargo dependencies into `vendor/` (+ `.cargo/config.toml`) —
see the script's comments for **why the toolchain version matters** (a
newer local cargo can vendor a tree an older, series-packaged cargo can't
verify — already hit and documented in linux-hello's equivalent script).
`vendor/` and `.cargo/` are git-ignored: regenerated per release, not part
of normal `main` history. Since `debian/source/format` is `3.0 (native)`,
`debuild -S` tars up whatever is physically present at that moment,
`.gitignore` notwithstanding.

## 3. Building and uploading a release

One **source-only** upload per target Ubuntu series:

```bash
dch --newversion "0.1.0~ppa1~resolute1" --distribution resolute --urgency medium \
  "Automated PPA build for resolute, release 0.1.0."

debuild -S -sa -k463A3894A7CAC3F707B93B2E37C96955509787D3

dput ppa:aarklendoia-edtech/megatokyo ../megatokyo_0.1.0~ppa1~resolute1_source.changes
```

Track build status at
`https://launchpad.net/~aarklendoia-edtech/+archive/ubuntu/megatokyo/+packages`.

## 4. Automated publishing (CI)

[.github/workflows/publish-ppa.yml](../.github/workflows/publish-ppa.yml)
automates the cycle above on every `vX.Y.Z` release tag (same trigger as
`build-debian.yml`), or manually via `workflow_dispatch`. Needs the PPA to
exist (step 1 above) before it can succeed.

## 5. Once published

Add the `add-apt-repository ppa:aarklendoia-edtech/megatokyo` /
`apt install megatokyo-daemon megatokyo-gui` instructions to the README's
install section and a Launchpad badge to the badge row.
