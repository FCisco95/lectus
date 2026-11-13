use super::ApiError;
use std::io::Cursor;

pub const MAX_BYTES: usize = 8 * 1024 * 1024;
const MAX_SAMPLES: usize = 120 * 16_000;

pub fn decode(bytes: &[u8]) -> Result<(Vec<f32>, u64), ApiError> {
    if bytes.len() > MAX_BYTES { return Err(ApiError::too_large()); }
    let mut reader = hound::WavReader::new(Cursor::new(bytes)).map_err(|_| bad_audio())?;
    let spec = reader.spec();
    if spec.channels != 1 || spec.sample_rate != 16_000 || reader.len() == 0 {
        return Err(bad_audio());
    }
    if reader.len() as usize > MAX_SAMPLES { return Err(ApiError::too_large()); }
    let samples: Vec<f32> = match (spec.sample_format, spec.bits_per_sample) {
        (hound::SampleFormat::Int, 16) => reader.samples::<i16>()
            .map(|v| v.map(|v| v as f32 / 32768.0).map_err(|_| bad_audio())).collect::<Result<_, _>>()?,
        (hound::SampleFormat::Float, 32) => reader.samples::<f32>()
            .map(|v| v.map_err(|_| bad_audio())).collect::<Result<_, _>>()?,
        _ => return Err(bad_audio()),
    };
    if samples.is_empty() || samples.iter().any(|v| !v.is_finite() || v.abs() > 1.0) {
        return Err(bad_audio());
    }
    let duration_ms = samples.len() as u64 * 1000 / 16_000;
    Ok((samples, duration_ms))
}

fn bad_audio() -> ApiError {
    ApiError::new(400, "bad_audio", "Upload nonempty 16 kHz mono WAV audio (16-bit PCM or 32-bit float).")
}

#[cfg(test)]
pub(super) fn fixture(rate: u32, channels: u16, count: usize) -> Vec<u8> {
    let mut cursor = Cursor::new(Vec::new());
    let mut writer = hound::WavWriter::new(&mut cursor, hound::WavSpec {
        channels, sample_rate: rate, bits_per_sample: 16, sample_format: hound::SampleFormat::Int,
    }).unwrap();
    for _ in 0..count { writer.write_sample(0i16).unwrap(); }
    writer.finalize().unwrap();
    cursor.into_inner()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_wav_shape_and_duration_before_inference() {
        assert!(decode(b"not audio").is_err());
        assert!(decode(&fixture(44_100, 1, 100)).is_err());
        assert!(decode(&fixture(16_000, 2, 100)).is_err());
        assert!(decode(&fixture(16_000, 1, 0)).is_err());
        assert_eq!(decode(&fixture(16_000, 1, 160)).unwrap().1, 10);
        assert_eq!(decode(&fixture(16_000, 1, MAX_SAMPLES + 1)).unwrap_err().status, 413);
        assert_eq!(decode(&vec![0; MAX_BYTES + 1]).unwrap_err().status, 413);
    }
}
