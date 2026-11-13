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
    /// Bind the snapshot to one endpoint. Single-endpoint adapters can retain
    /// the default methods; Windows supplies the stable Core Audio device ID.
    fn get_bound(&self) -> Result<(VolumeSnapshot, Option<String>), String> {
        self.get().map(|snapshot| (snapshot, None))
    }
    fn set_muted_on(&self, _endpoint: Option<&str>, muted: bool) -> Result<(), String> {
        self.set_muted(muted)
    }
    fn set_volume_on(&self, _endpoint: Option<&str>, volume: f32) -> Result<(), String> {
        self.set_volume(volume)
    }
}

struct Ducked {
    snapshot: VolumeSnapshot,
    action: DuckAction,
    endpoint: Option<String>,
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
        let (snapshot, endpoint) = match self.volume.get_bound() {
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
        let action = match self.volume.set_muted_on(endpoint.as_deref(), true) {
            Ok(()) => DuckAction::Mute,
            Err(e) => {
                log::warn!("playback: SetMute failed ({e}), ducking volume to 0");
                if let Err(e) = self.volume.set_volume_on(endpoint.as_deref(), 0.0) {
                    log::warn!("playback: volume duck failed: {e}");
                    return;
                }
                DuckAction::VolumeZero
            }
        };
        log::info!(
            "playback: ducked default output ({action:?}), saved {:.0}%",
            snapshot.volume * 100.0
        );
        *slot = Some(Ducked { snapshot, action, endpoint });
    }

    /// Restore the pre-duck mute AND volume. Keep the snapshot on failure
    /// so the next stop/idle/exit event can retry instead of forgetting it.
    pub fn restore(&self) {
        // Serialize OS writes with duck(): otherwise a new recording can
        // snapshot the lowered volume while restoration is still in flight.
        let mut slot = self.ducked.lock().unwrap();
        let Some(ducked) = slot.as_ref() else {
            return;
        };
        // Restore the level while still muted to avoid a loud transient.
        // Always try to unmute even if restoring the level fails.
        let volume_result = self.volume.set_volume_on(ducked.endpoint.as_deref(), ducked.snapshot.volume);
        let mute_result = match ducked.action {
            DuckAction::Mute => self.volume.set_muted_on(ducked.endpoint.as_deref(), ducked.snapshot.muted),
            DuckAction::VolumeZero => Ok(()),
        };
        match volume_result.and(mute_result) {
            Ok(()) => {
                log::info!(
                    "playback: restored output to {:.0}%",
                    ducked.snapshot.volume * 100.0
                );
                *slot = None;
            }
            Err(e) => log::warn!("playback: restore failed (snapshot retained for retry): {e}"),
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
    use windows::Win32::Media::Audio::{
        eMultimedia, eRender, IMMDeviceEnumerator, MMDeviceEnumerator,
    };
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_MULTITHREADED,
    };
    use windows::core::PCWSTR;

    fn with_endpoint<T>(
        endpoint: Option<&str>,
        f: impl FnOnce(&IAudioEndpointVolume) -> Result<T, String>,
    ) -> Result<T, String> {
        unsafe {
            // Already-initialized STA/MTA is fine; ignore the HRESULT.
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                    .map_err(|e| format!("MMDeviceEnumerator: {e}"))?;
            // A missing saved device is an error, never permission to mutate
            // whichever unrelated endpoint became the default in its place.
            let device = if let Some(id) = endpoint {
                let wide: Vec<u16> = id.encode_utf16().chain(Some(0)).collect();
                enumerator.GetDevice(PCWSTR(wide.as_ptr()))
                    .map_err(|e| format!("GetDevice for saved playback endpoint: {e}"))?
            } else {
                enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)
                    .map_err(|e| format!("GetDefaultAudioEndpoint: {e}"))?
            };
            let volume: IAudioEndpointVolume = device
                .Activate(CLSCTX_ALL, None)
                .map_err(|e| format!("Activate IAudioEndpointVolume: {e}"))?;
            f(&volume)
        }
    }

    impl OutputVolume for super::PlatformOutput {
        fn get(&self) -> Result<VolumeSnapshot, String> {
            with_endpoint(None, |vol| unsafe {
                let muted = vol
                    .GetMute()
                    .map_err(|e| format!("GetMute: {e}"))?
                    .as_bool();
                let volume = vol
                    .GetMasterVolumeLevelScalar()
                    .map_err(|e| format!("GetMasterVolumeLevelScalar: {e}"))?;
                Ok(VolumeSnapshot { muted, volume })
            })
        }

        fn set_muted(&self, muted: bool) -> Result<(), String> {
            self.set_muted_on(None, muted)
        }

        fn get_bound(&self) -> Result<(VolumeSnapshot, Option<String>), String> {
            let id = unsafe {
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
                let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                    .map_err(|e| format!("MMDeviceEnumerator: {e}"))?;
                let device = enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)
                    .map_err(|e| format!("GetDefaultAudioEndpoint: {e}"))?;
                let allocated = device.GetId().map_err(|e| format!("GetId for playback endpoint: {e}"))?;
                let id = allocated.to_string().map_err(|e| format!("Playback endpoint ID: {e}"));
                CoTaskMemFree(Some(allocated.0.cast()));
                id?
            };
            let snapshot = with_endpoint(Some(&id), |vol| unsafe {
                Ok(VolumeSnapshot {
                    muted: vol.GetMute().map_err(|e| format!("GetMute: {e}"))?.as_bool(),
                    volume: vol.GetMasterVolumeLevelScalar().map_err(|e| format!("GetMasterVolumeLevelScalar: {e}"))?,
                })
            })?;
            Ok((snapshot, Some(id)))
        }

        fn set_muted_on(&self, endpoint: Option<&str>, muted: bool) -> Result<(), String> {
            with_endpoint(endpoint, |vol| unsafe {
                vol.SetMute(muted, std::ptr::null())
                    .map_err(|e| format!("SetMute: {e}"))?;
                Ok(())
            })
        }

        fn set_volume(&self, volume: f32) -> Result<(), String> {
            self.set_volume_on(None, volume)
        }

        fn set_volume_on(&self, endpoint: Option<&str>, volume: f32) -> Result<(), String> {
            with_endpoint(endpoint, |vol| unsafe {
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
        volume_ok: bool,
    }

    impl FakeOutput {
        fn new(muted: bool, volume: f32) -> Self {
            Self {
                inner: Arc::new(Mutex::new(FakeState {
                    muted,
                    volume,
                    mute_ok: true,
                    get_ok: true,
                    volume_ok: true,
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
            let mut s = self.inner.lock().unwrap();
            if !s.volume_ok {
                return Err("volume unavailable".into());
            }
            s.volume = volume;
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
    fn restores_original_volume_when_level_drops_during_mute() {
        for reduced in [0.01, 0.04] {
            let fake = FakeOutput::new(false, 0.7);
            let ducker = PlaybackDucker::new(fake.clone());
            ducker.duck();
            // Model an external level change while muted. Its source is not
            // known; dictation must still restore the pre-recording level.
            fake.set_volume(reduced).unwrap();
            ducker.restore();
            assert_eq!(fake.snapshot(), (false, 0.7));
        }
    }

    #[test]
    fn failed_volume_restore_keeps_snapshot_for_idle_retry() {
        let fake = FakeOutput::new(false, 0.65);
        fake.fail_mute();
        let ducker = PlaybackDucker::new(fake.clone());
        ducker.duck();
        fake.inner.lock().unwrap().volume_ok = false;
        on_event(&ducker, true, PlaybackEvent::HoldStop);
        assert_eq!(fake.snapshot(), (false, 0.0));
        // A new start must not replace the saved level with the ducked zero.
        ducker.duck();
        fake.inner.lock().unwrap().volume_ok = true;
        on_event(&ducker, true, PlaybackEvent::Idle);
        assert_eq!(fake.snapshot(), (false, 0.65));
    }

    #[test]
    fn failed_unmute_keeps_snapshot_for_idle_retry() {
        let fake = FakeOutput::new(false, 0.6);
        let ducker = PlaybackDucker::new(fake.clone());
        ducker.duck();
        fake.fail_mute();
        on_event(&ducker, true, PlaybackEvent::HoldStop);
        assert_eq!(fake.snapshot(), (true, 0.6));
        fake.inner.lock().unwrap().mute_ok = true;
        on_event(&ducker, true, PlaybackEvent::Idle);
        assert_eq!(fake.snapshot(), (false, 0.6));
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

    struct RoutedState {
        default: usize,
        devices: [Option<VolumeSnapshot>; 2],
        switch_after_snapshot: bool,
        fail_volume: bool,
    }

    #[derive(Clone)]
    struct RoutedOutput(Arc<Mutex<RoutedState>>);

    impl RoutedOutput {
        fn new() -> Self {
            Self(Arc::new(Mutex::new(RoutedState {
                default: 0,
                devices: [Some(VolumeSnapshot { muted: false, volume: 0.7 }), Some(VolumeSnapshot { muted: true, volume: 0.2 })],
                switch_after_snapshot: false,
                fail_volume: false,
            })))
        }
    }

    impl OutputVolume for RoutedOutput {
        fn get(&self) -> Result<VolumeSnapshot, String> {
            let state = self.0.lock().unwrap();
            state.devices[state.default].ok_or_else(|| "endpoint unavailable".into())
        }
        fn set_muted(&self, _: bool) -> Result<(), String> { Err("unbound write forbidden".into()) }
        fn set_volume(&self, _: f32) -> Result<(), String> { Err("unbound write forbidden".into()) }
        fn get_bound(&self) -> Result<(VolumeSnapshot, Option<String>), String> {
            let mut state = self.0.lock().unwrap();
            let index = state.default;
            let snapshot = state.devices[index].ok_or("endpoint unavailable")?;
            if state.switch_after_snapshot { state.default = 1; }
            Ok((snapshot, Some(index.to_string())))
        }
        fn set_muted_on(&self, endpoint: Option<&str>, muted: bool) -> Result<(), String> {
            let index = endpoint.ok_or("unbound endpoint")?.parse::<usize>().map_err(|e| e.to_string())?;
            let mut state = self.0.lock().unwrap();
            state.devices[index].as_mut().ok_or("endpoint unavailable")?.muted = muted;
            Ok(())
        }
        fn set_volume_on(&self, endpoint: Option<&str>, volume: f32) -> Result<(), String> {
            let index = endpoint.ok_or("unbound endpoint")?.parse::<usize>().map_err(|e| e.to_string())?;
            let mut state = self.0.lock().unwrap();
            if state.fail_volume { return Err("volume restore failed".into()); }
            state.devices[index].as_mut().ok_or("endpoint unavailable")?.volume = volume;
            Ok(())
        }
    }

    #[test]
    fn endpoint_change_between_snapshot_and_mute_preserves_new_default() {
        let output = RoutedOutput::new();
        output.0.lock().unwrap().switch_after_snapshot = true;
        let ducker = PlaybackDucker::new(output.clone());
        ducker.duck();
        {
            let mut state = output.0.lock().unwrap();
            assert_eq!(state.devices[0].unwrap().muted, true);
            assert_eq!(state.devices[1].unwrap(), VolumeSnapshot { muted: true, volume: 0.2 });
            // A user's independent adjustment to B must survive restoring A.
            state.devices[1].as_mut().unwrap().volume = 0.33;
        }
        ducker.restore();
        let state = output.0.lock().unwrap();
        assert_eq!(state.devices[0].unwrap(), VolumeSnapshot { muted: false, volume: 0.7 });
        assert_eq!(state.devices[1].unwrap(), VolumeSnapshot { muted: true, volume: 0.33 });
    }

    #[test]
    fn missing_original_endpoint_retains_snapshot_without_touching_replacement() {
        let output = RoutedOutput::new();
        let ducker = PlaybackDucker::new(output.clone());
        ducker.duck();
        {
            let mut state = output.0.lock().unwrap();
            state.default = 1;
            state.devices[0] = None;
        }
        ducker.restore();
        ducker.duck(); // must not replace the pending A snapshot with B
        {
            let mut state = output.0.lock().unwrap();
            assert_eq!(state.devices[1].unwrap(), VolumeSnapshot { muted: true, volume: 0.2 });
            state.devices[0] = Some(VolumeSnapshot { muted: true, volume: 0.01 });
        }
        ducker.restore();
        let state = output.0.lock().unwrap();
        assert_eq!(state.devices[0].unwrap(), VolumeSnapshot { muted: false, volume: 0.7 });
        assert_eq!(state.devices[1].unwrap(), VolumeSnapshot { muted: true, volume: 0.2 });
    }

    #[test]
    fn failed_bound_restore_retries_original_endpoint_after_default_change() {
        let output = RoutedOutput::new();
        let ducker = PlaybackDucker::new(output.clone());
        ducker.duck();
        {
            let mut state = output.0.lock().unwrap();
            state.default = 1;
            state.fail_volume = true;
            state.devices[0].as_mut().unwrap().volume = 0.04;
        }
        ducker.restore();
        output.0.lock().unwrap().fail_volume = false;
        on_event(&ducker, true, PlaybackEvent::Idle);
        let state = output.0.lock().unwrap();
        assert_eq!(state.devices[0].unwrap(), VolumeSnapshot { muted: false, volume: 0.7 });
        assert_eq!(state.devices[1].unwrap(), VolumeSnapshot { muted: true, volume: 0.2 });
    }

    #[cfg(windows)]
    #[test]
    fn windows_bound_snapshot_is_readable_without_mutating_audio() {
        let (snapshot, endpoint) = PlatformOutput.get_bound().expect("read bound render endpoint");
        assert!(!endpoint.expect("stable Windows endpoint ID").is_empty());
        assert!((0.0..=1.0).contains(&snapshot.volume));
    }
}
