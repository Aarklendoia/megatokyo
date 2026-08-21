#!/bin/sh
# Run this locally, with network access, before building a Launchpad source
# upload (debuild -S -sa). NEVER run as part of debian/rules — Launchpad's
# build farm has no general internet access, so everything this script
# fetches must already be sitting in the working directory by the time
# dpkg-source tars it up.
#
# Unlike linux-hello/kio-protondrive, this workspace is pure Cargo (no
# CMake/Corrosion, no build.rs downloading model files) so there is exactly
# one thing to vendor: crates.io dependencies.
#
# IMPORTANT: vendor with the SAME cargo version the target Ubuntu series
# ships (check with `rmadison -u ubuntu cargo`), not whatever's locally
# "stable" — a newer cargo vendoring the tree can silently omit
# Cargo.toml.orig companion files an OLDER cargo needs at build time to
# verify a vendored crate's checksum against Cargo.lock (hit for real on
# linux-hello, see its own copy of this script). Set RUST_TOOLCHAIN to the
# version to vendor with; defaults to "stable", which is very likely wrong
# for an older LTS target — pass the real one explicitly:
#   RUST_TOOLCHAIN=1.93.1 ./debian/scripts/prepare-offline-build.sh
#
# See docs/LAUNCHPAD.md for how this fits into the release process.

set -eu

RUST_TOOLCHAIN="${RUST_TOOLCHAIN:-stable}"

cd "$(dirname "$0")/../.."

echo "==> Removing target/ (must not exist when dpkg-source tars up the tree)"
rm -rf target

echo "==> Vendoring Cargo dependencies into vendor/ (toolchain: $RUST_TOOLCHAIN)"
if [ "$RUST_TOOLCHAIN" = "stable" ]; then
  echo "    WARNING: no RUST_TOOLCHAIN given, using 'stable' — check the target" >&2
  echo "    series' cargo version first (rmadison -u ubuntu cargo) and pass" >&2
  echo "    RUST_TOOLCHAIN=<version> explicitly if it differs." >&2
fi
rustup toolchain install "$RUST_TOOLCHAIN" > /dev/null 2>&1 || true
rm -rf vendor .cargo
cargo "+$RUST_TOOLCHAIN" vendor vendor > /tmp/cargo-vendor-config.toml.tmp
mkdir -p .cargo
cat /tmp/cargo-vendor-config.toml.tmp > .cargo/config.toml
rm -f /tmp/cargo-vendor-config.toml.tmp
echo "    $(du -sh vendor | cut -f1) in vendor/, $(find vendor -name '*.orig' | wc -l) .orig files"

echo "==> Disabling cargo's per-file checksum verification for vendored crates"
# dpkg-source's native-tarball builder has a hardcoded exclude list (VCS
# control files, backup/swap files — .git, .gitignore, .svn, CVS, *.orig,
# DEADJOE, ...) that cannot be turned off via debian/source/options. Some
# vendored crate, somewhere, will always have a test fixture or metadata
# file that happens to match one of those generic names — dpkg-source
# silently drops it from the tarball, and cargo's offline build then fails
# verifying that crate's per-file checksums against .cargo-checksum.json.
# Documented Debian Rust-packaging fix: blank out each vendored crate's
# "files" checksum map so cargo only trusts the vendor directory as-is (the
# "package" checksum, verified against Cargo.lock, is untouched). Same fix
# as linux-hello's and kio-protondrive's own copy of this script.
find vendor -maxdepth 2 -name ".cargo-checksum.json" |
  while IFS= read -r f; do
    jq '.files = {}' "$f" > "$f.tmp" && mv "$f.tmp" "$f"
  done

cat <<EOF

Ready. From this same working directory (with vendor/ and .cargo/config.toml
populated), debian/rules will build with 'cargo build --offline'. Proceed
with the dch / debuild -S -sa / dput cycle from docs/LAUNCHPAD.md.

Nothing here is meant to be committed to git — vendor/ and .cargo/ are
regenerated per release right before packaging (see .gitignore).
EOF
