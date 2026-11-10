use std::io::Cursor;

pub fn pcm_to_wav(pcm: &[u8]) -> Result<Vec<u8>, hound::Error> {
    let mut output = Cursor::new(Vec::new());

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    {
        let mut writer = hound::WavWriter::new(&mut output, spec)?;

        for bytes in pcm.chunks_exact(2) {
            writer.write_sample(i16::from_le_bytes([bytes[0], bytes[1]]))?;
        }

        writer.finalize()?;
    }

    Ok(output.into_inner())
}
