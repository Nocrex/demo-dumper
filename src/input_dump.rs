use std::{collections::HashMap, fs, path::PathBuf};

use bitbuffer::BitRead;
use regex::Regex;
use tf_demo_parser::{
    demo::{
        header::Header,
        packet::{consolecmd::ConsoleCmdPacket, Packet},
        parser::{DemoHandler, RawPacketStream},
    },
    Demo,
};

static KEYMAP_STR: &str = include_str!("data/source keymap.txt");

pub fn dump_inputs(demo_file: PathBuf, out_file: Option<PathBuf>) {
    let mut out = String::new();
    let mapping_re = Regex::new(r"(\d+), ([^\)]+)").unwrap();
    let keymap: HashMap<&str, &str> = mapping_re
        .captures_iter(KEYMAP_STR)
        .map(|cap| (cap.get(1).unwrap().as_str(), cap.get(2).unwrap().as_str()))
        .collect();

    let data = fs::read(&demo_file).expect("Couldn't read demo file");

    let demo = Demo::new(&data);

    let mut handler = DemoHandler::default();

    let mut stream = demo.get_stream();
    let header = Header::read(&mut stream).expect("Failed to parse demo header");
    out += &format!(
        "Map: {}\nRecorder: {}\nDuration: {} ({} ticks)\n\nInputs:\n",
        header.map, header.nick, header.duration, header.ticks
    );
    handler.handle_header(&header);

    let mut packets = RawPacketStream::new(stream);

    let num_re = Regex::new(r" \d+\s*$").unwrap();

    while let Some(packet) = packets
        .next(&handler.state_handler)
        .expect("Error while parsing demo")
    {
        match &packet {
            Packet::ConsoleCmd(ConsoleCmdPacket { tick, command }) => {
                if !command.starts_with("+") && !command.starts_with("-") {
                    continue;
                }
                if let Some(mat) = num_re.find(&command) {
                    let keycode = mat.as_str().trim();
                    if let Some(key) = keymap.get(keycode) {
                        out += &format!("{tick}: {command:20} -> {}{key}\n", &command[0..1]);
                    }
                }
            }
            _ => (),
        }
        handler.handle_packet(packet).unwrap();
    }
    print!("{}", out);
    let path = out_file.unwrap_or_else(|| {
        PathBuf::from(format!(
            "inputs-{}.txt",
            demo_file.file_name().unwrap_or_default().to_string_lossy()
        ))
    });
    fs::write(&path, out).unwrap();
}
