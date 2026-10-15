# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 invisigoth

cmake_minimum_required(VERSION 3.21)

project(WallFlux VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# WebEngineQuick pulls in Quick, Qml and Gui on its own, but naming them keeps
# the error at configure time -- with a missing module named -- rather than at
# link time.
find_package(Qt6 6.5 REQUIRED COMPONENTS Core Gui Qml Quick WebEngineQuick)

# Only the tests need this one, and a machine that just wants to run WallFlux
# should not have to have it.
find_package(Qt6 6.5 QUIET COMPONENTS Test)

# Qt speaks no layer-shell of its own, so this is what puts the window in the
# compositor's background layer. KDE ships it; so do the wlroots distributions.
find_package(LayerShellQt REQUIRED)

find_package(ZLIB REQUIRED)

# lz4 ships no CMake config on Arch, only a pkg-config file, so it comes in
# that way. PkgConfig::LZ4 is an imported target like any other.
find_package(PkgConfig REQUIRED)
pkg_check_modules(LZ4 REQUIRED IMPORTED_TARGET liblz4)

qt_standard_project_setup(REQUIRES 6.5)

qt_add_executable(wallflux
    src/main.cpp
)

# Kept in qml/ on disk but placed at the module's root inside the binary.
# Without this the file would land in a subdirectory of the module, which Qt
# warns about and which needs a qmldir of its own.
set_source_files_properties(qml/Main.qml PROPERTIES QT_RESOURCE_ALIAS Main.qml)

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
    SOURCES
        src/wallpapercore.h
        src/wallpapercore.cpp
)

target_include_directories(wallflux PRIVATE src)

target_link_libraries(wallflux PRIVATE
    LayerShellQt::Interface
    Qt6::Core
    Qt6::Gui
    Qt6::Qml
    Qt6::Quick
    Qt6::WebEngineQuick
    ZLIB::ZLIB
    PkgConfig::LZ4
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
    target_link_libraries(tst_wallpapercore PRIVATE Qt6::Qml Qt6::Test)
    add_test(NAME wallpapercore COMMAND tst_wallpapercore)
endif()

install(TARGETS wallflux
    RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}
)
