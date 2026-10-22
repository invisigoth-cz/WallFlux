// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

/// Who made a workshop item, asked for without an API key.
///
/// Two questions, both of which Valve answers to anybody:
///
///  - `ISteamRemoteStorage/GetPublishedFileDetails/v1/` is a POST that takes
///    published file ids and needs no key. Its `creator` is the maker's
///    steamID64, which is the only way to put an author on a wallpaper that
///    is already installed -- project.json carries a title, a type, tags
///    and housekeeping, and says nothing about who wrote it. Checked across
///    all of them. The same answer carries the workshop's own `tags`,
///    which is why this returns both: project.json has only the genres the
///    author ticked, and Steam adds Wallpaper, the type, the resolution,
///    the age rating and whether it answers to sound. Measured on one
///    wallpaper: `["Sci-Fi"]` on the disk against six in the workshop.
///  - the profile page asked for as XML gives the persona name and the
///    little avatar. `ISteamUser/GetPlayerSummaries`, which says the same
///    thing, wants a key.
///
/// Shared, so the library and the workshop ask once between them: a grid of
/// fifty cards is often a dozen authors, and the same author again on the
/// next card is free.
.pragma library

/// workshop id -> `{ maker, tags }`, the maker being "" where Steam named
/// nobody.
var items = ({});

/// steamID64 -> { name, avatar }.
var people = ({});

/// Who is already being asked about, so two cards in a row by the same
/// person are one question rather than two.
var asking = ({});

/// What the workshop says about one item: `done({ maker, tags })`, once.
/// Never called at all for an id Steam had nothing to say about.
function detailsOf(id, done) {
    if (!id)
        return;
    if (items[id] !== undefined) {
        done(items[id]);
        return;
    }
    if (asking["d" + id])
        return;
    asking["d" + id] = true;

    var call = new XMLHttpRequest();
    call.open("POST", "https://api.steampowered.com/ISteamRemoteStorage/"
                      + "GetPublishedFileDetails/v1/");
    call.setRequestHeader("Content-Type", "application/x-www-form-urlencoded");
    call.onreadystatechange = function () {
        if (call.readyState !== XMLHttpRequest.DONE)
            return;
        delete asking["d" + id];
        if (call.status !== 200) {
            console.warn("the workshop was asked about", id, "and said", call.status);
            return;
        }

        var said;
        try {
            said = JSON.parse(call.responseText).response.publishedfiledetails[0];
        } catch (e) {
            console.warn("what the workshop said about", id, "was not readable:", e);
            return;
        }
        if (!said)
            return;

        var tags = [];
        if (said.tags) {
            for (var i = 0; i < said.tags.length; ++i)
                tags.push(said.tags[i].tag);
        }

        items[id] = { maker: said.creator ? String(said.creator) : "", tags: tags };
        done(items[id]);
    };
    call.send("itemcount=1&publishedfileids[0]=" + encodeURIComponent(id));
}

/// The same question about a whole library at once. `done()` once every
/// id asked about has an answer in `items`, so `detailsOf` is free for
/// each of them afterwards.
///
/// The endpoint takes as many ids as are given it in one POST -- measured
/// with all 56 on this machine, answered in one go -- but it is asked in
/// hundreds so that a very large library is several requests rather than
/// one enormous one.
function detailsOfMany(ids, done) {
    var wanted = [];
    for (var i = 0; i < ids.length; ++i) {
        if (ids[i] && items[ids[i]] === undefined)
            wanted.push(ids[i]);
    }
    if (wanted.length === 0) {
        done();
        return;
    }

    var chunks = [];
    for (var at = 0; at < wanted.length; at += 100)
        chunks.push(wanted.slice(at, at + 100));

    var left = chunks.length;
    for (var c = 0; c < chunks.length; ++c)
        askAbout(chunks[c], function () {
            if (--left === 0)
                done();
        });
}

/// One POST about one handful of ids.
function askAbout(ids, done) {
    var body = "itemcount=" + ids.length;
    for (var i = 0; i < ids.length; ++i)
        body += "&publishedfileids[" + i + "]=" + encodeURIComponent(ids[i]);

    var call = new XMLHttpRequest();
    call.open("POST", "https://api.steampowered.com/ISteamRemoteStorage/"
                      + "GetPublishedFileDetails/v1/");
    call.setRequestHeader("Content-Type", "application/x-www-form-urlencoded");
    call.onreadystatechange = function () {
        if (call.readyState !== XMLHttpRequest.DONE)
            return;
        if (call.status !== 200) {
            console.warn("the workshop was asked about", ids.length, "wallpapers and said",
                         call.status);
            done();
            return;
        }

        var said;
        try {
            said = JSON.parse(call.responseText).response.publishedfiledetails;
        } catch (e) {
            console.warn("what the workshop said about those wallpapers was not readable:", e);
            done();
            return;
        }

        for (var i = 0; said && i < said.length; ++i) {
            var one = said[i];
            if (!one || !one.publishedfileid)
                continue;
            var tags = [];
            if (one.tags) {
                for (var t = 0; t < one.tags.length; ++t)
                    tags.push(one.tags[t].tag);
            }
            items[String(one.publishedfileid)] =
                { maker: one.creator ? String(one.creator) : "", tags: tags };
        }
        done();
    };
    call.send(body);
}

/// The name and the face behind a steamID64. `done({ name, avatar })`.
function profileOf(who, done) {
    if (!who || who === "0")
        return;
    if (people[who] !== undefined) {
        done(people[who]);
        return;
    }
    if (asking["p" + who])
        return;
    asking["p" + who] = true;

    var call = new XMLHttpRequest();
    call.open("GET", "https://steamcommunity.com/profiles/" + who + "?xml=1");
    call.onreadystatechange = function () {
        if (call.readyState !== XMLHttpRequest.DONE)
            return;
        delete asking["p" + who];
        if (call.status !== 200) {
            console.warn("the profile of", who, "came back", call.status);
            return;
        }

        var name = /<steamID><!\[CDATA\[([\s\S]*?)\]\]><\/steamID>/.exec(call.responseText);
        var face = /<avatarIcon><!\[CDATA\[([\s\S]*?)\]\]><\/avatarIcon>/.exec(call.responseText);
        if (!name)
            return;

        people[who] = { name: name[1], avatar: face ? face[1] : "" };
        done(people[who]);
    };
    call.send();
}
