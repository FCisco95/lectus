//! Duck system playback while the dictation key is held, then restore it.
//!
//! Capture, VAD, and injection are untouched. Windows mutes the default
//! render endpoint (`IAudioEndpointVolume`) so music, YouTube, and exclusive-
//! mode games go quiet; devices that reject mute fall back to volume 0.
//! Restore is idempotent and runs on every exit path so the PC is never left
//! silent.

use std::sync::Mutex;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VolumeSnapshot {
    pub muted: bool,
    pub volume: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DuckAction {
    Mute,
    VolumeZero,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaybackEvent {
    HoldStart,
    HoldStop,
    PipelineError,
    Idle,
    ProcessExit,
}

pub trait OutputVolume {
    fn get(&self) -> Result<VolumeSnapshot, String>;
    fn set_muted(&self, muted: bool) -> Result<(), String>;
    fn set_volume(&self, volume: f32) -> Result<(), String>;
}

struct Ducked {
    snapshot: VolumeSnapshot,
    action: DuckAction,
}

pub struct PlaybackDucker<V: OutputVolume> {
    volume: V,
    ducked: Mutex<Option<Ducked>>,
}

impl<V: OutputVolume> PlaybackDucker<V> {
    pub fn new(volume: V) -> Self {
        Self {
            volume,
            ducked: Mutex::new(None),
        }
    }

    /// Mute (or duck) playback. No-op if already ducked or if the device
    /// cannot be read. An already-muted device is left alone so restore
    /// does not unmute a user mute.
    pub fn duck(&self) {
        let mut slot = self.ducked.lock().unwrap();
        if slot.is_some() {
            return;
        }
        let snapshot = match self.volume.get() {
            Ok(s) => s,
            Err(e) => {
                log::warn!("playback: could not read output volume: {e}");
                return;
            }
        };
        if snapshot.muted {
            log::info!("playback: output already muted, leaving as-is");
            return;
        }
        let action = match self.volume.set_muted(true) {
            Ok(()) => DuckAction::Mute,
            Err(e) => {
                log::warn!("playback: SetMute failed ({e}), ducking volume to 0");
                if let Err(e) = self.volume.set_volume(0.0) {
                    log::warn!("playback: volume duck failed: {e}");
                    return;
                }
                DuckAction::VolumeZero
            }
        };
        log::info!("playback: ducked default output ({action:?})");
        *slot = Some(Ducked { snapshot, action });
    }

    /// Restore the pre-duck mute/volume. Idempotent; safe if never ducked.
    pub fn restore(&self) {
        let Some(ducked) = self.ducked.lock().unwrap().take() else {
            return;
        };
        let result = match ducked.action {
            DuckAction::Mute => self.volume.set_muted(false),
            DuckAction::VolumeZero => self.volume.set_volume(ducked.snapshot.volume),
        };
        match result {
            Ok(()) => log::info!("playback: restored default output"),
            Err(e) => log::warn!("playback: restore failed: {e}"),
        }
    }
}

impl<V: OutputVolume> Drop for PlaybackDucker<V> {
    fn drop(&mut self) {
        self.restore();
    }
}

/// Mute only on hold-start (when the setting is on). Every other event
/// restores so a missed key-up, pipeline error, idle transition, or process
/// exit cannot leave the speakers silent.
pub fn on_event<V: OutputVolume>(ducker: &PlaybackDucker<V>, enabled: bool, event: PlaybackEvent) {
    match event {
        PlaybackEvent::HoldStart if enabled => ducker.duck(),
        PlaybackEvent::HoldStart => {}
        PlaybackEvent::HoldStop
        | PlaybackEvent::PipelineError
        | PlaybackEvent::Idle
        | PlaybackEvent::ProcessExit => ducker.restore(),
    }
}

/// Process-lifetime ducker used by the Tauri app.
pub type AppPlayback = PlaybackDucker<PlatformOutput>;

pub fn platform_ducker() -> AppPlayback {
    PlaybackDucker::new(PlatformOutput)
}

/// Default-render endpoint on Windows; no-op elsewhere (Windows-first).
pub struct PlatformOutput;

#[cfg(not(windows))]
impl OutputVolume for PlatformOutput {
    fn get(&self) -> Result<VolumeSnapshot, String> {
        Ok(VolumeSnapshot {
            muted: false,
            volume: 1.0,
        })
    }
    fn set_muted(&self, _muted: bool) -> Result<(), String> {
        Ok(())
    }
    fn set_volume(&self, _volume: f32) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(windows)]
mod win {
    use super::{OutputVolume, VolumeSnapshot};
    use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
    use windows::Win32::Media::Audio::{eMultimedia, eRender, IMMDeviceEnumerator, MMDeviceEnumerator};
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED,
    };

    fn with_endpoint<T>(f: impl FnOnce(&IAudioEndpointVolume) -> Result<T, String>) -> Result<T, String> {
        unsafe {
            // Already-initialized STA/MTA is fine; ignore the HRESULT.
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                    .map_err(|e| format!("MMDeviceEnumerator: {e}"))?;
            let device = enumerator
                .GetDefaultAudioEndpoint(eRender, eMultimedia)
                .map_err(|e| format!("GetDefaultAudioEndpoint: {e}"))?;
            let volume: IAudioEndpointVolume = device
                .Activate(CLSCTX_ALL, None)
                .map_err(|e| format!("Activate IAudioEndpointVolume: {e}"))?;
            f(&volume)
        }
    }

    impl OutputVolume for super::PlatformOutput {
        fn get(&self) -> Result<VolumeSnapshot, String> {
            with_endpoint(|vol| unsafe {
                let muted = vol.GetMute().map_err(|e| format!("GetMute: {e}"))?.as_bool();
                let volume = vol
                    .GetMasterVolumeLevelScalar()
                    .map_err(|e| format!("GetMasterVolumeLevelScalar: {e}"))?;
                Ok(VolumeSnapshot { muted, volume })
            })
        }

        fn set_muted(&self, muted: bool) -> Result<(), String> {
            with_endpoint(|vol| unsafe {
                vol.SetMute(muted, std::ptr::null())
                    .map_err(|e| format!("SetMute: {e}"))?;
                Ok(())
            })
        }

        fn set_volume(&self, volume: f32) -> Result<(), String> {
            with_endpoint(|vol| unsafe {
                vol.SetMasterVolumeLevelScalar(volume, std::ptr::null())
                    .map_err(|e| format!("SetMasterVolumeLevelScalar: {e}"))?;
                Ok(())
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    struct FakeOutput {
        inner: Arc<Mutex<FakeState>>,
    }

    struct FakeState {
        muted: bool,
        volume: f32,
        mute_ok: bool,
        get_ok: bool,
    }

    impl FakeOutput {
        fn new(muted: bool, volume: f32) -> Self {
            Self {
                inner: Arc::new(Mutex::new(FakeState {
                    muted,
                    volume,
                    mute_ok: true,
                    get_ok: true,
                })),
            }
        }

        fn snapshot(&self) -> (bool, f32) {
            let s = self.inner.lock().unwrap();
            (s.muted, s.volume)
        }

        fn fail_mute(&self) {
            self.inner.lock().unwrap().mute_ok = false;
        }

        fn fail_get(&self) {
            self.inner.lock().unwrap().get_ok = false;
        }
    }

    impl OutputVolume for FakeOutput {
        fn get(&self) -> Result<VolumeSnapshot, String> {
            let s = self.inner.lock().unwrap();
            if !s.get_ok {
                return Err("no device".into());
            }
            Ok(VolumeSnapshot {
                muted: s.muted,
                volume: s.volume,
            })
        }

        fn set_muted(&self, muted: bool) -> Result<(), String> {
            let mut s = self.inner.lock().unwrap();
            if !s.mute_ok {
                return Err("mute unsupported".into());
            }
            s.muted = muted;
            Ok(())
        }

        fn set_volume(&self, volume: f32) -> Result<(), String> {
            self.inner.lock().unwrap().volume = volume;
            Ok(())
        }
    }

    #[test]
    fn duck_mutes_then_restore_unmutes_at_same_volume() {
        let fake = FakeOutput::new(false, 0.7);
        let ducker = PlaybackDucker::new(fake.clone());
        ducker.duck();
        assert_eq!(fake.snapshot(), (true, 0.7));
        ducker.restore();
        assert_eq!(fake.snapshot(), (false, 0.7));
    }

    #[test]
    fn already_muted_stays_muted_after_restore() {
        let fake = FakeOutput::new(true, 0.4);
        let ducker = PlaybackDucker::new(fake.clone());
        ducker.duck();
        assert_eq!(fake.snapshot(), (true, 0.4));
        ducker.restore();
        assert_eq!(fake.snapshot(), (true, 0.4));
    }

    #[test]
    fn mute_failure_ducks_volume_to_zero_then_restores() {
        let fake = FakeOutput::new(false, 0.55);
        fake.fail_mute();
        let ducker = PlaybackDucker::new(fake.clone());
        ducker.duck();
        assert_eq!(fake.snapshot(), (false, 0.0));
        ducker.restore();
        assert_eq!(fake.snapshot(), (false, 0.55));
    }

    #[test]
    fn drop_restores() {
        let fake = FakeOutput::new(false, 1.0);
        {
            let ducker = PlaybackDucker::new(fake.clone());
            ducker.duck();
            assert_eq!(fake.snapshot(), (true, 1.0));
        }
        assert_eq!(fake.snapshot(), (false, 1.0));
    }

    #[test]
    fn restore_is_idempotent() {
        let fake = FakeOutput::new(false, 0.9);
        let ducker = PlaybackDucker::new(fake.clone());
        ducker.duck();
        ducker.restore();
        ducker.restore();
        assert_eq!(fake.snapshot(), (false, 0.9));
    }

    #[test]
    fn second_duck_keeps_original_snapshot() {
        let fake = FakeOutput::new(false, 0.8);
        fake.fail_mute();
        let ducker = PlaybackDucker::new(fake.clone());
        ducker.duck();
        assert_eq!(fake.snapshot(), (false, 0.0));
        ducker.duck(); // must not snapshot the already-ducked 0.0
        ducker.restore();
        assert_eq!(fake.snapshot(), (false, 0.8));
    }

    #[test]
    fn get_failure_is_a_noop() {
        let fake = FakeOutput::new(false, 0.3);
        fake.fail_get();
        let ducker = PlaybackDucker::new(fake.clone());
        ducker.duck();
        ducker.restore();
        assert_eq!(fake.snapshot(), (false, 0.3));
    }

    #[test]
    fn hold_start_ducks_only_when_enabled() {
        let fake = FakeOutput::new(false, 1.0);
        let ducker = PlaybackDucker::new(fake.clone());
        on_event(&ducker, false, PlaybackEvent::HoldStart);
        assert_eq!(fake.snapshot(), (false, 1.0));
        on_event(&ducker, true, PlaybackEvent::HoldStart);
        assert_eq!(fake.snapshot(), (true, 1.0));
    }

    #[cfg(windows)]
    #[test]
    fn windows_default_render_endpoint_is_readable() {
        let snap = PlatformOutput
            .get()
            .expect("default render endpoint should expose IAudioEndpointVolume");
        assert!(
            (0.0..=1.0).contains(&snap.volume),
            "volume scalar out of range: {}",
            snap.volume
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_mute_roundtrip_restores() {
        let before = PlatformOutput.get().expect("read volume before duck");
        let ducker = PlaybackDucker::new(PlatformOutput);
        ducker.duck();
        ducker.restore();
        let after = PlatformOutput.get().expect("read volume after restore");
        assert_eq!(before.muted, after.muted);
        assert!(
            (before.volume - after.volume).abs() < 0.02,
            "volume drifted: {} -> {}",
            before.volume,
            after.volume
        );
    }

    #[test]
    fn every_exit_path_restores() {
        let exits = [
            PlaybackEvent::HoldStop,
            PlaybackEvent::PipelineError,
            PlaybackEvent::Idle,
            PlaybackEvent::ProcessExit,
        ];
        for event in exits {
            let fake = FakeOutput::new(false, 0.6);
            let ducker = PlaybackDucker::new(fake.clone());
            on_event(&ducker, true, PlaybackEvent::HoldStart);
            assert_eq!(fake.snapshot(), (true, 0.6), "ducked before {event:?}");
            on_event(&ducker, true, event);
            assert_eq!(
                fake.snapshot(),
                (false, 0.6),
                "restore on {event:?} must unmute"
            );
        }
    }
}
