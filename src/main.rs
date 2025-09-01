mod folder_player_dump;
mod input_dump;
mod packet_dump;
mod voice_extract;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Dump all players from the demos in the current folder into a json file
    Players {
        /// File to save players into
        out_file: Option<PathBuf>,
    },
    /// Dump (some) inputs from a demo
    Inputs {
        /// Demo to extract inputs from
        demo: PathBuf,
        /// File to save inputs to
        out_file: Option<PathBuf>,
    },
    /// Extract voicechat from demo
    Voice {
        /// Demo to extract audio from
        demo: PathBuf,
        /// Output seperate audio clips for each voicechat transmission
        #[arg(short)]
        split_clips: bool,
        /// Output folder
        #[arg(short)]
        output_folder: Option<PathBuf>,
    },
    /// Dump packets from demo (not really useful except for developers :P)
    Packets {
        /// Input demo
        demo: PathBuf,
        /// Output file
        outfile: PathBuf,
    },
}

fn main() {
    let args = Args::parse();
    if let Some(cmd) = args.command {
        match cmd {
            Command::Players { out_file } => {
                folder_player_dump::folder_player_dump(out_file);
            }
            Command::Inputs { demo, out_file } => {
                input_dump::dump_inputs(demo, out_file);
            }
            Command::Voice {
                demo: file,
                split_clips,
                output_folder,
            } => {
                voice_extract::voice_extract(
                    file,
                    split_clips,
                    output_folder.unwrap_or_else(|| PathBuf::from(".")),
                );
            }
            Command::Packets {
                demo: file,
                outfile,
            } => {
                packet_dump::packet_dump(file, outfile);
            }
        }
    }
}
