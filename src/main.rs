mod folder_player_dump;
mod voice_extract;

use std::path::PathBuf;

use clap::Parser;
use folder_player_dump::folder_player_dump;
use voice_extract::voice_extract;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Args{
    /// Extract voice chat audio from demo
    #[arg(short, long, value_name="DEMO FILE")]
    pub voice: Option<PathBuf>,
}

fn main() {
    let args = Args::parse();
    if let Some(file) = args.voice{
        voice_extract(file);
    }else{
        folder_player_dump();
    }
}