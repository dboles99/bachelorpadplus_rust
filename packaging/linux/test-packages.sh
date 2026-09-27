#!/usr/bin/env bash
# Installs the release packages in clean containers of the distributions they
# promise, runs them, removes them and checks nothing is left (release plan
# M3-02). There is no CI; this is the run a human does before a release.
#
# Usage:
#   packaging/linux/test-packages.sh --dir DIR [--distro NAME]... [--extra-ca FILE]
#
#   --dir DIR        the output directory of build-packages.sh (the .deb, the
#                    .rpm, the tarball and PKGBUILD)
#   --distro NAME    one of ubuntu-22.04, ubuntu-24.04, debian-12, fedora,
#                    arch; repeatable (default: ubuntu-22.04 ubuntu-24.04
#                    fedora arch)
#   --extra-ca FILE  for a host behind a TLS-intercepting proxy: trust FILE in
#                    the containers, switch apt to https and pass HTTPS_PROXY
#
# In each container: install the package with only its hard dependencies
# (no Recommends, no weak deps -- so a missing Depends shows up), then
#   * `bachelorpad --self-check` under xvfb-run (the loader resolves every
#     linked library),
#   * `bpad --version` (the M2-16 symlink),
#   * the window opens and stays up for five seconds under Xvfb (winit loads
#     xkbcommon-x11, X11 and the rest with dlopen only then),
#   * desktop-file-validate on the installed entry,
# then remove the package and assert no file of it, and no generated MIME or
# desktop-database entry for it, remains.
#
# The tools the checks use (xvfb-run, desktop-file-validate, shared-mime-info
# so the MIME trigger has something to run) are installed before the
# package, so they cannot hide a dependency the package forgot.
#
# Needs docker (or podman as `docker`) and network access to each
# distribution's mirrors. Exit status: 0 every distribution passed; 1 one
# failed; 2 a prerequisite was missing.

set -euo pipefail

dir=
distros=()
extra_ca=

die() { echo "test-packages: $*" >&2; exit 2; }

while (($#)); do
    case $1 in
        --dir) dir=${2:?--dir needs a directory}; shift 2 ;;
        --distro) distros+=("${2:?--distro needs a name}"); shift 2 ;;
        --extra-ca) extra_ca=${2:?--extra-ca needs a file}; shift 2 ;;
        -h | --help) sed -n '2,/^$/s/^# \{0,1\}//p' "$0"; exit 0 ;;
        *) die "unknown argument: $1" ;;
    esac
done
[[ -n $dir && -d $dir ]] || die "--dir must name build-packages.sh's output directory"
dir=$(cd -- "$dir" && pwd)
((${#distros[@]})) || distros=(ubuntu-22.04 ubuntu-24.04 fedora arch)
command -v docker >/dev/null || die "docker is not on PATH"

shopt -s nullglob
debs=("$dir"/bachelorpad_*_amd64.deb)
rpms=("$dir"/bachelorpad-*.x86_64.rpm)
tars=("$dir"/bachelorpad-*-x86_64-linux.tar.gz)
shopt -u nullglob

run_args=(--rm -v "$dir:/pkgs:ro")
if [[ -n $extra_ca ]]; then
    [[ -r $extra_ca ]] || die "cannot read $extra_ca"
    run_args+=(-v "$(cd -- "$(dirname -- "$extra_ca")" && pwd)/$(basename -- "$extra_ca"):/extra-ca.crt:ro" -e EXTRA_CA=/extra-ca.crt)
fi
proxy=${HTTPS_PROXY:-${https_proxy:-}}
if [[ -n $proxy ]]; then
    run_args+=(--network host -e "HTTPS_PROXY=$proxy" -e "https_proxy=$proxy")
fi

# The checks shared by every distribution, run inside the container after
# the install. $1 is the command that lists the package's files, $2 the one
# that removes it.
read -r -d '' common <<'EOF' || true
set -euo pipefail
list_cmd=$1
remove_cmd=$2
fail() { echo "FAIL  $*"; exit 1; }
pass() { echo "PASS  $*"; }

out=$(xvfb-run -a bachelorpad --self-check 2>&1) || fail "bachelorpad --self-check: $out"
[[ $out == *BACHELORPAD_SELF_CHECK_OK* ]] || fail "--self-check printed: $out"
pass "bachelorpad --self-check"

[[ -L /usr/bin/bpad && $(readlink /usr/bin/bpad) == bachelorpad ]] || fail "/usr/bin/bpad is not a symlink to bachelorpad"
bpad --version || fail "bpad --version"
pass "bpad --version"

# The window: winit loads xkbcommon-x11 and the X11 libraries with dlopen
# only when it opens, so --self-check cannot see a missing one. Alive after
# five seconds, with no panic, is the pass.
home=$(mktemp -d)
log=$(mktemp)
HOME=$home xvfb-run -a bash -c 'bachelorpad & pid=$!; sleep 5; kill -0 $pid || exit 1; kill $pid; wait $pid || true' >"$log" 2>&1 \
    || { cat "$log"; fail "the window did not stay up under Xvfb"; }
if grep -q -i panicked "$log"; then cat "$log"; fail "bachelorpad panicked under Xvfb"; fi
pass "window opens under Xvfb"

desktop-file-validate /usr/share/applications/io.github.dboles99.BachelorPad.desktop || fail "desktop-file-validate"
pass "desktop-file-validate"
[[ -f /usr/share/mime/application/x-bachelorpad-encrypted.xml ]] || fail "the MIME database was not updated on install"
pass "MIME database knows application/x-bachelorpad-encrypted"

$list_cmd > /tmp/pkg-files.txt
$remove_cmd
left=()
while IFS= read -r f; do
    # Directories the package shares with the system stay; files must go.
    if [[ -e $f || -L $f ]] && [[ ! -d $f ]]; then left+=("$f"); fi
done < /tmp/pkg-files.txt
[[ -e /usr/share/mime/application/x-bachelorpad-encrypted.xml ]] && left+=(/usr/share/mime/application/x-bachelorpad-encrypted.xml)
if [[ -f /usr/share/applications/mimeinfo.cache ]] && grep -q io.github.dboles99.BachelorPad /usr/share/applications/mimeinfo.cache; then
    left+=("/usr/share/applications/mimeinfo.cache (still names the entry)")
fi
((${#left[@]} == 0)) || fail "left after removal: ${left[*]}"
pass "removed cleanly"
EOF

# The container scripts are single-quoted on purpose: their $variables
# belong to the container's shell, not this one.
# shellcheck disable=SC2016
apt_prelude='
set -euo pipefail
export DEBIAN_FRONTEND=noninteractive
opts=()
if [[ -n ${EXTRA_CA:-} ]]; then
    sed -i "s#http://#https://#g" /etc/apt/sources.list.d/*.sources /etc/apt/sources.list 2>/dev/null || true
    opts=(-o "Acquire::https::CAInfo=$EXTRA_CA")
fi
apt-get "${opts[@]}" update -qq
# The tools first, so the package install below brings exactly its Depends.
apt-get "${opts[@]}" install -y -qq --no-install-recommends xvfb xauth desktop-file-utils shared-mime-info >/dev/null
apt-get "${opts[@]}" install -y -qq --no-install-recommends /pkgs/'

run_distro() {
    local distro=$1 image script
    case $distro in
        ubuntu-22.04 | ubuntu-24.04 | debian-12)
            ((${#debs[@]} == 1)) || die "expected one .deb in $dir"
            case $distro in
                ubuntu-22.04) image=ubuntu:22.04 ;;
                ubuntu-24.04) image=ubuntu:24.04 ;;
                debian-12) image=debian:12 ;;
            esac
            script="$apt_prelude$(basename -- "${debs[0]}") >/dev/null
bash -c \"\$COMMON\" _ 'dpkg -L bachelorpad' 'apt-get purge -y -qq bachelorpad'"
            docker run "${run_args[@]}" -e COMMON="$common" "$image" bash -c "$script" 2>&1 | sed "s/^/[$distro] /"
            return "${PIPESTATUS[0]}"
            ;;
        fedora)
            ((${#rpms[@]} == 1)) || die "expected one .rpm in $dir"
            image=fedora:latest
            # shellcheck disable=SC2016
            script='set -euo pipefail
if [[ -n ${EXTRA_CA:-} ]]; then cp "$EXTRA_CA" /etc/pki/ca-trust/source/anchors/ && update-ca-trust; fi
dnf -y -q install --setopt=install_weak_deps=False xorg-x11-server-Xvfb xorg-x11-xauth desktop-file-utils shared-mime-info >/dev/null
dnf -y -q install --setopt=install_weak_deps=False /pkgs/'"$(basename -- "${rpms[0]}")"' >/dev/null
bash -c "$COMMON" _ "rpm -ql bachelorpad" "dnf -y -q remove bachelorpad"'
            docker run "${run_args[@]}" -e COMMON="$common" "$image" bash -c "$script" 2>&1 | sed "s/^/[$distro] /"
            return "${PIPESTATUS[0]}"
            ;;
        arch)
            ((${#tars[@]} == 1)) || die "expected one tarball in $dir"
            [[ -f $dir/PKGBUILD ]] || die "no PKGBUILD in $dir"
            image=archlinux:latest
            # makepkg refuses to run as root, and finds the tarball beside
            # the PKGBUILD instead of downloading it from the release page.
            # shellcheck disable=SC2016
            script='set -euo pipefail
if [[ -n ${EXTRA_CA:-} ]]; then trust anchor --store "$EXTRA_CA"; fi
pacman -Syu --noconfirm --needed base-devel xorg-server-xvfb xorg-xauth desktop-file-utils shared-mime-info >/dev/null
useradd -m builder
install -d -o builder /home/builder/pkg
cp /pkgs/PKGBUILD /pkgs/'"$(basename -- "${tars[0]}")"' /home/builder/pkg/
chown -R builder /home/builder/pkg
su builder -c "cd /home/builder/pkg && makepkg --nodeps --noconfirm" >/dev/null
pacman -U --noconfirm /home/builder/pkg/bachelorpad-bin-*.pkg.tar.zst >/dev/null
bash -c "$COMMON" _ "pacman -Qlq bachelorpad-bin" "pacman -Rns --noconfirm bachelorpad-bin"'
            docker run "${run_args[@]}" -e COMMON="$common" "$image" bash -c "$script" 2>&1 | sed "s/^/[$distro] /"
            return "${PIPESTATUS[0]}"
            ;;
        *) die "unknown distribution: $distro" ;;
    esac
}

failed=()
for distro in "${distros[@]}"; do
    echo "=== $distro"
    if ! run_distro "$distro"; then
        failed+=("$distro")
    fi
done
if ((${#failed[@]})); then
    echo "test-packages: FAILED on ${failed[*]}"
    exit 1
fi
echo "test-packages: passed on ${distros[*]}"
