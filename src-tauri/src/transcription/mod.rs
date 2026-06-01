pub mod local;
pub use local::LocalWhisper;

use anyhow::Result;

/// Common interface — both local and cloud backends implement this.
pub trait TranscriptionEngine: Send + Sync {
    fn transcribe(&self, samples: &[f32]) -> Result<String>;
}

impl TranscriptionEngine for LocalWhisper {
    fn transcribe(&self, samples: &[f32]) -> Result<String> {
        LocalWhisper::transcribe(self, samples)
    }
}
