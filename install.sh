#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Installs WallFlux, or updates it to the newest release: the same line does
# both. Arch Linux and systems built on it only.
#
#   curl -fsSL https://raw.githubusercontent.com/invisigoth-cz/WallFlux/main/install.sh | bash
#
# What it does: Valve's Steamworks SDK out of the AUR if it is missing (no
# AUR account or helper is needed to build from it), then the PKGBUILD of the
# newest WallFlux release, built with makepkg on this machine and installed
# as an ordinary pacman package -- `sudo pacman -R wallflux` removes it.
# `--noconfirm` after it (`| bash -s -- --noconfirm`) asks nothing at all;
# whatever is given there goes to pacman and makepkg alike.
#
# Everything is inside main(), so a download cut off half way runs nothing.

set -euo pipefail

repo=invisigoth-cz/WallFlux

say() { printf '\033[1m==> %s\033[0m\n' "$*"; }
die() { printf '\033[1;31m==> %s\033[0m\n' "$*" >&2; exit 1; }

main() {
    [[ $EUID -ne 0 ]] || die "Run this as yourself, not as root: makepkg refuses root and asks for sudo itself."
    command -v pacman >/dev/null || die "WallFlux installs on Arch Linux and systems built on it, and this has no pacman."

    # Piped from curl, the script itself is on stdin, so pacman's questions
    # are answered from the terminal instead.
    local answers=/dev/stdin
    if [[ ! -t 0 ]]; then
        [[ -r /dev/tty ]] || die "No terminal to ask questions in: download the script and run it with bash."
        answers=/dev/tty
    fi

    say "Asking GitHub for the newest release"
    local version
    version=$(curl -fsSL "https://api.github.com/repos/$repo/releases/latest" \
        | sed -n 's/.*"tag_name": *"v\([^"]*\)".*/\1/p')
    [[ $version =~ ^[0-9]+(\.[0-9]+)*$ ]] || die "GitHub did not say what the newest release is."

    local installed
    installed=$(pacman -Q wallflux 2>/dev/null | cut -d' ' -f2 | cut -d- -f1 || true)
    if [[ $installed == "$version" ]]; then
        say "WallFlux $version is installed already, and it is the newest."
        exit 0
    fi

    local missing=()
    pacman -Qq base-devel >/dev/null 2>&1 || missing+=(base-devel)
    pacman -Qq git >/dev/null 2>&1 || missing+=(git)
    if ((${#missing[@]})); then
        say "Installing what building needs: ${missing[*]}"
        sudo pacman -S --needed "$@" "${missing[@]}" <"$answers"
    fi

    # Built on the disk: /tmp is often memory, and the build takes gigabytes.
    local work="${XDG_CACHE_HOME:-$HOME/.cache}/wallflux-install"
    rm -rf "$work"
    mkdir -p "$work"
    trap 'rm -rf "$work"' EXIT

    if ! pacman -Qq steamworks-sdk >/dev/null 2>&1; then
        say "Installing Valve's Steamworks SDK from the AUR"
        git clone --depth 1 https://aur.archlinux.org/steamworks-sdk.git "$work/steamworks-sdk"
        # As a dependency, so removing WallFlux with -Rns takes it too.
        (cd "$work/steamworks-sdk" && makepkg -si --asdeps "$@" <"$answers")
    fi

    say "Building WallFlux $version${installed:+ (replacing $installed)} -- this takes a few minutes"
    mkdir "$work/wallflux"
    curl -fsSL -o "$work/wallflux/PKGBUILD" \
        "https://raw.githubusercontent.com/$repo/v$version/packaging/PKGBUILD"
    (cd "$work/wallflux" && makepkg -si "$@" <"$answers")

    say "WallFlux $version is installed.${installed:+ Quit the running one from its tray icon and start it again.}"
}

main "$@"
