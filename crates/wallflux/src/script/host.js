// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

// What a scene's scripts find around them: the globals Wallpaper Engine
// gives a script, written out from what the scripts in the library use, and
// the few functions `script.rs` drives them through. Run once, as a plain
// script, before any of the scene's own modules.
//
// Everything that crosses into Rust is a number, a string or JSON, so the
// vectors and layers below never have to be understood on the other side.

'use strict';

class VecN {
    add(o) { return this.zip(o, (a, b) => a + b); }
    subtract(o) { return this.zip(o, (a, b) => a - b); }
    multiply(o) { return this.zip(o, (a, b) => a * b); }
    divide(o) { return this.zip(o, (a, b) => a / b); }
    dot(o) { return this.keys().reduce((sum, k) => sum + this[k] * o[k], 0); }
    lengthSqr() { return this.dot(this); }
    length() { return Math.sqrt(this.lengthSqr()); }
    normalize() {
        const length = this.length();
        return length > 0 ? this.divide(length) : this.copy();
    }
    copy() { return new this.constructor(this); }
    equals(o) { return this.keys().every(k => this[k] === o[k]); }
    mix(o, t) { return this.zip(o, (a, b) => a + (b - a) * t); }
    toString() { return this.keys().map(k => this[k]).join(' '); }
    // A number stands for the same number in every component, which is how
    // `value.multiply(2)` reads.
    zip(o, op) {
        const out = new this.constructor(this);
        for (const k of this.keys())
            out[k] = op(this[k], typeof o === 'number' ? o : (o[k] ?? 0));
        return out;
    }
    // Any of: nothing, one number for every component, the components, another
    // vector of any length, or the "x y z" a scene writes its vectors as.
    fill(args) {
        const keys = this.keys();
        let from = args;
        if (args.length === 1 && typeof args[0] === 'string')
            from = args[0].trim().split(/\s+/).map(Number);
        else if (args.length === 1 && Array.isArray(args[0]))
            from = args[0];
        else if (args.length === 1 && typeof args[0] === 'object' && args[0] !== null)
            from = keys.map(k => args[0][k] ?? 0);
        else if (args.length === 1)
            from = keys.map(() => args[0]);
        keys.forEach((k, i) => { this[k] = Number(from[i] ?? 0); });
    }
}

class Vec2 extends VecN {
    constructor(...args) { super(); this.fill(args); }
    keys() { return ['x', 'y']; }
}

class Vec3 extends VecN {
    constructor(...args) { super(); this.fill(args); }
    keys() { return ['x', 'y', 'z']; }
    cross(o) {
        return new Vec3(this.y * o.z - this.z * o.y, this.z * o.x - this.x * o.z,
                        this.x * o.y - this.y * o.x);
    }
}

class Vec4 extends VecN {
    constructor(...args) { super(); this.fill(args); }
    keys() { return ['x', 'y', 'z', 'w']; }
}

globalThis.Vec2 = Vec2;
globalThis.Vec3 = Vec3;
globalThis.Vec4 = Vec4;

// The sound, as the arrays a script asked for. Sixteen bands are what the
// renderer measures; a script asking for 32 or 64 gets each band spread over
// as many entries as it covers.
const audio = [];

globalThis.engine = {
    frametime: 0,
    runtime: 0,
    AUDIO_RESOLUTION_16: 16,
    AUDIO_RESOLUTION_32: 32,
    AUDIO_RESOLUTION_64: 64,
    registerAudioBuffers(resolution) {
        const n = [16, 32, 64].includes(resolution) ? resolution : 16;
        const buffer = { left: new Float32Array(n), right: new Float32Array(n),
                         average: new Float32Array(n) };
        audio.push(buffer);
        return buffer;
    },
    isDesktopDevice() { return true; },
    isMobileDevice() { return false; },
    isWallpaper() { return true; },
    isScreensaver() { return false; },
    isRunningInEditor() { return false; },
};

globalThis.console = {
    log() {}, info() {}, warn() {}, error() {}, debug() {},
};

// The settings a script declares for itself. Each `add...` names one and
// gives its default; what the viewer chose is laid over it afterwards.
globalThis.createScriptProperties = function () {
    const properties = {};
    const add = spec => {
        if (spec && typeof spec.name === 'string')
            properties[spec.name] = spec.value;
        return builder;
    };
    const builder = new Proxy(properties, {
        get(target, key) {
            if (key === 'finish')
                return () => properties;
            if (typeof key === 'string' && key.startsWith('add'))
                return add;
            return target[key];
        },
    });
    return builder;
};

// Every object in the scene, by its place in scene.json -- including the
// ones that are never drawn, which some scenes keep only to hold a number
// that other scripts read.
const layers = [];
const byName = new Map();

globalThis.thisScene = {
    getLayer(name) { return byName.get(String(name)) ?? null; },
};

const scripts = [];

function plain(value) {
    return value instanceof VecN ? value.toString() : value;
}

function lively(value) {
    // A vector the scene wrote as "x y z" is handed to a script as a vector.
    if (typeof value === 'string' && /^\s*-?[\d.e+-]+(\s+-?[\d.e+-]+){1,3}\s*$/.test(value)) {
        const parts = value.trim().split(/\s+/);
        return new [Vec2, Vec3, Vec4][parts.length - 2](value);
    }
    return value;
}

// A scene stores its angles in radians and hands them to a script in
// degrees -- the clock in Dark Core turns its hands with `* WEMath.rad2deg`.
const DEGREES = 180 / Math.PI;

// The scene's objects, as JSON: `[{name, origin, angles, scale, size, alpha,
// visible}]` with the vectors as arrays, and `text` on those that have one.
globalThis.__layers = function (json) {
    for (const one of JSON.parse(json)) {
        const layer = {
            name: one.name,
            origin: new Vec3(one.origin),
            angles: new Vec3(one.angles).multiply(DEGREES),
            scale: new Vec3(one.scale),
            size: new Vec2(one.size),
            alpha: one.alpha,
            visible: one.visible,
        };
        if (one.text !== undefined)
            layer.text = one.text;
        layers.push(layer);
        if (!byName.has(one.name))
            byName.set(one.name, layer);
    }
};

// One scripted value: the module that drives it, the layer it belongs to,
// and either the layer's own property it drives or the value it starts at.
// Answers with what went wrong, or nothing.
globalThis.__script = function (module, layer, property, start, chosen, user) {
    const entry = { module, layer: layers[layer], property, value: lively(JSON.parse(start)) };
    scripts.push(entry);
    try {
        const own = module.scriptProperties;
        const told = JSON.parse(chosen);
        if (own && told && typeof told === 'object')
            for (const [name, setting] of Object.entries(told))
                own[name] = setting && typeof setting === 'object' && 'value' in setting
                    ? setting.value : setting;
        globalThis.thisLayer = entry.layer;
        if (typeof module.init === 'function') {
            const now = module.init(current(entry));
            if (now !== undefined)
                keep(entry, now);
        }
        if (typeof module.applyUserProperties === 'function')
            module.applyUserProperties(JSON.parse(user));
        return '';
    } catch (e) {
        entry.broken = true;
        return String(e);
    }
};

function current(entry) {
    return entry.property ? entry.layer[entry.property] : entry.value;
}

function keep(entry, value) {
    if (entry.property)
        entry.layer[entry.property] = value;
    else
        entry.value = value;
}

globalThis.__frame = function (frametime, runtime, left, right) {
    engine.frametime = frametime;
    engine.runtime = runtime;
    for (const buffer of audio) {
        const n = buffer.left.length;
        // Each the loudest of the 64 entries it covers, as `Spectrum::fewer`.
        const per = left.length / n;
        for (let i = 0; i < n; i++) {
            let l = 0, r = 0;
            for (let j = i * per; j < (i + 1) * per; j++) {
                l = Math.max(l, left[j]);
                r = Math.max(r, right[j]);
            }
            buffer.left[i] = l;
            buffer.right[i] = r;
            buffer.average[i] = (l + r) / 2;
        }
    }
};

// Runs one script's update. Answers with what went wrong, or nothing.
globalThis.__update = function (index) {
    const entry = scripts[index];
    if (entry.broken || typeof entry.module.update !== 'function')
        return '';
    try {
        globalThis.thisLayer = entry.layer;
        const next = entry.module.update(current(entry));
        if (next !== undefined)
            keep(entry, next);
        return '';
    } catch (e) {
        entry.broken = true;
        return String(e);
    }
};

globalThis.__break = function (index) {
    scripts[index].broken = true;
};

globalThis.__listens = function () {
    return audio.length > 0;
};

// What the frame left behind, as JSON: each layer as `[origin, angles,
// scale, alpha, visible]`, each value a script drives on its own, and each
// layer's text. A script may answer with a number -- `date.getDate()` -- and
// a text is words whatever it was handed.
globalThis.__state = function () {
    const vector = v => [Number(v?.x ?? 0), Number(v?.y ?? 0), Number(v?.z ?? 0)];
    return JSON.stringify({
        layers: layers.map(l => [vector(l.origin), vector(l.angles).map(a => a / DEGREES), vector(l.scale),
                                 Number(l.alpha), Boolean(l.visible)]),
        values: scripts.map(s => (s.property ? null : plain(s.value))),
        texts: layers.map(l => (l.text === undefined ? null : String(l.text))),
    });
};
