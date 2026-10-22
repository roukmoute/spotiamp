# Local patch to librespot-playback 0.8.0

The player changes are in `src/player.rs`. For thread startup,
`Runtime::new()` is replaced with an explicitly configured
multithread Tokio runtime using one asynchronous worker and all drivers enabled.

The dedicated player thread is preserved. Upstream explicitly states that
`PlayerInternal` contains blocking code and must run through `block_on()` on its
own thread. The asynchronous worker must remain separate; switching to a
current-thread runtime could stall asynchronous work while the player blocks.

The purpose is predictable thread count across computers, without setting
`TOKIO_WORKER_THREADS` for the process or its other runtimes. This does not cap
Tokio's separate blocking pool, librespot's per-track loader threads, or threads
created by an audio backend. Three threads is an idle probe observation, not a
guarantee for playback.

Before this patch, the initialization-only probe on the 16-logical-CPU Linux
host used 18 threads and about 4.668 MiB RSS / 2.625 MiB private resident memory.
An A/B run using `TOKIO_WORKER_THREADS=1` used 3 threads and about 4.332 MiB RSS /
2.375 MiB private resident memory. That is a modest approximately 0.34 MiB RSS
reduction at idle, not evidence of a large playback-memory saving. The local
patch makes that worker choice explicit in the player code.

These measurements created a session object and player without authenticating
to Spotify. They do not validate authenticated playback memory, Windows playback
memory or sustained playback latency. Measure these separately before claiming
a playback-memory or performance improvement.

`Player::load_with_bitrate` adds an optional bitrate to the existing load command.
This keeps each track and its requested bitrate together in the command queue,
including rapid consecutive loads. Spotiamp only calls it when loading a track;
changing the preference does not send any command to the current player.

When a load changes the bitrate, the old decoder/loading state and any preload
are discarded before the existing reuse checks. This ensures loading the same
Spotify URI again can request the new quality. The ordinary `load()` API and
seeking retain the existing bitrate. No buffer sizes, format fallback order or
Spotify entitlement checks change. The UI describes the requested quality;
upstream can select another available format if the requested one is absent.

Offline regression tests verify command ordering and bitrate pairing, discarded
old state/preload when quality changes, and preservation on an unchanged bitrate
or seek. Run them from the Spotiamp project root (they are dependency tests and
are not included by a plain root `cargo test`):

```sh
cargo test -p librespot-playback --lib
```

The tests do not connect to Spotify or verify which audio format Spotify serves.

## Abandoned track loads

Upstream creates a dedicated thread for each track loader and returns only its
oneshot result receiver to `PlayerState::Loading`. Replacing that state previously
dropped the receiver while the old thread continued its metadata, CDN, audio-key
and decoder work. Rapid navigation could therefore leave several unwanted loads
running concurrently; the receiver alone did not cancel them.

The existing loader thread now races its load future against the existing Tokio
oneshot sender's `closed()` future. The cancellation future is polled first by
`futures_util::future::select`, so an already-abandoned load does not start work.
Dropping a loading state, replacing a preload or stopping the player cancels
outstanding asynchronous waits at the next yield. No extra thread, timer, channel,
buffer or dependency is introduced. Synchronous stream reads, decoder setup and
seeks remain non-preemptible; this does **not** promise instantaneous cancellation
inside those sections or cancel bytes already sent to Spotify.

The loader-handle inventory lock is held across spawn and insertion. This closes
the pre-existing race in which an immediately completed thread removed its entry
before it had been inserted, leaving a finished handle behind. Player destruction
drops both loading-state and preload receivers before draining/joining the loader
threads, and never joins while holding the inventory lock.

Offline tests execute the production cancellation bridge on real loader threads:
already-closed receivers skip work, a dropped receiver wakes a pending load,
successful and failed loads keep their result semantics, replacement and Stop
discard only abandoned work, and PlayerInternal destruction releases both a load
and preload before joining. A real failed result still emits one `Unavailable`
event with its request ID; cancellation of an obsolete receiver does not emit
one. Tests use controlled futures and a sink that rejects audio, without network
requests or a Spotify account. Authenticated server behavior and the cause of a
particular upstream `Unavailable` error still require playback logs.

Upstream commit recorded by the published crate:
`d36f9f1907e8cc9d68a93f8ebc6b627b1bf7267d` (subdirectory `playback`).
Source: https://github.com/librespot-org/librespot/tree/d36f9f1907e8cc9d68a93f8ebc6b627b1bf7267d/playback
The MIT license has been copied from the root of that same upstream commit;
the published subcrate did not contain its own license file.

## Compiled audio formats

The Symphonia `flac` feature is disabled in both Cargo manifests. The remote
loader's complete fallback lists select Ogg Vorbis and MP3 for the supported
96 / 160 / 320 kb/s settings; those codecs and the Ogg container remain enabled.
Spotiamp rejects local files and does not offer lossless playback. Removing an
unreachable codec avoids embedding its decoder, format registry and constants.
This does not change the remote format preference order or audio buffers.
A future local-file or lossless feature must revisit this deliberate restriction.
