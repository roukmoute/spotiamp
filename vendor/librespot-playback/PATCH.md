# Local patch to librespot-playback 0.8.0

Only the player runtime construction in `src/player.rs` differs from the
published crate. `Runtime::new()` is replaced with an explicitly configured
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
to Spotify. Authenticated Spotify playback, Windows playback and sustained
playback latency have not been validated. Retest these before claiming a
playback-memory or performance improvement.

Upstream commit recorded by the published crate:
`d36f9f1907e8cc9d68a93f8ebc6b627b1bf7267d` (subdirectory `playback`).
Source: https://github.com/librespot-org/librespot/tree/d36f9f1907e8cc9d68a93f8ebc6b627b1bf7267d/playback
The MIT license has been copied from the root of that same upstream commit;
the published subcrate did not contain its own license file.
