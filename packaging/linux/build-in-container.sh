#!/usr/bin/env bash
# Builds the Linux release packages inside the pinned Debian 12 container
# (release plan M3-02, decision 19). This is what the release script (M3-03)
# calls on Linux.
#
# Usage:
#   packaging/linux/build-in-container.sh --out DIR [options] [-- ENTRYPOINT-ARGS...]
#
#   --out DIR            where the packages go (created if missing)
#   --target-dir DIR     a host directory for cargo's target directory, kept
#                        between runs (default: the docker volume
#                        bachelorpad-release-target)
#   --cargo-cache DIR    a host directory for cargo's registry cache (default:
#                        the docker volume bachelorpad-release-registry)
#   --base-image IMAGE   build the image from IMAGE instead of the pinned
#                        Debian 12 (only to prove the Dockerfile on a mirror
#                        that can be reached; never for a release)
#   --extra-ca FILE      trust FILE's CA certificates too, for a build host
#                        behind a TLS-intercepting proxy; with it, apt uses
#                        https and HTTPS_PROXY from the environment is passed on
#   -- ARGS              passed to release-entrypoint.sh (e.g. --require-notices)
#
# The repository is mounted read-only; only the output and target
# directories are writable. The image is tagged with a hash of the
# Dockerfile, so a changed Dockerfile builds a new image and an unchanged one
# is reused.
#
# Exit status: that of the container (0 packages written and checked).

set -euo pipefail

here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo=$(cd -- "$here/../.." && pwd)

out=
target_mount=bachelorpad-release-target
registry_mount=bachelorpad-release-registry
base_image=
extra_ca=
entry_args=()

die() { echo "build-in-container: $*" >&2; exit 2; }

while (($#)); do
    case $1 in
        --out) out=${2:?--out needs a directory}; shift 2 ;;
        --target-dir) target_mount=${2:?--target-dir needs a directory}; shift 2 ;;
        --cargo-cache) registry_mount=${2:?--cargo-cache needs a directory}; shift 2 ;;
        --base-image) base_image=${2:?--base-image needs an image}; shift 2 ;;
        --extra-ca) extra_ca=${2:?--extra-ca needs a file}; shift 2 ;;
        --) shift; entry_args=("$@"); break ;;
        -h | --help) sed -n '2,/^$/s/^# \{0,1\}//p' "$0"; exit 0 ;;
        *) die "unknown argument: $1" ;;
    esac
done
[[ -n $out ]] || die "--out is required"
command -v docker >/dev/null || die "docker is not on PATH"
mkdir -p "$out"
out=$(cd -- "$out" && pwd)
# A host directory must be absolute for docker to take it as a bind mount
# rather than a volume name.
for var in target_mount registry_mount; do
    if [[ ${!var} == */* ]]; then
        mkdir -p "${!var}"
        printf -v "$var" '%s' "$(cd -- "${!var}" && pwd)"
    fi
done

build_args=()
run_args=()
if [[ -n $base_image ]]; then
    build_args+=(--build-arg "BASE_IMAGE=$base_image")
fi
if [[ -n $extra_ca ]]; then
    [[ -r $extra_ca ]] || die "cannot read $extra_ca"
    build_args+=(--secret "id=extra-ca,src=$extra_ca" --build-arg APT_HTTPS=1)
fi
proxy=${HTTPS_PROXY:-${https_proxy:-}}
if [[ -n $proxy ]]; then
    # A proxy on the host's loopback is reachable from a container only on
    # the host's network.
    build_args+=(--network host --build-arg "HTTPS_PROXY=$proxy" --build-arg "https_proxy=$proxy")
    run_args+=(--network host -e "HTTPS_PROXY=$proxy" -e "https_proxy=$proxy")
fi

tag=bachelorpad-release:$( { cat "$here/Dockerfile.release"; echo "$base_image"; } | sha256sum | cut -c1-12)
if ! docker image inspect "$tag" >/dev/null 2>&1; then
    DOCKER_BUILDKIT=1 docker build "${build_args[@]}" -f "$here/Dockerfile.release" -t "$tag" "$here"
fi

# The commit time, from the host: in a git worktree the container cannot
# read the repository's history (see release-entrypoint.sh).
sde=${SOURCE_DATE_EPOCH:-$(git -C "$repo" log -1 --format=%ct 2>/dev/null || true)}
[[ -n $sde ]] && run_args+=(-e "SOURCE_DATE_EPOCH=$sde")

exec docker run --rm "${run_args[@]}" \
    -v "$repo:/src:ro" \
    -v "$out:/out" \
    -v "$target_mount:/build/target" \
    -v "$registry_mount:/usr/local/cargo/registry" \
    "$tag" "${entry_args[@]}"
