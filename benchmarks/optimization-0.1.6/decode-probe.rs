//! Offline benchmark of the exact decoder shipped in librespot-playback.
//! No Spotify access, account, output sink or playback-speed claim.
use librespot_playback::decoder::{AudioDecoder, SymphoniaDecoder};
use std::{fs::File, hint::black_box, io::{BufWriter, Write}, time::Instant};
use symphonia::core::probe::Hint;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let path = args.get(1).expect("synthetic fixture path");
    let passes: usize = args.get(2).expect("passes").parse()?;
    let expected: usize = args.get(3).expect("expected stereo sample count").parse()?;
    let mut pcm_output = if args.get(4).map(String::as_str) == Some("--pcm-output") {
        assert_eq!(passes, 1, "PCM verification is a separate single-pass run");
        Some(BufWriter::new(File::create(args.get(5).expect("PCM output path"))?))
    } else {
        None
    };
    let extension = std::path::Path::new(path).extension().unwrap().to_str().unwrap();
    let started = Instant::now();
    let mut sample_count = 0usize;
    let mut packet_count = 0usize;
    let mut checksum = 0f64;
    for _ in 0..passes {
        let mut hint = Hint::new();
        hint.with_extension(extension);
        let mut decoder = SymphoniaDecoder::new(File::open(path)?, hint)?;
        let mut pass_samples = 0usize;
        while let Some((position, packet)) = decoder.next_packet()? {
            assert!(!position.skipped, "fixture unexpectedly skipped a packet");
            let samples = packet.samples()?;
            pass_samples += samples.len();
            packet_count += 1;
            if let Some(first) = samples.first() {
                assert!(first.is_finite());
                checksum += first;
            }
            if let Some(output) = pcm_output.as_mut() {
                for sample in samples {
                    assert!(sample.is_finite(), "complete PCM output must be finite");
                    output.write_all(&sample.to_bits().to_le_bytes())?;
                }
            }
            black_box(samples);
        }
        assert_eq!(pass_samples, expected, "complete fixture decoded");
        sample_count += pass_samples;
    }
    assert!(checksum.is_finite() && checksum != 0.0);
    if let Some(output) = pcm_output.as_mut() {
        output.flush()?;
    }
    println!("{{\"format\":\"{extension}\",\"passes\":{passes},\"samples\":{sample_count},\"packets\":{packet_count},\"wall_seconds\":{},\"checksum\":{checksum}}}", started.elapsed().as_secs_f64());
    Ok(())
}
