#!/bin/sh
# home row installer. usage:
#   curl -fsSL https://raw.githubusercontent.com/dhitalkamal/homerow/main/install.sh | sh
#
# apple silicon macs get the prebuilt release binary (checksum verified);
# everything else builds from source with cargo. never uses sudo, only writes
# to the install folder (default ~/.local/bin).
#
# overrides: HOMEROW_VERSION=v0.1.0 pins a release, HOMEROW_INSTALL_DIR picks
# the folder. HOMEROW_BASE_URL and HOMEROW_PLATFORM exist for the tests.
set -eu

REPO="dhitalkamal/homerow"
ASSET="homerow-aarch64-apple-darwin.tar.gz"
VERSION="${HOMEROW_VERSION:-latest}"
DIR="${HOMEROW_INSTALL_DIR:-$HOME/.local/bin}"
PLATFORM="${HOMEROW_PLATFORM:-$(uname -s)-$(uname -m)}"

if [ "$VERSION" = "latest" ]; then
    BASE="${HOMEROW_BASE_URL:-https://github.com/$REPO/releases/latest/download}"
else
    BASE="${HOMEROW_BASE_URL:-https://github.com/$REPO/releases/download/$VERSION}"
fi

say() { printf 'homerow: %s\n' "$1"; }
fail() { printf 'homerow: %s\n' "$1" >&2; exit 1; }

# no prebuilt binary for this platform: build from source if cargo is here
from_source() {
    if command -v cargo >/dev/null 2>&1; then
        say "no prebuilt binary for $PLATFORM, building from source with cargo"
        cargo install --locked --git "https://github.com/$REPO"
        say "installed homerow with cargo. run: homerow"
        exit 0
    fi
    fail "no prebuilt binary for $PLATFORM and cargo is not installed.
install rust from https://rustup.rs (curl https://sh.rustup.rs -sSf | sh),
then run this installer again."
}

case "$PLATFORM" in
    Darwin-arm64) ;;
    *) from_source ;;
esac

command -v curl >/dev/null 2>&1 || fail "curl is needed to download homerow"
command -v tar >/dev/null 2>&1 || fail "tar is needed to unpack homerow"

# work in a temp folder that is removed however the script ends
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT INT TERM

say "downloading $VERSION release"
curl -fsSL "$BASE/$ASSET" -o "$TMP/$ASSET" || fail "download failed, nothing installed"
curl -fsSL "$BASE/$ASSET.sha256" -o "$TMP/$ASSET.sha256" || fail "checksum download failed, nothing installed"

# the tarball must match the published checksum before anything is installed
want=$(cut -d ' ' -f 1 < "$TMP/$ASSET.sha256")
if command -v shasum >/dev/null 2>&1; then
    have=$(shasum -a 256 "$TMP/$ASSET" | cut -d ' ' -f 1)
elif command -v sha256sum >/dev/null 2>&1; then
    have=$(sha256sum "$TMP/$ASSET" | cut -d ' ' -f 1)
else
    fail "no shasum or sha256sum to check the download, nothing installed"
fi
[ -n "$want" ] && [ "$want" = "$have" ] || fail "checksum mismatch, the download is broken or changed. nothing installed"

tar xzf "$TMP/$ASSET" -C "$TMP" || fail "could not unpack the download, nothing installed"
[ -f "$TMP/homerow" ] || fail "the download has no homerow binary, nothing installed"

# downloads are quarantined by macos; the binary is not apple signed
if command -v xattr >/dev/null 2>&1; then
    xattr -d com.apple.quarantine "$TMP/homerow" 2>/dev/null || true
fi

# copy under a temp name, then rename, so a failed copy never leaves a
# broken homerow behind (an older one stays until the new one is complete)
mkdir -p "$DIR"
cp "$TMP/homerow" "$DIR/.homerow.new"
chmod 755 "$DIR/.homerow.new"
mv -f "$DIR/.homerow.new" "$DIR/homerow"
say "installed homerow to $DIR/homerow"

case ":$PATH:" in
    *":$DIR:"*)
        say "run: homerow"
        exit 0
        ;;
esac

# the folder is not on PATH yet: add it to the shell's startup file, once.
# HOMEROW_NO_MODIFY_PATH=1 skips this and only prints what to add.
if [ "$DIR" = "$HOME/.local/bin" ]; then
    # shellcheck disable=SC2016
    SHOWN='$HOME/.local/bin'
else
    SHOWN="$DIR"
fi

manual() {
    say "$DIR is not on your PATH yet. add this line to your shell profile, then open a new terminal:"
    # $PATH stays literal on purpose: the user pastes this into their profile
    # shellcheck disable=SC2016
    printf '    export PATH="%s:$PATH"\n' "$SHOWN"
    say "or run it now with: $DIR/homerow"
}

PROFILE=""
LINE=""
case "${SHELL:-}" in
    */zsh)
        PROFILE="${ZDOTDIR:-$HOME}/.zshrc"
        LINE="export PATH=\"$SHOWN:\$PATH\""
        ;;
    */bash)
        case "$PLATFORM" in
            Darwin-*) PROFILE="$HOME/.bash_profile" ;;
            *) PROFILE="$HOME/.bashrc" ;;
        esac
        LINE="export PATH=\"$SHOWN:\$PATH\""
        ;;
    */fish)
        PROFILE="$HOME/.config/fish/config.fish"
        LINE="fish_add_path $SHOWN"
        ;;
esac

if [ -n "${HOMEROW_NO_MODIFY_PATH:-}" ] || [ -z "$PROFILE" ]; then
    manual
    exit 0
fi

# never add it twice, even across reinstalls
if [ -f "$PROFILE" ] && grep -qF "$SHOWN" "$PROFILE"; then
    say "$PROFILE already adds $SHOWN to PATH. open a new terminal, then run: homerow"
    exit 0
fi

mkdir -p "$(dirname "$PROFILE")"
printf '\n# added by the homerow installer\n%s\n' "$LINE" >> "$PROFILE"
say "added $SHOWN to your PATH in $PROFILE"
say "open a new terminal (or run: . $PROFILE), then run: homerow"
