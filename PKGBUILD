# Maintainer: Kai <https://github.com/Kai-J-G>
pkgname=cosmic-ext-mini-taskmanager
pkgver=1.3.0
pkgrel=1
pkgdesc="Native mini task manager applet for the COSMIC Desktop Environment"
arch=('x86_64' 'aarch64')
url="https://github.com/Kai-J-G/cosmic-ext-mini-taskmanager"
license=('MIT')
depends=('cosmic-icon-theme' 'glibc' 'gcc-libs')
makedepends=('cargo' 'just')
source=()
sha256sums=()

build() {
  cd "$startdir"
  cargo build --release --locked
}

check() {
  cd "$startdir"
  cargo test --release --locked
}

package() {
  cd "$startdir"
  DESTDIR="$pkgdir" PREFIX="/usr" just install
  install -Dm0644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
  install -Dm0644 README.md "$pkgdir/usr/share/doc/$pkgname/README.md"
}
