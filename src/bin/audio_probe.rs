//! Exercises Spotiamp's exact native output sink with synthetic PCM. This is NOT
//! a Spotify/network/decoder test. On Linux use a private ALSA null config.
#![allow(dead_code)]
#[path = "../credentials.rs"]
mod credentials;
#[path = "../engine.rs"]
mod engine;
#[path = "../locale.rs"]
mod locale;
#[path = "../localized_text.rs"]
mod localized_text;
#[path = "../model.rs"]
mod model;
#[path = "../playback_settings.rs"]
mod playback_settings;
#[path = "../settings.rs"]
mod settings;
#[path = "../spotify_library.rs"]
mod spotify_library;
#[path = "../spotify_playlist_write.rs"]
mod spotify_playlist_write;
#[path = "../spotify_search.rs"]
mod spotify_search;
#[path = "../translations/mod.rs"]
mod translations;

use librespot_playback::{audio_backend::Sink, convert::Converter, decoder::AudioPacket};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

fn main() {
    if let Err(error) = run() {
        eprintln!("audio-probe: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    #[cfg(target_os = "linux")]
    if std::env::var_os("ALSA_CONFIG_PATH").is_none() {
        return Err("set ALSA_CONFIG_PATH to a private null/no-device config; probe refuses to use the real default output".into());
    }
    let expect_failure = args.iter().any(|arg| arg == "--expect-no-device");
    let seconds = args
        .windows(2)
        .find(|pair| pair[0] == "--seconds")
        .and_then(|pair| pair[1].parse::<u64>().ok())
        .unwrap_or(3)
        .clamp(1, 30);
    let errors = Arc::new(Mutex::new(None));
    let mut sink = engine::NativeSink::new(errors.clone());
    println!(
        "audio-probe: baseline pid={} no Spotify/no decoder",
        std::process::id()
    );
    std::thread::sleep(Duration::from_secs(2));
    if expect_failure {
        for attempt in 1..=2 {
            if sink.start().is_ok() {
                return Err("expected absent device but output opened".into());
            }
            if errors.lock().unwrap().is_none() {
                return Err("device failure was not reported to GUI state".into());
            }
            sink.stop()?;
            println!(
                "audio-probe: no-device attempt={attempt} returned error; process alive; stop OK"
            );
        }
        return Ok(());
    }
    let mut converter = Converter::new(None);
    let mut frames = 0u64;
    for phase in 1..=2 {
        sink.start()?;
        println!("audio-probe: phase={phase} output started; PCM f64->f32 stereo 44100Hz");
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(seconds) {
            let samples: Vec<_> = (0..441)
                .flat_map(|i| {
                    let sample = ((frames + i) as f64 * std::f64::consts::TAU * 440.0 / 44_100.0)
                        .sin()
                        * 0.01;
                    [sample, sample]
                })
                .collect();
            frames += 441;
            sink.write(AudioPacket::Samples(samples), &mut converter)?;
            std::thread::sleep(Duration::from_millis(10));
        }
        let stop = Instant::now();
        sink.stop()?;
        let elapsed = stop.elapsed();
        if elapsed > Duration::from_secs(1) {
            return Err("sink stop exceeded its bounded deadline".into());
        }
        println!(
            "audio-probe: phase={phase} output stopped/released; stop_ms={} cumulative_frames={frames}",
            elapsed.as_millis()
        );
        std::thread::sleep(Duration::from_secs(2));
    }
    if let Some(message) = errors.lock().unwrap().take() {
        return Err(message.into());
    }
    println!("audio-probe: PASS two start/write/stop cycles; no Spotify/no decoder");
    Ok(())
}
