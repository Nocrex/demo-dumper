use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::sync::{Mutex, Arc};
use anyhow::Result;
use std::thread;

use glob::glob;
use tf_demo_parser::{Demo, DemoParser};

struct ParseResult {
    pub demo: PathBuf,
    pub players: Result<PlayerMap>,
}
type PlayerMap = HashMap<String, String>;
type Output = HashMap<PathBuf, PlayerMap>;

fn main() {
    let jobs = Arc::new(Mutex::new(Vec::new()));
    let (res_tx, res_rx) = channel();
    
    let threadcount = thread::available_parallelism().unwrap();

    for path in glob("*.dem").unwrap(){
        jobs.lock().unwrap().push(path);
    }
    
    let count = jobs.lock().unwrap().len();

    println!("Parsing {count} demos on {threadcount} threads");
    
    for _i in 0..threadcount.into() {
        let res_tx = res_tx.clone();
        let jobs = jobs.clone();
        thread::spawn(move || {
            while let Some(Ok(demo)) = {
                let mut guard = jobs.lock().unwrap();
                guard.pop()
            } {
                res_tx.send(ParseResult{players: parse_demo(&demo), demo: demo}).unwrap();
            }
        });
    }
    
    let mut processed = 0;
    
    let mut out: Output = HashMap::new();
    
    while processed < count {
        let result =  res_rx.recv().unwrap();
        processed += 1;
        let demo = result.demo;
        match result.players {
            Ok(p) => {
                println!("Processed demo {demo:?} ({processed}/{count} | {:.2}%)", processed as f64*100.0/count as f64);
                out.insert(demo, p);
            },
            Err(e) => {
                eprintln!("Failed to parse demo {demo:?}: {e:?}");
            }
        }
    }
    save(&out);
}

fn save(out: &Output){
    println!("Writing data to demo_dump.json");
    fs::write(Path::new("demo_dump.json"), serde_json::to_string_pretty(out).unwrap()).unwrap();
}

fn parse_demo(path: &Path) -> Result<PlayerMap> {
    let bytes = fs::read(path)?;
    let demo = Demo::new(&bytes);
    let parser = DemoParser::new(demo.get_stream());
    let (_, result) = parser.parse()?;
    
    let mut players = HashMap::new();

    for user in result.users.values() {
        players.insert( user.name.clone(), user.steam_id.clone());
    }
    Ok(players)
}