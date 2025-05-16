use std::{collections::HashMap, fs, path::{Path, PathBuf}};

use bitbuffer::BitRead;
use tf_demo_parser::{demo::{header::Header, parser::{DemoHandler, RawPacketStream}}, Demo};

const FRAME_SIZE: usize = 480;

struct Player {
    pub dec: opus::Decoder,
    pub current_frame: u32,
    
    pub audio: Vec<i16>,
}

impl Player{
    pub fn new() -> Self {
        Self {
            dec: opus::Decoder::new(24000, opus::Channels::Mono).unwrap(),
            current_frame: 0,
            audio: Vec::new(),
        }
    }
}

pub fn voice_extract(file: PathBuf){
    let data = fs::read(file).expect("Couldn't read demo file");
    
    let demo = Demo::new(&data);

    let mut handler = DemoHandler::default();

    let mut stream = demo.get_stream();
    let header = Header::read(&mut stream).expect("Failed to parse demo header");
    println!("Map: {}\nRecorder: {}\nDuration: {} ({} ticks)", header.map, header.nick, header.duration, header.ticks);
    handler.handle_header(&header);
    
    let mut packets = RawPacketStream::new(stream);
    
    let mut players: HashMap<u64, Player> = HashMap::new();
    
    while let Some(packet) = packets.next(&handler.state_handler).expect("Error while parsing demo") {
        match &packet {
            tf_demo_parser::demo::packet::Packet::Message(message) => {
                for m in &message.messages {
                    let time = u32::from(message.tick) as f32/66.66666;
                    match m {
                        tf_demo_parser::demo::message::Message::NetTick(net_tick_message) => {
                            print!("Tick {}, {:.2}%\r", u32::from(message.tick), u32::from(message.tick) as f32 * 100.0 / header.ticks as f32);
                        },
                        tf_demo_parser::demo::message::Message::VoiceData(voice_data_message) => {
                            let mut voice_data = voice_data_message.data.clone();
                            let steamid: u64 = voice_data.read().unwrap();
                            let player = players.entry(steamid).or_insert_with(|| Player::new());
                            assert_eq!(voice_data.read::<u8>().unwrap(), 0xB);
                            let samplerate: u16 = voice_data.read().unwrap();
                            match voice_data.read::<u8>().unwrap() {
                                0x6 => {
                                    let data_len: u16 = voice_data.read().unwrap();
                                    let mut codec_data = voice_data.read_bits(data_len as usize * 8).unwrap();
                                    let checksum: u32 = voice_data.read().unwrap();
                                    println!("Tick: {} ({time:.2}), SteamID: {steamid}, Sample Rate: {samplerate}, Len: {data_len}", u32::from(message.tick));
                                    
                                    while codec_data.bits_left() > 0 {
                                        let chunk_length: i16 = codec_data.read().unwrap();
                                        println!("Decoding chunk of length {chunk_length}");
                                        if chunk_length == -1 {
                                            break;
                                        }
                                        let cur_frame: u16 = codec_data.read().unwrap();
                                        let chunk_data = codec_data.read_bytes(chunk_length as usize).unwrap();
                                        let mut output = vec![0; FRAME_SIZE];
                                        player.dec.decode(&chunk_data, &mut output, false).unwrap();
                                        player.audio.append(&mut output);
                                    }
                                },
                                0x0 => {
                                    let duration: u16 = voice_data.read().unwrap();
                                    println!("Tick: {} ({time:.2}), SteamID: {steamid}, Silence duration: {duration} ns", u32::from(message.tick));
                                },
                                t => println!("Unknown voice data type {t}"),
                            }
                        },
                        _ => (),
                    }
                }
            },
            _ => (),
        }
        handler.handle_packet(packet).unwrap();
    }
    
    let spec = hound::WavSpec{
        channels: 1,
        sample_rate: 24000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    for (id, player) in players{
        let mut writer = hound::WavWriter::create(format!("{id}.wav"), spec).unwrap();
        for sample in player.audio{
            writer.write_sample(sample).unwrap();
        }
        writer.finalize().unwrap();
    }
}