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
/* The same, with the wallpaper's properties set to `values`: a JSON object of
 * name to value laid over project.json's, or NULL for what it says. */
WallfluxScene *wallflux_scene_load_with(const char *folder, const char *values,
                                        WallfluxLoader loader, void *user, char *error,
                                        size_t error_len);
/* The scene `previous` loaded again with other `values`, carrying on from
 * where it has got to -- clock, particles, scripts, videos -- rather than
 * starting over. `previous` is still the caller's to free afterwards, and is
 * whole again where NULL comes back. */
WallfluxScene *wallflux_scene_reload_with(WallfluxScene *previous, const char *folder,
                                          const char *values, WallfluxLoader loader,
                                          void *user, char *error, size_t error_len);
bool wallflux_scene_draw(WallfluxScene *scene, uint32_t width, uint32_t height, float seconds);
void wallflux_scene_point_at(WallfluxScene *scene, bool inside, float x, float y,
                             uint32_t width, uint32_t height);
void wallflux_scene_free(WallfluxScene *scene);
/* What of the scene was left out, one line each, into `out`; returns the
 * length the whole list needs. */
size_t wallflux_scene_skipped(const WallfluxScene *scene, char *out, size_t out_len);

/* What every scene may do, as the Settings tab says: draw `fps` pictures a
 * second, follow the music, lean with the mouse, have particles, meet a
 * screen of another shape by `scaling` it (0 fill, 1 fit, 2 stretch), glow
 * where it asks to (`bloom`), and load its pictures at half their size
 * (`half_textures`, which reaches only scenes loaded afterwards). */
void wallflux_options(uint32_t fps, bool music, bool parallax, bool particles, uint32_t scaling,
                      bool bloom, bool half_textures);
size_t wallflux_scene_sounds(const char *folder, const char *cache, char *out, size_t out_len);
bool wallflux_scene_has_sound(const char *folder);
void wallflux_speakers(float *out);

#ifdef __cplusplus
}
#endif
