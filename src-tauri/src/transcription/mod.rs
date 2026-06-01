pub mod cloud;
pub mod local;
pub use cloud::CloudWhisper;
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

impl TranscriptionEngine for CloudWhisper {
    fn transcribe(&self, samples: &[f32]) -> Result<String> {
        CloudWhisper::transcribe(self, samples)
    }
}
