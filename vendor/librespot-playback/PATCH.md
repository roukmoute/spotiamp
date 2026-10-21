# Local patch to librespot-playback 0.8.0

The local changes are confined to `src/player.rs`. For thread startup,
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
cargo test -p librespot-playback --lib quality_tests
```

The tests do not connect to Spotify or verify which audio format Spotify serves.

Upstream commit recorded by the published crate:
`d36f9f1907e8cc9d68a93f8ebc6b627b1bf7267d` (subdirectory `playback`).
Source: https://github.com/librespot-org/librespot/tree/d36f9f1907e8cc9d68a93f8ebc6b627b1bf7267d/playback
The MIT license has been copied from the root of that same upstream commit;
the published subcrate did not contain its own license file.
