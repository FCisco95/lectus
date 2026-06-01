//! Cross-platform low-level keyboard hook for hold-to-talk activation.
//!
//! On key-down of the configured target key the platform hook sets
//! `recording_active = true` and emits `hold-start`; on key-up it clears the
//! flag and emits `hold-stop`. The key-string→native-key maps below are the
//! unit-testable core shared by both platforms.

// Platform implementations + re-export are added in later tasks.
