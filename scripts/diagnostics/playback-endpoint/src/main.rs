//! Read-only Core Audio observation and an isolated reproduction using the real
//! playback state machine. The fake endpoint test never changes OS audio.
#[path = "../../../../src-tauri/src/playback.rs"]
mod playback;

fn main() -> Result<(), String> {
    use playback::OutputVolume;
    use std::time::{Duration, Instant};
    let seconds = std::env::args().nth(1).unwrap_or_else(|| "10".into())
        .parse::<u64>().map_err(|_| "Expected observation seconds (1–60)".to_string())?;
    if !(1..=60).contains(&seconds) { return Err("Observation seconds must be 1–60".into()); }
    let start = Instant::now();
    let mut previous = None;
    let mut samples = 0;
    while start.elapsed() < Duration::from_secs(seconds) {
        let current = playback::PlatformOutput.get()?;
        if previous != Some(current) {
            println!("elapsed_ms={} muted={} volume_percent={:.2}", start.elapsed().as_millis(), current.muted, current.volume * 100.0);
            previous = Some(current);
        }
        samples += 1;
        std::thread::sleep(Duration::from_millis(200));
    }
    println!("read_only=true samples={samples} duration_ms={}", start.elapsed().as_millis());
    Ok(())
}

#[cfg(test)]
mod diagnosis {
    use super::playback::{OutputVolume, PlaybackDucker, VolumeSnapshot};
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    struct ChangingDefault(Arc<Mutex<(usize, [VolumeSnapshot; 2])>>);
    impl OutputVolume for ChangingDefault {
        fn get(&self) -> Result<VolumeSnapshot, String> {
            let state = self.0.lock().unwrap();
            Ok(state.1[state.0])
        }
        fn set_muted(&self, muted: bool) -> Result<(), String> {
            let mut state = self.0.lock().unwrap();
            let index = state.0;
            state.1[index].muted = muted;
            Ok(())
        }
        fn set_volume(&self, volume: f32) -> Result<(), String> {
            let mut state = self.0.lock().unwrap();
            let index = state.0;
            state.1[index].volume = volume;
            Ok(())
        }
        fn get_bound(&self) -> Result<(VolumeSnapshot, Option<String>), String> {
            let state = self.0.lock().unwrap();
            Ok((state.1[state.0], Some(state.0.to_string())))
        }
        fn set_muted_on(&self, endpoint: Option<&str>, muted: bool) -> Result<(), String> {
            let index = endpoint.unwrap().parse::<usize>().unwrap();
            self.0.lock().unwrap().1[index].muted = muted;
            Ok(())
        }
        fn set_volume_on(&self, endpoint: Option<&str>, volume: f32) -> Result<(), String> {
            let index = endpoint.unwrap().parse::<usize>().unwrap();
            self.0.lock().unwrap().1[index].volume = volume;
            Ok(())
        }
    }

    // This assertion failed against the prior production state machine. It now
    // specifies the required behavior without changing any OS audio endpoint.
    #[test]
    fn restores_original_endpoint_when_default_changes() {
        let endpoints = ChangingDefault(Arc::new(Mutex::new((0, [
            VolumeSnapshot { muted: false, volume: 0.70 },
            VolumeSnapshot { muted: true, volume: 0.20 },
        ]))));
        let ducker = PlaybackDucker::new(endpoints.clone());
        ducker.duck();
        endpoints.0.lock().unwrap().0 = 1;
        ducker.restore();
        let state = endpoints.0.lock().unwrap();
        assert_eq!(state.1[0], VolumeSnapshot { muted: false, volume: 0.70 }, "original endpoint must be restored");
        assert_eq!(state.1[1], VolumeSnapshot { muted: true, volume: 0.20 }, "replacement endpoint must remain untouched");
    }
}
