use std::{collections::HashMap, fs, path::PathBuf};

use anyhow::Result;
use bitbuffer::BitRead;
use tf_demo_parser::{
    demo::{
        header::Header,
        parser::{DemoHandler, RawPacketStream},
    },
    Demo,
};

const FRAME_SIZE: usize = 480;
const SAMPLE_RATE: u16 = 24000;
const TICKRATE: f64 = 200.0 / 3.0;

struct AudioClip {
    pub data: Vec<f32>,
    pub start_tick: u32,
}

impl AudioClip {
    pub fn new(tick: u32) -> Self {
        Self {
            data: Vec::new(),
            start_tick: tick,
        }
    }

    pub fn end_tick_float(&self) -> f64 {
        self.start_tick as f64 + (self.data.len() as f64 / SAMPLE_RATE as f64 * TICKRATE)
    }

    pub fn end_tick(&self) -> u32 {
        self.end_tick_float().ceil() as u32
    }
}

struct Player {
    pub dec: opus::Decoder,
    pub current_frame: u16,

    pub audio: Vec<AudioClip>,
}

impl Player {
    pub fn new() -> Self {
        Self {
            dec: opus::Decoder::new(24000, opus::Channels::Mono).unwrap(),
            current_frame: 0,
            audio: vec![],
        }
    }

    pub fn decode_chunk(&mut self, chunk: &[u8], frame: u16, tick: u32) -> Result<usize> {
        let mut decoded_samples = 0;
        if frame == 0 {
            self.audio.push(AudioClip::new(tick));
            self.current_frame = 0;
        }
        if self.current_frame < frame {
            let diff = frame - self.current_frame;
            for _ in 0..diff {
                let mut out = vec![0.0; FRAME_SIZE];

                decoded_samples += self.dec.decode_float(&[], &mut out, false)?;
                self.audio.last_mut().unwrap().data.append(&mut out);
            }
        }

        let mut output = vec![0.0; FRAME_SIZE];
        decoded_samples += self.dec.decode_float(&chunk, &mut output, false)?;
        self.audio.last_mut().unwrap().data.append(&mut output);
        self.current_frame = frame + 1;
        Ok(decoded_samples)
    }

    pub fn get_continuous_audio(&self) -> Vec<f32> {
        let mut samples = vec![];
        let mut tick = 0.0;

        for clip in &self.audio {
            let diff =
                ((clip.start_tick as f64 - tick) / TICKRATE * SAMPLE_RATE as f64).round() as u32;
            if diff > 0 {
                samples.extend_from_slice(&vec![0.0; diff as usize]);
            }
            samples.extend_from_slice(&clip.data);
            tick = clip.end_tick_float();
        }
        samples.extend_from_slice(&vec![0.0; SAMPLE_RATE as usize]);

        samples
    }
}

fn decode_voice_packet(
    voice_data_message: &tf_demo_parser::demo::message::voice::VoiceDataMessage,
    players: &mut HashMap<u64, Player>,
    tick: u32,
) {
    let mut voice_data = voice_data_message.data.clone();

    let data = voice_data
        .read_bits(voice_data.bits_left() - 4 * 8)
        .unwrap();
    let checksum: u32 = voice_data.read().unwrap();

    let crc = crc::crc32::checksum_ieee(&data.clone().read_bytes(data.bits_left() / 8).unwrap());

    if checksum != crc {
        println!("CRC Mismatch, skipping packet");
        return;
    }

    let mut voice_data = data;

    let steamid: u64 = voice_data.read().unwrap();
    let player = players.entry(steamid).or_insert_with(|| Player::new());
    assert_eq!(voice_data.read::<u8>().unwrap(), 0xB);

    let samplerate: u16 = voice_data.read().unwrap();
    assert_eq!(samplerate, SAMPLE_RATE);

    match voice_data.read::<u8>().unwrap() {
        0x6 => {
            let data_len: u16 = voice_data.read().unwrap();
            let mut codec_data = voice_data.read_bits(data_len as usize * 8).unwrap();

            while codec_data.bits_left() > 0 {
                let chunk_length: i16 = codec_data.read().unwrap();
                if chunk_length == -1 {
                    continue;
                }
                let cur_frame: u16 = codec_data.read().unwrap();
                let chunk_data = codec_data.read_bytes(chunk_length as usize).unwrap();

                player.decode_chunk(&chunk_data, cur_frame, tick).unwrap();
            }
        }
        0x0 => {
            let duration: u16 = voice_data.read().unwrap();
            println!(
                "Tick: {}, SteamID: {steamid}, Silence duration: {duration} ns",
                tick
            );
        }
        t => panic!("Unknown voice data type {t}"),
    }
}

pub fn voice_extract(file: PathBuf, split_clips: bool) {
    let data = fs::read(&file).expect("Couldn't read demo file");

    let demo = Demo::new(&data);

    let mut handler = DemoHandler::default();

    let mut stream = demo.get_stream();
    let header = Header::read(&mut stream).expect("Failed to parse demo header");
    println!(
        "Map: {}\nRecorder: {}\nDuration: {} ({} ticks)",
        header.map, header.nick, header.duration, header.ticks
    );
    handler.handle_header(&header);

    let mut packets = RawPacketStream::new(stream);

    let mut players: HashMap<u64, Player> = HashMap::new();

    let mut voice_packets = 0;

    while let Some(packet) = packets
        .next(&handler.state_handler)
        .expect("Error while parsing demo")
    {
        match &packet {
            tf_demo_parser::demo::packet::Packet::Message(message) => {
                for m in &message.messages {
                    match m {
                        tf_demo_parser::demo::message::Message::NetTick(_) => {
                            print!(
                                "Tick {}, {:.2}%, {voice_packets} voice packets, {} players, {} clips\r",
                                u32::from(message.tick),
                                u32::from(message.tick) as f32 * 100.0 / header.ticks as f32,
                                players.len(),
                                players.values().map(|p|p.audio.len()).sum::<usize>()
                            );
                        }
                        tf_demo_parser::demo::message::Message::VoiceData(voice_data_message) => {
                            decode_voice_packet(
                                voice_data_message,
                                &mut players,
                                u32::from(message.tick),
                            );
                            voice_packets += 1;
                        }
                        _ => (),
                    }
                }
            }
            _ => (),
        }
        handler.handle_packet(packet).unwrap();
    }
    println!();

    let demo_name = file.file_stem().unwrap().to_string_lossy();

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 24000,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    for (playerid, player) in players {
        if split_clips {
            for clip in player.audio {
                let mut writer = hound::WavWriter::create(
                    format!(
                        "{}_{}-{}_{playerid}.wav",
                        &demo_name,
                        clip.start_tick,
                        clip.end_tick()
                    ),
                    spec,
                )
                .unwrap();
                for sample in &clip.data {
                    writer.write_sample(*sample).unwrap();
                }
                writer.finalize().unwrap();
            }
        } else {
            let mut writer =
                hound::WavWriter::create(format!("{}_{playerid}.wav", &demo_name), spec).unwrap();
            for sample in player.get_continuous_audio() {
                writer.write_sample(sample).unwrap();
            }
            writer.finalize().unwrap();
        }
    }
}
