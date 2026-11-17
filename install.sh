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
# It asks once whether to go ahead, and sudo asks for the password; then it
# runs to the end, and starts the new WallFlux if the old one was running.
# `--noconfirm` after it (`| bash -s -- --noconfirm`) asks nothing at all;
# whatever is given there goes to pacman and makepkg alike.
#
# Everything is inside main(), so a download cut off half way runs nothing.

set -euo pipefail

repo=invisigoth-cz/WallFlux
work=
keeper=

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
        | sed -n 's/.*"tag_name": *"v\([^"]*\)".*/\1/p' || true)
    [[ $version =~ ^[0-9]+(\.[0-9]+)*$ ]] || die "GitHub did not say what the newest release is."

    local installed
    installed=$(pacman -Q wallflux 2>/dev/null | cut -d' ' -f2 | cut -d- -f1 || true)
    if [[ $installed == "$version" ]]; then
        say "WallFlux $version is installed already, and it is the newest."
        exit 0
    fi

    # One question, and after it none: pacman and makepkg are told
    # --noconfirm. Given --noconfirm, not even this one.
    local asked=1 option
    for option in "$@"; do
        [[ $option != --noconfirm ]] || asked=0
    done
    if ((asked)); then
        local reply
        printf '\nWallFlux is built on this computer, which takes a few minutes, and sudo asks\nfor your password.\n\n'
        local question="Install WallFlux $version?"
        [[ -z $installed ]] || question="Update WallFlux $installed to $version?"
        read -rp "$question [Y/n] " reply <"$answers"
        [[ ! $reply =~ ^[Nn] ]] || { say "Nothing was changed."; exit 0; }
        set -- --noconfirm "$@"
    fi

    # sudo is kept from forgetting the password for as long as the build
    # runs, which can be longer than it remembers on its own. CachyOS asks
    # again before installing a package from outside its repositories and
    # the AUR, on purpose: that second question is the system's, and stays.
    sudo -v <"$answers"
    ( while sleep 50; do kill -0 "$$" 2>/dev/null || exit 0; sudo -n -v 2>/dev/null || exit 0; done ) &
    keeper=$!

    # Built on the disk: /tmp is often memory, and the build takes gigabytes.
    # Global, not local: the trap runs after main has returned.
    work="${XDG_CACHE_HOME:-$HOME/.cache}/wallflux-install"
    rm -rf "$work"
    mkdir -p "$work"
    trap 'kill "$keeper" 2>/dev/null; rm -rf "$work"' EXIT

    local missing=()
    pacman -Qq base-devel >/dev/null 2>&1 || missing+=(base-devel)
    pacman -Qq git >/dev/null 2>&1 || missing+=(git)
    if ((${#missing[@]})); then
        say "Installing what building needs: ${missing[*]}"
        sudo pacman -S --needed "$@" "${missing[@]}" <"$answers"
    fi

    if ! pacman -Qq steamworks-sdk >/dev/null 2>&1; then
        say "Installing Valve's Steamworks SDK from the AUR"
        # Three tries: the AUR drops connections when it is busy.
        local try
        for try in 1 2 3; do
            git clone -q --depth 1 https://aur.archlinux.org/steamworks-sdk.git "$work/steamworks-sdk" && break
            ((try < 3)) || die "The AUR did not answer. Try again in a while."
            rm -rf "$work/steamworks-sdk"
            sleep 5
        done
        # As a dependency, so removing WallFlux with -Rns takes it too.
        (cd "$work/steamworks-sdk" && makepkg -si --asdeps "$@" <"$answers")
    fi

    say "Building WallFlux $version${installed:+ (replacing $installed)} -- this takes a few minutes"
    mkdir "$work/wallflux"
    curl -fsSL -o "$work/wallflux/PKGBUILD" \
        "https://raw.githubusercontent.com/$repo/v$version/packaging/PKGBUILD"
    (cd "$work/wallflux" && makepkg -si "$@" <"$answers")

    # The one running is the old program until it is started again: quit it
    # and start the new one, out of this terminal's session so that closing
    # the terminal leaves it running. The wallpaper goes back to the
    # desktop's own until one is chosen, as at any start.
    local me
    me=$(id -u)
    if pgrep -xu "$me" wallflux >/dev/null; then
        say "Starting the new WallFlux in place of the old one"
        pkill -xu "$me" wallflux || true
        local wait
        for wait in {1..50}; do
            pgrep -xu "$me" wallflux >/dev/null || break
            sleep 0.1
        done
        pkill -KILL -xu "$me" wallflux 2>/dev/null || true
        setsid -f wallflux >/dev/null 2>&1 </dev/null
        say "WallFlux $version is installed and running."
    else
        say "WallFlux $version is installed. It is in the application menu."
    fi
}

main "$@"
