#!/usr/bin/env bash

# Maintainer: Shobhit Parvan <shobhit.gdsc.ai@gmail.com>
pkgname=duplicates-analyzer
pkgver=1.0.0
pkgrel=1
pkgdesc="A sleek, multi-threaded GUI application to find and clean up duplicate files via MD5 hashes"
arch=('x86_64')
url="https://github.com/jodi42-shb/duplicates" # Points to your public source repo
license=('BSL 1.1')
depends=('gcc-libs' 'gtk3') # gtk3 is required by rfd for the file picker dialog
makedepends=('cargo')
source=("${pkgname}-${pkgver}.tar.gz::${url}/archive/v${pkgver}.tar.gz")
sha256sums=('PLACEHOLDER_SHA256_SUM')

prepare() {
  cd "duplicates-${pkgver}"
  export CARGO_HOME="${srcdir}/cargo-home"
  cargo fetch --locked --target "$CARCH-unknown-linux-gnu"
}

build() {
  cd "duplicates-${pkgver}"
  export CARGO_HOME="${srcdir}/cargo-home"
  cargo build --frozen --release
}

package() {
  cd "duplicates-${pkgver}"

  # 1. Install the compiled binary executable
  install -Dm755 "target/release/duplicates" "${pkgdir}/usr/bin/${pkgname}"

  # 2. Install the desktop menu entry entry
  # Note: Ensure your repo contains a .desktop file or add it here manually
  install -Dm644 "duplicates.desktop" "${pkgdir}/usr/share/applications/${pkgname}.desktop"
}
