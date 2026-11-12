#!/bin/sh
# keyup installer. usage:
#   curl -fsSL https://raw.githubusercontent.com/dhitalkamal/keyup/main/install.sh | sh
#
# apple silicon macs get the prebuilt release binary (checksum verified);
# everything else builds from source with cargo. never uses sudo, only writes
# to the install folder (default ~/.local/bin).
#
# overrides: KEYUP_VERSION=v0.3.0 pins a release, KEYUP_INSTALL_DIR picks
# the folder. KEYUP_BASE_URL and KEYUP_PLATFORM exist for the tests.
# pinning works for v0.3.0 and later (older releases shipped homerow assets).
set -eu

REPO="dhitalkamal/keyup"
ASSET="keyup-aarch64-apple-darwin.tar.gz"
VERSION="${KEYUP_VERSION:-latest}"
DIR="${KEYUP_INSTALL_DIR:-$HOME/.local/bin}"
PLATFORM="${KEYUP_PLATFORM:-$(uname -s)-$(uname -m)}"

if [ "$VERSION" = "latest" ]; then
    BASE="${KEYUP_BASE_URL:-https://github.com/$REPO/releases/latest/download}"
else
    BASE="${KEYUP_BASE_URL:-https://github.com/$REPO/releases/download/$VERSION}"
fi

say() { printf 'keyup: %s\n' "$1"; }
fail() { printf 'keyup: %s\n' "$1" >&2; exit 1; }

# no prebuilt binary for this platform: build from source if cargo is here
from_source() {
    if command -v cargo >/dev/null 2>&1; then
        say "no prebuilt binary for $PLATFORM, building from source with cargo"
        cargo install --locked --git "https://github.com/$REPO"
        say "installed keyup with cargo. run: keyup"
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

command -v curl >/dev/null 2>&1 || fail "curl is needed to download keyup"
command -v tar >/dev/null 2>&1 || fail "tar is needed to unpack keyup"

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
[ -f "$TMP/keyup" ] || fail "the download has no keyup binary, nothing installed"

# downloads are quarantined by macos; the binary is not apple signed
if command -v xattr >/dev/null 2>&1; then
    xattr -d com.apple.quarantine "$TMP/keyup" 2>/dev/null || true
fi

# copy under a temp name, then rename, so a failed copy never leaves a
# broken keyup behind (an older one stays until the new one is complete)
mkdir -p "$DIR"
cp "$TMP/keyup" "$DIR/.keyup.new"
chmod 755 "$DIR/.keyup.new"
mv -f "$DIR/.keyup.new" "$DIR/keyup"
say "installed keyup to $DIR/keyup"

# the app used to be called homerow; never delete it, just point it out
if [ -e "$DIR/homerow" ]; then
    say "the old homerow command is still at $DIR/homerow, remove it when you like: rm $DIR/homerow"
fi

case ":$PATH:" in
    *":$DIR:"*)
        say "run: keyup"
        exit 0
        ;;
esac

# the folder is not on PATH yet: add it to the shell's startup file, once.
# KEYUP_NO_MODIFY_PATH=1 skips this and only prints what to add.
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
    say "or run it now with: $DIR/keyup"
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

if [ -n "${KEYUP_NO_MODIFY_PATH:-}" ] || [ -z "$PROFILE" ]; then
    manual
    exit 0
fi

# never add it twice, even across reinstalls
if [ -f "$PROFILE" ] && grep -qF "$SHOWN" "$PROFILE"; then
    say "$PROFILE already adds $SHOWN to PATH. open a new terminal, then run: keyup"
    exit 0
fi

mkdir -p "$(dirname "$PROFILE")"
printf '\n# added by the keyup installer\n%s\n' "$LINE" >> "$PROFILE"
say "added $SHOWN to your PATH in $PROFILE"
say "open a new terminal (or run: . $PROFILE), then run: keyup"
