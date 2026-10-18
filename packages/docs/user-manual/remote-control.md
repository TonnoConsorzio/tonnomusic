---
description: Control Nuclear from your phone or any other device on your network.
---

# Remote control

Nuclear Jam is temporarily restricted to browsers on **the same computer**. Phone access and LAN QR sharing are disabled until authenticated, encrypted remote control is available.

## Enable Jam locally

1. Open Settings → Integrations in Nuclear.
2. Enable **Nuclear Jam**.
3. Open **Remote URL** in a browser on the same computer. The server uses `127.0.0.1` and a port from 4120 to 4129.
4. **API URL** is available for local scripts. See [HTTP API](../integrations/http-api.md).

This release does not display the LAN address or QR icon. In development, the local browser UI may be served on `http://localhost:5173`. Other devices on the same Wi-Fi cannot connect.

## What the remote does

The main screen of the remote has four sections: a search bar in the header, now playing, controls, and the queue. Lyrics have a page of their own.

<figure><img src="../.gitbook/assets/jam-remote.png" alt="Nuclear Jam remote control UI on a phone showing now playing, controls, and queue" width="300"><figcaption><p>Nuclear Jam on a phone</p></figcaption></figure>

- **Now playing** at the top, with cover art, track title, and artist. The microphone button next to the title opens the lyrics page.
- **Controls** in the middle: previous, play/pause, next, plus a seek bar, shuffle, repeat, and discovery toggle.
- **Up next** at the bottom shows the queue, with the currently playing track highlighted. The number next to the heading is the number of tracks in the queue. Each item has an X button to remove it from the queue.

A badge in the header shows the connection status: **Connecting**, **Connected**, **Reconnecting**, or **Disconnected**.

Anything you do on the remote happens in Nuclear immediately, and vice versa: skipping a track on the desktop updates every connected remote. The local browser and desktop player stay in sync.

### Searching for music

Use the search bar at the top to enter your query. A drawer will slide down with matching tracks.

<figure><img src="../.gitbook/assets/jam-search.png" alt="Search drawer showing results with an add button on each track" width="300"><figcaption><p>Searching for tracks from the remote</p></figcaption></figure>

Click (or tap) a track to add it to the queue. The drawer will close after adding. If the queue was empty, playback starts automatically.

### Lyrics

To open the lyrics page, tap the microphone button next to the title of the current track. The lyrics fill the screen. To go back to the main screen, tap the back arrow at the top of the page, or use the back button of your phone.

<figure><img src="../.gitbook/assets/jam-lyrics.png" alt="The lyrics page on a phone, with word-synced lyrics and the current line highlighted" width="300"><figcaption><p>Lyrics on the remote</p></figcaption></figure>

The remote gets the lyrics from the lyrics plugins in Nuclear, so you must install at least one lyrics plugin in Nuclear. The lyrics page works like the Lyrics view in Nuclear. It shows the same types of lyrics, and you can tap a line to jump to it, switch to a different source, and fix the timing of synced lyrics. For more information, see [Lyrics](../core-concepts/lyrics.md).

The lyrics page is different from the Lyrics view in these ways:

- The source and the timing that you choose apply only to the remote that you use. They don't change the lyrics in Nuclear or on other remotes.
- The page always scrolls to the current line. It has no auto-scroll switch.
- The page uses the text size that you set in the Lyrics view of Nuclear. It has no text size buttons.
- If no lyrics plugin is installed in Nuclear, the page shows **No lyrics for this track**.
- If the remote can't get the lyrics from Nuclear, the page shows **Could not load lyrics**. Check the connection to Nuclear and try again.

### Removing tracks from the queue

Each queue item shows an X button on the right side. Tap it to remove that track from the queue.

<figure><img src="../.gitbook/assets/jam-queue-remove.png" alt="Queue showing the X button on each item" width="300"><figcaption><p>Removing a track from the queue</p></figcaption></figure>
