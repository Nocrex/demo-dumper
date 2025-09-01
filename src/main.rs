mod folder_player_dump;
mod input_dump;
mod packet_dump;
mod voice_extract;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// File to extract inputs from
    pub input_file: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Extract voicechat from demo
    Voice {
        /// File to extract audio from
        file: PathBuf,
        /// Output seperate audio clips for each voicechat transmission
        #[arg(short)]
        split_clips: bool,
        /// Output folder
        #[arg(short)]
        output_folder: Option<PathBuf>,
    },
    /// Dump packets from demo
    Packets {
        /// Input file
        file: PathBuf,
        /// Output file
        outfile: PathBuf,
    },
}

fn main() {
    let args = Args::parse();
    if let Some(cmd) = args.command {
        match cmd {
            Command::Voice { file, split_clips , output_folder} => {
                voice_extract::voice_extract(file, split_clips, output_folder.unwrap_or_else(||PathBuf::from(".")));
            }
            Command::Packets { file, outfile } => {
                packet_dump::packet_dump(file, outfile);
            }
        }
    } else if let Some(file) = args.input_file {
        input_dump::dump_inputs(file);
    } else {
        folder_player_dump::folder_player_dump();
    }
}
