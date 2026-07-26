use std::borrow::Cow;
use std::io::{Cursor, Read, Seek};

use ogg::{PacketReader, PacketWriteEndInfo, PacketWriter};
use opus_head_sys::{
    opus_decode_float, opus_decoder_create, opus_decoder_destroy, opus_encode, opus_encoder_create,
    opus_encoder_ctl, opus_encoder_destroy, OpusDecoder, OpusEncoder, OPUS_APPLICATION_AUDIO,
    OPUS_GET_LOOKAHEAD_REQUEST, OPUS_OK, OPUS_SET_BITRATE_REQUEST,
};
use rand::Rng as _;

use crate::{MediaError, MediaResult};

pub const VOICE_SAMPLE_RATE: u32 = 16_000;
const PLAYBACK_SAMPLE_RATE: u32 = 48_000;
const FRAME_SAMPLES: usize = 320;
const MAX_PACKET_BYTES: usize = 4_000;

struct Encoder(*mut OpusEncoder);

impl Drop for Encoder {
    fn drop(&mut self) {
        unsafe { opus_encoder_destroy(self.0) };
    }
}

struct Decoder(*mut OpusDecoder);

impl Drop for Decoder {
    fn drop(&mut self) {
        unsafe { opus_decoder_destroy(self.0) };
    }
}

pub fn encode_voice(samples: &[i16]) -> MediaResult<Vec<u8>> {
    if samples.is_empty() {
        return Err(MediaError::EmptyRecording);
    }

    let mut error = OPUS_OK as i32;
    let encoder = Encoder(unsafe {
        opus_encoder_create(
            VOICE_SAMPLE_RATE as i32,
            1,
            OPUS_APPLICATION_AUDIO as i32,
            &mut error,
        )
    });
    if encoder.0.is_null() || error != OPUS_OK as i32 {
        return Err(MediaError::Opus(error));
    }
    let bitrate_result =
        unsafe { opus_encoder_ctl(encoder.0, OPUS_SET_BITRATE_REQUEST as i32, 16_000_i32) };
    if bitrate_result != OPUS_OK as i32 {
        return Err(MediaError::Opus(bitrate_result));
    }
    let mut lookahead = 0_i32;
    let lookahead_result =
        unsafe { opus_encoder_ctl(encoder.0, OPUS_GET_LOOKAHEAD_REQUEST as i32, &mut lookahead) };
    if lookahead_result != OPUS_OK as i32 {
        return Err(MediaError::Opus(lookahead_result));
    }

    let pre_skip_48k = (lookahead.max(0) as u32 * PLAYBACK_SAMPLE_RATE / VOICE_SAMPLE_RATE) as u16;
    let serial = rand::rng().random::<u32>();
    let mut encoded = Vec::new();
    let mut writer = PacketWriter::new(&mut encoded);

    let mut header = [0_u8; 19];
    header[..8].copy_from_slice(b"OpusHead");
    header[8] = 1;
    header[9] = 1;
    header[10..12].copy_from_slice(&pre_skip_48k.to_le_bytes());
    header[12..16].copy_from_slice(&VOICE_SAMPLE_RATE.to_le_bytes());
    writer.write_packet(
        Cow::Owned(header.to_vec()),
        serial,
        PacketWriteEndInfo::EndPage,
        0,
    )?;

    let vendor = b"mixin_desktop_media";
    let mut tags = Vec::with_capacity(16 + vendor.len());
    tags.extend_from_slice(b"OpusTags");
    tags.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
    tags.extend_from_slice(vendor);
    tags.extend_from_slice(&0_u32.to_le_bytes());
    writer.write_packet(Cow::Owned(tags), serial, PacketWriteEndInfo::EndPage, 0)?;

    let lookahead = lookahead.max(0) as usize;
    let mut input = vec![0_i16; lookahead];
    input.extend_from_slice(samples);
    let frame_count = input.len().div_ceil(FRAME_SAMPLES);
    input.resize(frame_count * FRAME_SAMPLES, 0);
    let final_granule = u64::from(pre_skip_48k)
        + samples.len() as u64 * u64::from(PLAYBACK_SAMPLE_RATE) / u64::from(VOICE_SAMPLE_RATE);

    for (index, frame) in input.chunks_exact(FRAME_SAMPLES).enumerate() {
        let mut packet = vec![0_u8; MAX_PACKET_BYTES];
        let packet_size = unsafe {
            opus_encode(
                encoder.0,
                frame.as_ptr(),
                FRAME_SAMPLES as i32,
                packet.as_mut_ptr(),
                packet.len() as i32,
            )
        };
        if packet_size < 0 {
            return Err(MediaError::Opus(packet_size));
        }
        packet.truncate(packet_size as usize);
        let last = index + 1 == frame_count;
        let granule = if last {
            final_granule
        } else {
            ((index + 1) * FRAME_SAMPLES) as u64 * u64::from(PLAYBACK_SAMPLE_RATE)
                / u64::from(VOICE_SAMPLE_RATE)
        };
        writer.write_packet(
            Cow::Owned(packet),
            serial,
            if last {
                PacketWriteEndInfo::EndStream
            } else {
                PacketWriteEndInfo::NormalPacket
            },
            granule,
        )?;
    }
    drop(writer);
    Ok(encoded)
}

pub struct DecodedAudio {
    pub samples: Vec<f32>,
    pub channels: u16,
    pub sample_rate: u32,
}

pub fn decode_ogg_opus(bytes: Vec<u8>) -> MediaResult<DecodedAudio> {
    decode_from(Cursor::new(bytes))
}

fn decode_from<R: Read + Seek>(source: R) -> MediaResult<DecodedAudio> {
    let mut reader = PacketReader::new(source);
    let header = reader
        .read_packet_expected()
        .map_err(|error| MediaError::MalformedAudio(error.to_string()))?;
    if header.data.len() < 19 || &header.data[..8] != b"OpusHead" {
        return Err(MediaError::MalformedAudio("missing OpusHead".to_string()));
    }
    let channels = u16::from(header.data[9]);
    if channels != 1 && channels != 2 {
        return Err(MediaError::MalformedAudio(format!(
            "unsupported channel count {channels}"
        )));
    }
    let pre_skip = usize::from(u16::from_le_bytes([header.data[10], header.data[11]]));
    let tags = reader
        .read_packet_expected()
        .map_err(|error| MediaError::MalformedAudio(error.to_string()))?;
    if tags.data.len() < 8 || &tags.data[..8] != b"OpusTags" {
        return Err(MediaError::MalformedAudio("missing OpusTags".to_string()));
    }

    let mut error = OPUS_OK as i32;
    let decoder = Decoder(unsafe {
        opus_decoder_create(PLAYBACK_SAMPLE_RATE as i32, channels.into(), &mut error)
    });
    if decoder.0.is_null() || error != OPUS_OK as i32 {
        return Err(MediaError::Opus(error));
    }

    let mut samples = Vec::new();
    let mut remaining_skip = pre_skip;
    let mut decoded_per_channel = 0_u64;
    while let Some(packet) = reader
        .read_packet()
        .map_err(|error| MediaError::MalformedAudio(error.to_string()))?
    {
        let mut frame = vec![0_f32; 5_760 * usize::from(channels)];
        let frame_count = unsafe {
            opus_decode_float(
                decoder.0,
                packet.data.as_ptr(),
                packet.data.len() as i32,
                frame.as_mut_ptr(),
                5_760,
                0,
            )
        };
        if frame_count < 0 {
            return Err(MediaError::Opus(frame_count));
        }
        let frame_count = frame_count as usize;
        decoded_per_channel += frame_count as u64;
        let skip = remaining_skip.min(frame_count);
        remaining_skip -= skip;
        let mut end = frame_count;
        if packet.last_in_stream() {
            let granule = packet.absgp_page();
            if decoded_per_channel > granule {
                end = end.saturating_sub((decoded_per_channel - granule) as usize);
            }
        }
        if end > skip {
            samples.extend_from_slice(
                &frame[skip * usize::from(channels)..end * usize::from(channels)],
            );
        }
    }
    if samples.is_empty() {
        return Err(MediaError::MalformedAudio("no audio packets".to_string()));
    }
    Ok(DecodedAudio {
        samples,
        channels,
        sample_rate: PLAYBACK_SAMPLE_RATE,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoded_voice_round_trips_duration_and_channel_count() {
        let samples = (0..VOICE_SAMPLE_RATE)
            .map(|index| {
                let phase = index as f32 * 440.0 * std::f32::consts::TAU / VOICE_SAMPLE_RATE as f32;
                (phase.sin() * i16::MAX as f32 * 0.3) as i16
            })
            .collect::<Vec<_>>();

        let encoded = encode_voice(&samples).unwrap();
        let decoded = decode_ogg_opus(encoded).unwrap();

        assert_eq!(decoded.channels, 1);
        assert_eq!(decoded.sample_rate, PLAYBACK_SAMPLE_RATE);
        assert!((decoded.samples.len() as i64 - PLAYBACK_SAMPLE_RATE as i64).abs() < 1_000);
    }
}
