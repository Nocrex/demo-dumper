mod folder_player_dump;
mod input_dump;
mod voice_extract;

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use folder_player_dump::folder_player_dump;
use input_dump::dump_inputs;
use voice_extract::voice_extract;

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
        split_clips: Option<bool>,
    }
}

fn main() {
    let args = Args::parse();
    if let Some(cmd) = args.command {
        match cmd {
            Command::Voice {file, split_clips} => {
                voice_extract(file, split_clips.unwrap_or_default());
            }
        }
    } else if let Some(file) = args.input_file {
        dump_inputs(file);
    } else {
        folder_player_dump();
    }
}
