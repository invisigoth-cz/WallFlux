# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 invisigoth

cmake_minimum_required(VERSION 3.21)

project(WallFlux VERSION 0.14.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# WebEngineQuick pulls in Quick, Qml and Gui on its own, but naming them keeps
# the error at configure time -- with a missing module named -- rather than at
# link time.
# DBus is how the compositor is asked what is covering the wallpaper: KWin
# speaks no other language and Wayland tells a window nothing about the ones
# in front of it.
find_package(Qt6 6.5 REQUIRED COMPONENTS
    Core DBus Gui LinguistTools Multimedia Network OpenGL Qml Quick WebEngineQuick Widgets)

# Only the tests need this one, and a machine that just wants to run WallFlux
# should not have to have it.
find_package(Qt6 6.5 QUIET COMPONENTS Test)

# Qt speaks no layer-shell of its own, so this is what puts the window in the
# compositor's background layer. KDE ships it; so do the wlroots distributions.
find_package(LayerShellQt REQUIRED)

qt_standard_project_setup(REQUIRES 6.5)

qt_add_executable(wallflux
    src/main.cpp
)

# Kept in qml/ on disk but placed at the module's root inside the binary.
# Without this the file would land in a subdirectory of the module, which Qt
# warns about and which needs a qmldir of its own.
set_source_files_properties(qml/Main.qml PROPERTIES QT_RESOURCE_ALIAS Main.qml)
set_source_files_properties(qml/Picker.qml PROPERTIES QT_RESOURCE_ALIAS Picker.qml)
set_source_files_properties(qml/WallpaperCard.qml PROPERTIES QT_RESOURCE_ALIAS WallpaperCard.qml)
set_source_files_properties(qml/FilterPanel.qml PROPERTIES QT_RESOURCE_ALIAS FilterPanel.qml)
set_source_files_properties(qml/PreviewTile.qml PROPERTIES QT_RESOURCE_ALIAS PreviewTile.qml)
set_source_files_properties(qml/WorkshopView.qml PROPERTIES QT_RESOURCE_ALIAS WorkshopView.qml)
set_source_files_properties(qml/WorkshopFilters.qml PROPERTIES QT_RESOURCE_ALIAS WorkshopFilters.qml)
set_source_files_properties(qml/Chip.qml PROPERTIES QT_RESOURCE_ALIAS Chip.qml)
set_source_files_properties(qml/ChipSection.qml PROPERTIES QT_RESOURCE_ALIAS ChipSection.qml)
set_source_files_properties(qml/Scroller.qml PROPERTIES QT_RESOURCE_ALIAS Scroller.qml)
set_source_files_properties(qml/RoundedButton.qml PROPERTIES QT_RESOURCE_ALIAS RoundedButton.qml)
set_source_files_properties(qml/RoundedTool.qml PROPERTIES QT_RESOURCE_ALIAS RoundedTool.qml)
set_source_files_properties(qml/Details.qml PROPERTIES QT_RESOURCE_ALIAS Details.qml)
set_source_files_properties(qml/PlaylistBar.qml PROPERTIES QT_RESOURCE_ALIAS PlaylistBar.qml)
set_source_files_properties(qml/WallpaperSurface.qml PROPERTIES QT_RESOURCE_ALIAS WallpaperSurface.qml)
set_source_files_properties(qml/SceneSound.qml PROPERTIES QT_RESOURCE_ALIAS SceneSound.qml)
set_source_files_properties(qml/DisplaysView.qml PROPERTIES QT_RESOURCE_ALIAS DisplaysView.qml)
set_source_files_properties(qml/steamprofile.js PROPERTIES QT_RESOURCE_ALIAS steamprofile.js)

# The QML lives in the binary rather than beside it, so the program runs from
# wherever it is put without losing its interface.
# The backend belongs to the QML module rather than merely being compiled
# alongside it: that is what makes qmltyperegistrar see QML_NAMED_ELEMENT and
# QML_VALUE_TYPE and write the types into qmldir, where qmllint and the QML
# compiler can find them.
qt_add_qml_module(wallflux
    URI WallFlux
    VERSION 1.0
    QML_FILES
        qml/Main.qml
        qml/Picker.qml
        qml/WallpaperCard.qml
        qml/FilterPanel.qml
        qml/PreviewTile.qml
        qml/WorkshopView.qml
        qml/WorkshopFilters.qml
        qml/Chip.qml
        qml/ChipSection.qml
        qml/Scroller.qml
        qml/RoundedButton.qml
        qml/RoundedTool.qml
        qml/Details.qml
        qml/PlaylistBar.qml
        qml/WallpaperSurface.qml
        qml/SceneSound.qml
        qml/DisplaysView.qml
        qml/steamprofile.js
    SOURCES
        src/otheraudio.h
        src/otheraudio.cpp
        src/playlists.h
        src/playlists.cpp
        src/language.h
        src/language.cpp
        src/screencover.h
        src/screencover.cpp
        src/kwinscript.h
        src/kwinscript.cpp
        src/pointer.h
        src/pointer.cpp
        src/desktop.h
        src/desktop.cpp
        src/sceneview.h
        src/sceneview.cpp
        src/steamworks.h
        src/steamworks.cpp
        src/tags.h
        src/tags.cpp
        src/wallpapercore.h
        src/wallpapercore.cpp
)

# The Czech catalogue, compiled into the binary under /i18n -- which is where
# Language looks for it. `cmake --build build --target update_translations`
# refreshes the .ts file after strings change.
qt_add_translations(wallflux TS_FILES i18n/wallflux_cs.ts)

target_include_directories(wallflux PRIVATE src)

# Scene wallpapers are drawn by the renderer under crates/, which cargo builds
# into a static library inside this build tree: one file linked into the
# binary, nothing installed beside it. Always in release -- a debug renderer
# draws a heavy scene at a few frames a second, which is no way to judge one.
# WALLFLUX_CARGO_FLAGS is where a package build says --frozen, and a machine
# without the network says --offline.
find_program(CARGO cargo REQUIRED)
set(WALLFLUX_CARGO_FLAGS "--locked" CACHE STRING "Extra flags for cargo build")
separate_arguments(wallflux_cargo_flags UNIX_COMMAND "${WALLFLUX_CARGO_FLAGS}")
# The .js is the scripting host, which the renderer takes in with include_str!.
file(GLOB_RECURSE wallflux_rust_sources CONFIGURE_DEPENDS
    ${CMAKE_SOURCE_DIR}/crates/*.rs ${CMAKE_SOURCE_DIR}/crates/*.js
    ${CMAKE_SOURCE_DIR}/crates/*/Cargo.toml)
set(wallflux_scene_lib ${CMAKE_BINARY_DIR}/cargo/release/libwallflux.a)
add_custom_command(
    OUTPUT ${wallflux_scene_lib}
    COMMAND ${CARGO} build --release --lib -p wallflux
            --target-dir ${CMAKE_BINARY_DIR}/cargo ${wallflux_cargo_flags}
    WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}
    DEPENDS ${wallflux_rust_sources} ${CMAKE_SOURCE_DIR}/Cargo.toml ${CMAKE_SOURCE_DIR}/Cargo.lock
    COMMENT "Building the scene renderer with cargo"
    USES_TERMINAL
)
add_custom_target(wallflux_scene_build DEPENDS ${wallflux_scene_lib})
add_library(wallflux_scene STATIC IMPORTED)
# What the renderer wants from the system -- PulseAudio, to hear what the
# speakers are playing, and what Rust's standard library needs -- goes with
# it to whatever links it. `cargo rustc -- --print native-static-libs` is
# where this list is from.
set_target_properties(wallflux_scene PROPERTIES
    IMPORTED_LOCATION ${wallflux_scene_lib}
    INTERFACE_LINK_LIBRARIES "pulse-simple;pulse;gcc_s;util;rt;pthread;m;dl")
add_dependencies(wallflux_scene wallflux_scene_build)
target_include_directories(wallflux PRIVATE crates/wallflux/include)

# The number the footer's About icon shows, taken off project() above rather
# than written out a third time: it already has to agree with the PKGBUILD's
# pkgver, and two places is one too many as it is.
target_compile_definitions(wallflux PRIVATE WALLFLUX_VERSION="${PROJECT_VERSION}")

target_link_libraries(wallflux PRIVATE
    LayerShellQt::Interface
    Qt6::Core
    Qt6::DBus
    Qt6::Gui
    Qt6::Multimedia
    # The previews the workshop shows come off Steam's own servers, and
    # keeping them is what makes a page already looked at cost nothing.
    Qt6::Network
    # The framebuffer a scene wallpaper is drawn into.
    Qt6::OpenGL
    Qt6::Qml
    Qt6::Quick
    Qt6::WebEngineQuick
    # Only for the tray icon's fallback, which Qt.labs.platform needs linked
    # whether or not this desktop ends up using it.
    Qt6::Widgets
    # The scene renderer.
    wallflux_scene
)

if(Qt6Test_FOUND)
    enable_testing()
    # Compiles the one file it is about rather than linking the application,
    # so the test needs neither main() nor the web engine.
    qt_add_executable(tst_wallpapercore
        tests/tst_wallpapercore.cpp
        src/wallpapercore.cpp
    )
    target_include_directories(tst_wallpapercore PRIVATE src)
    target_include_directories(tst_wallpapercore PRIVATE crates/wallflux/include)
    target_link_libraries(tst_wallpapercore PRIVATE Qt6::Qml Qt6::Test wallflux_scene)
    add_test(NAME wallpapercore COMMAND tst_wallpapercore)

    qt_add_executable(tst_playlists
        tests/tst_playlists.cpp
        src/playlists.cpp
    )
    target_include_directories(tst_playlists PRIVATE src)
    target_link_libraries(tst_playlists PRIVATE Qt6::Qml Qt6::Test)
    add_test(NAME playlists COMMAND tst_playlists)

    qt_add_executable(tst_tags
        tests/tst_tags.cpp
        src/tags.cpp
        src/wallpapercore.cpp
    )
    target_include_directories(tst_tags PRIVATE src)
    target_include_directories(tst_tags PRIVATE crates/wallflux/include)
    target_link_libraries(tst_tags PRIVATE Qt6::Qml Qt6::Test wallflux_scene)
    add_test(NAME tags COMMAND tst_tags)

    qt_add_executable(tst_otheraudio
        tests/tst_otheraudio.cpp
        src/otheraudio.cpp
    )
    target_include_directories(tst_otheraudio PRIVATE src)
    target_link_libraries(tst_otheraudio PRIVATE Qt6::Qml Qt6::Test)
    add_test(NAME otheraudio COMMAND tst_otheraudio)

    qt_add_executable(tst_sceneview
        tests/tst_sceneview.cpp
        src/sceneview.cpp
        src/wallpapercore.cpp
    )
    target_include_directories(tst_sceneview PRIVATE src crates/wallflux/include)
    target_link_libraries(tst_sceneview PRIVATE
        Qt6::OpenGL Qt6::Qml Qt6::Quick Qt6::Test wallflux_scene)
    add_test(NAME sceneview COMMAND tst_sceneview)

    qt_add_executable(tst_screencover
        tests/tst_screencover.cpp
        src/screencover.cpp
        src/kwinscript.cpp
    )
    target_include_directories(tst_screencover PRIVATE src)
    target_link_libraries(tst_screencover PRIVATE Qt6::DBus Qt6::Qml Qt6::Test)
    add_test(NAME screencover COMMAND tst_screencover)
    # Pointed at a bus that is not there, so building ScreenCover in a test
    # cannot load a script into the compositor of whoever runs the tests.
    set_tests_properties(screencover PROPERTIES
        ENVIRONMENT "DBUS_SESSION_BUS_ADDRESS=unix:path=/nonexistent/wallflux-no-bus")

    qt_add_executable(tst_pointer
        tests/tst_pointer.cpp
        src/pointer.cpp
        src/kwinscript.cpp
    )
    target_include_directories(tst_pointer PRIVATE src)
    target_link_libraries(tst_pointer PRIVATE Qt6::DBus Qt6::Qml Qt6::Test)
    add_test(NAME pointer COMMAND tst_pointer)
    set_tests_properties(pointer PROPERTIES
        ENVIRONMENT "DBUS_SESSION_BUS_ADDRESS=unix:path=/nonexistent/wallflux-no-bus")
endif()

include(GNUInstallDirs)

install(TARGETS wallflux
    RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}
)

# Without this on the system, the desktop portal refuses to register the
# application and says so on every start.
install(FILES wallflux.desktop
    DESTINATION ${CMAKE_INSTALL_DATADIR}/applications
)

# The same file again, where the session looks for what to start at login: a
# wallpaper that waits to be started by hand is not doing its job. One file
# rather than two because nothing about the way it is launched differs.
#
# GNUInstallDirs keeps sysconfdir under the prefix, so a /usr install would
# file this under /usr/etc, which no session reads and which gives no sign
# that anything is wrong -- the wallpaper simply never starts. Anyone who
# wants it elsewhere can still say so with -DCMAKE_INSTALL_SYSCONFDIR.
if(CMAKE_INSTALL_PREFIX STREQUAL "/usr" AND NOT IS_ABSOLUTE "${CMAKE_INSTALL_SYSCONFDIR}")
    set(CMAKE_INSTALL_SYSCONFDIR "/etc")
endif()

install(FILES wallflux.desktop
    DESTINATION ${CMAKE_INSTALL_SYSCONFDIR}/xdg/autostart
)
