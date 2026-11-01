# Maintainer: Asif Arbaj <asifarbaj@gmail.com>
_name=weather
pkgname=totoro-weather
pkgver=r0
pkgrel=1
pkgdesc="Weather from Open-Meteo as JSON: a Waybar module, or a whole forecast with --forecast"
arch=('x86_64')
url="https://github.com/anisthdev/weather"
provides=('weather')
conflicts=('weather')
replaces=('weather')
license=('unknown')
depends=('dbus' 'curl' 'gcc-libs' 'glibc')
optdepends=('geoclue: location from GeoClue rather than the IP address')
makedepends=('git' 'cargo')
source=("git+$url.git")
sha256sums=('SKIP')

pkgver() {
    cd "$_name"
    printf "r%s.%s" "$(git rev-list --count HEAD)" "$(git rev-parse --short=7 HEAD)"
}

prepare() {
    cd "$_name"
    export RUSTUP_TOOLCHAIN=stable
    cargo fetch --locked --target "$(rustc -vV | sed -n 's/host: //p')"
}

build() {
    cd "$_name"
    export RUSTUP_TOOLCHAIN=stable
    cargo build --frozen --release
}

package() {
    cd "$_name"
    install -Dm755 target/release/weather "$pkgdir/usr/bin/weather"
    install -Dm644 README.md "$pkgdir/usr/share/doc/$_name/README.md"
    install -Dm644 waybar-weather.css "$pkgdir/usr/share/$_name/waybar-weather.css"
    cp -r --no-preserve=ownership icons "$pkgdir/usr/share/$_name/"
}
