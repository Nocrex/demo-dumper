use std::{fs, io::Write, path::PathBuf};

use bitbuffer::BitRead;
use tf_demo_parser::{
    demo::{
        header::Header,
        packet::Packet,
        parser::{DemoHandler, RawPacketStream},
    },
    Demo,
};

pub fn dump_commands(fp_in: PathBuf, fp_out: PathBuf) {
    let out = fs::File::create(fp_out).expect("Couldn't create output file");

    let data = fs::read(&fp_in).expect("Couldn't read demo file");

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

    while let Some(packet) = packets
        .next(&handler.state_handler)
        .expect("Error while parsing demo")
    {
        match &packet {
            Packet::Message(pack) => {
                println!(
                    "{}/{} ({:.0}%)",
                    pack.tick,
                    header.ticks,
                    (u32::from(pack.tick) as f32 / header.ticks as f32) * 100.
                );
            }
            Packet::ConsoleCmd(pack) => {
                writeln!(&out, "{}: {}", pack.tick, pack.command).unwrap();
            }
            _ => (),
        }
        handler.handle_packet(packet).unwrap();
    }
}
