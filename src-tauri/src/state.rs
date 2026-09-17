use std::sync::Mutex;
use crate::config::Config;

#[derive(Debug, Clone, PartialEq)]
pub enum RecordingState {
    Idle,
    Recording,
    Transcribing,
    Error(String),
}

pub struct AppState {
    pub recording: Mutex<RecordingState>,
    pub config: Mutex<Config>,
}

impl AppState {
    pub fn new() -> Self {
        Self::with_config(Config::default())
    }

    pub fn with_config(config: Config) -> Self {
        Self {
            recording: Mutex::new(RecordingState::Idle),
            config: Mutex::new(config),
        }
    }

    pub fn set_state(&self, next: RecordingState) {
        *self.recording.lock().unwrap() = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state_is_idle() {
        let s = AppState::new();
        assert_eq!(*s.recording.lock().unwrap(), RecordingState::Idle);
    }

    #[test]
    fn test_transition_idle_to_recording() {
        let s = AppState::new();
        s.set_state(RecordingState::Recording);
        assert_eq!(*s.recording.lock().unwrap(), RecordingState::Recording);
    }

    #[test]
    fn test_transition_recording_to_transcribing() {
        let s = AppState::new();
        s.set_state(RecordingState::Recording);
        s.set_state(RecordingState::Transcribing);
        assert_eq!(*s.recording.lock().unwrap(), RecordingState::Transcribing);
    }
}
