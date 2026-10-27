// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth

// The scene renderer's C interface, written by hand to match
// crates/wallflux/src/ffi.rs -- which says what each call promises. Every
// call is made on the render thread with the OpenGL context current.

#pragma once

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct WallfluxScene WallfluxScene;

typedef const void *(*WallfluxLoader)(const char *name, void *user);

WallfluxScene *wallflux_scene_load(const char *folder, WallfluxLoader loader, void *user,
                                   char *error, size_t error_len);
bool wallflux_scene_draw(WallfluxScene *scene, uint32_t width, uint32_t height, float seconds);
void wallflux_scene_point_at(WallfluxScene *scene, bool inside, float x, float y,
                             uint32_t width, uint32_t height);
void wallflux_scene_free(WallfluxScene *scene);
size_t wallflux_scene_sounds(const char *folder, const char *cache, char *out, size_t out_len);

#ifdef __cplusplus
}
#endif
