//! Measures allocation/thread startup only. No account, network connection,
//! Spotify track, decoder or real audio device is exercised by this probe.
use librespot_core::{Session, config::SessionConfig};
use librespot_playback::{
    audio_backend::{Sink, SinkResult},
    config::{Bitrate, PlayerConfig},
    convert::Converter,
    decoder::AudioPacket,
    mixer::NoOpVolume,
    player::Player,
};
use std::time::Duration;

struct Discard;
impl Sink for Discard {
    fn write(&mut self, _: AudioPacket, _: &mut Converter) -> SinkResult<()> {
        Ok(())
    }
}

fn main() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        println!("probe: before Session/Player; pid={}", std::process::id());
        tokio::time::sleep(Duration::from_secs(2)).await;
        let session = Session::new(SessionConfig::default(), None);
        let player = Player::new(
            PlayerConfig {
                bitrate: Bitrate::Bitrate320,
                gapless: false,
                ditherer: None,
                position_update_interval: Some(Duration::from_millis(500)),
                ..PlayerConfig::default()
            },
            session.clone(),
            Box::new(NoOpVolume),
            || Box::new(Discard),
        );
        let _events = player.get_player_event_channel();
        println!("probe: Player allocated; offline, discard sink, no music");
        tokio::time::sleep(Duration::from_secs(10)).await;
        println!(
            "probe: shutting down; player_failed={}",
            player.is_invalid()
        );
        session.shutdown();
        drop(player);
    });
}
