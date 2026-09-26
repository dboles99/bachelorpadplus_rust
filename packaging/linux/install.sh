#!/usr/bin/env bash
# Installs BachelorPad+ from this unpacked release tarball.
#
# Usage:
#   ./install.sh [--prefix DIR] [--uninstall]
#
#   --prefix DIR   install under DIR (default: ~/.local, so no root is needed;
#                  use /usr/local for every user of the machine)
#   --uninstall    remove what this tarball installs from DIR instead
#
# It copies bin/ and share/ under the prefix -- `bachelorpad`, its short name
# `bpad`, the menu entry, the file-type definition, the AppStream metadata,
# the icons and the licences -- and then refreshes the desktop's caches for
# that prefix where the tools for it are installed.
#
# The program needs the libraries a package would declare; on Debian or
# Ubuntu:
#   sudo apt install libfontconfig1 libxkbcommon-x11-0 libx11-xcb1 \
#       libxcursor1 libxi6 libwayland-client0 libdbus-1-3 xdg-desktop-portal-gtk
# A .deb, .rpm or the AUR package installs them for you and is the better
# choice where one fits.

set -euo pipefail

here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
prefix=${HOME:?HOME is not set}/.local
uninstall=0

while (($#)); do
    case $1 in
        --prefix) prefix=${2:?--prefix needs a directory}; shift 2 ;;
        --uninstall) uninstall=1; shift ;;
        -h | --help) sed -n '2,/^$/s/^# \{0,1\}//p' "$0"; exit 0 ;;
        *) echo "install.sh: unknown argument: $1" >&2; exit 2 ;;
    esac
done
[[ $prefix == /* ]] || prefix=$PWD/$prefix
app_id=io.github.dboles99.BachelorPad
desktop=share/applications/$app_id.desktop

# Every file and link this tarball carries under bin/ and share/, relative.
mapfile -t entries < <(cd "$here" && find bin share \( -type f -o -type l \) | LC_ALL=C sort)
[[ ${#entries[@]} -gt 0 ]] || { echo "install.sh: run it from the unpacked tarball" >&2; exit 2; }

refresh_caches() {
    # The same refreshes a distribution's triggers run for a package.
    # Without update-mime-database the .bpadx type is unknown to the
    # desktop; without update-desktop-database the menu entry is not
    # offered in Open With until something else rebuilds the cache.
    if command -v update-mime-database >/dev/null && [[ -d $prefix/share/mime ]]; then
        update-mime-database "$prefix/share/mime" || true
    fi
    if command -v update-desktop-database >/dev/null && [[ -d $prefix/share/applications ]]; then
        update-desktop-database -q "$prefix/share/applications" || true
    fi
    # gtk-update-icon-cache needs the theme's index.theme, which a system
    # hicolor has and ~/.local/share/icons/hicolor does not; without a cache
    # the icons are still found, only more slowly.
    if command -v gtk-update-icon-cache >/dev/null && [[ -f $prefix/share/icons/hicolor/index.theme ]]; then
        gtk-update-icon-cache -q "$prefix/share/icons/hicolor" || true
    fi
}

if ((uninstall)); then
    for rel in "${entries[@]}"; do
        rm -f "$prefix/$rel"
    done
    rmdir --ignore-fail-on-non-empty "$prefix/share/doc/bachelorpad" 2>/dev/null || true
    refresh_caches
    echo "Removed BachelorPad+ from $prefix."
    exit 0
fi

for rel in "${entries[@]}"; do
    mkdir -p "$prefix/$(dirname -- "$rel")"
    # -P keeps bpad a symlink; -f replaces an older install's files.
    cp -P -f "$here/$rel" "$prefix/$rel"
done

# The menu entry runs `bachelorpad` through PATH. If the prefix's bin/ is not
# on PATH the menu would hide the entry (TryExec fails), so point it at the
# installed binary instead.
case ":$PATH:" in
    *":$prefix/bin:"*) ;;
    *)
        sed -i -e "s|^Exec=bachelorpad |Exec=$prefix/bin/bachelorpad |" \
            -e "s|^TryExec=bachelorpad$|TryExec=$prefix/bin/bachelorpad|" "$prefix/$desktop"
        echo "Note: $prefix/bin is not on PATH; the menu entry uses the full path, and"
        echo "      'bachelorpad' and 'bpad' work in a shell once it is added to PATH."
        ;;
esac

refresh_caches
echo "Installed BachelorPad+ to $prefix: run 'bachelorpad', or 'bpad' for short."
echo "To remove it: $here/install.sh --uninstall --prefix $prefix"
