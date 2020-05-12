#[macro_use]
extern crate lazy_static;
extern crate pretty_env_logger;
#[macro_use] extern crate log;
extern crate serde_json;
extern crate serde;
use std::{io::Read, fs, path::PathBuf};
use anvil_region::AnvilChunkProvider;
use clap::{Arg, App, SubCommand};
use std::collections::{BTreeMap, btree_map, VecDeque};
use std::path::Path;
use std::io;
use std::thread;
use std::net::TcpListener;
use std::thread::spawn;
use tungstenite::server::accept;

mod models;

use models::*;

pub fn region_path_from(mut path: PathBuf, x: i32, z: i32) -> PathBuf {
    path.push(format!("r.{}.{}.mca", x, z));
    path
}

fn fname_xz(fname: &str) -> Option<(i32, i32)> {
    let mut fname = fname.split('_');
    let x: i32 = fname.next()?.parse().ok()?;
    let y: i32 = fname.next()?.parse().ok()?;
    Some((x, y))
}

fn get_chunks_fmap<T: AsRef<Path>>(dir: T) -> io::Result<Vec<Vec<PathBuf>>> {
    let mut ret: BTreeMap<i32, BTreeMap<i32, Vec<PathBuf>>> = BTreeMap::new();
    for entry in fs::read_dir(dir)? {
        if let Some(path) = entry.ok().and_then(|e| Some(e.path())) {
            let fname = path.file_stem().unwrap().to_str().unwrap();
            if let Some((x, z)) = fname_xz(fname) {
                match ret.entry(x / 32) {
                    btree_map::Entry::Occupied(mut e) => {
                        let ent = e.get_mut();
                        if let Some(ent) = ent.get_mut(&(z / 32)) {
                            ent.push(path);
                        } else {
                            ent.insert(z / 32, vec![path]);
                        }
                    },
                    btree_map::Entry::Vacant(e) => {
                        let mut new = BTreeMap::new();
                        new.insert(z / 32, vec![path]);
                        e.insert(new);
                    }
                }
            }
        }
    }
    let mut total = vec![];
    for (_x, item) in ret {
        for (_z, item) in item {
            total.push(item);
        }
    }
    Ok(total)
}

struct WorkHandler {
    payload: VecDeque<PathBuf>,
    buffer: String,
}

impl WorkHandler {

    pub fn new(payload: VecDeque<PathBuf>) -> Self {
        Self {
            payload,
            buffer: String::new(),
        }
    }

    pub fn next(&mut self, chunk_provider: &mut AnvilChunkProvider<'_>) -> Option<()> {
        let path = self.payload.pop_back()?;
        let mut file= std::fs::OpenOptions::new().read(true).open(&path).ok()?;
        self.buffer.clear();
        file.read_to_string(&mut self.buffer).ok()?;
        let chunk: PacketChunk = serde_json::from_str(&self.buffer).ok()?;
        let chunk_x = chunk.x;
        let chunk_z = chunk.z;
        let chunk = chunk.into();
        match chunk_provider.save_chunk(chunk_x, chunk_z, chunk) {
            Ok(_) =>{},// info!("{}:{} Patched !", chunk_x, chunk_z),
            Err(e) => error!("{}:{} Failed to patch: {:?}", chunk_x, chunk_z, e),
        }
        Some(())
    }

    pub fn spawn(chunks: Vec<Vec<PathBuf>>, folder: PathBuf) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            let mut provider =  AnvilChunkProvider::new(folder.to_str().unwrap());
            let mut w = WorkHandler::new(chunks.into_iter().flatten().collect());
            while let Some(_) = w.next(&mut provider) {};
        })
    }
}

const NBR_THREAD: usize = 16;

fn run(output: &str, patch: &str) -> std::io::Result<()> {
    let output = PathBuf::from(output);
    let patch = PathBuf::from(patch);
    let mut chunks = get_chunks_fmap(&patch)?;
    let nbr_chunk_per_thread = chunks.len() / NBR_THREAD;
    let mut join = Vec::with_capacity(NBR_THREAD);
    for _ in (0..(NBR_THREAD - 1)).into_iter() {
        if chunks.len() > nbr_chunk_per_thread {
            let h = WorkHandler::spawn(chunks.drain(0 .. nbr_chunk_per_thread).collect(), output.clone());
            join.push(h);
        } else {
            break;
        }
    }
    join.push(WorkHandler::spawn(chunks, output.clone()));
    let mut cptr = join.len();
    for join in join.into_iter() {
        let _ = join.join();
        cptr -= 1;
        info!("{} worker remaning ...", cptr);
    }

    Ok(())
}

fn main() {
    pretty_env_logger::init();
    let matches = App::new("dump-to-map")
        .arg(
            Arg::with_name("output")
                .help("Minecraft region directory to update")
                .short("o")
                .required(true)
                .takes_value(true)
        )
        .subcommand(
            SubCommand::with_name("bulk")
                .about("Copy a bunch of json chunk sections into an existing minecraft world")
                .arg(
                    Arg::with_name("patch")
                        .help("A directory containing JOSN chunk regions")
                        .short("p")
                        .required(true)
                        .takes_value(true)
                )
        )
        .subcommand(
            SubCommand::with_name("listen")
                .about("Listen for chunk sections over a websocket and apply them to an existing minecraft world")
                .arg(
                    Arg::with_name("port")
                        .help("Listen port")
                        .default_value("4242")
                        .short("p")
                        .takes_value(true)
                )
        )
        .get_matches();
    let output = matches.value_of("output").unwrap();
    match  matches.subcommand() {
        ("bulk", Some(matches)) => {
            let patch = matches.value_of("patch").unwrap();
            if let Err(e) = run( output, patch) {
                eprintln!("{}", e);
            }
        },
        ("listen", Some(matches)) => {
            let port = matches.value_of("port").and_then(|port| port.parse().ok()).unwrap_or(4242u32);
            let addr = format!("127.0.0.1:{}", port);
            let server = TcpListener::bind(&addr).unwrap();
            info!("Listening on {} ...", addr);
            for stream in server.incoming() {
                let path = output.clone();
                spawn (move || {
                    let mut websocket = accept(stream.unwrap()).unwrap();
                    loop {
                        let msg = websocket.read_message().unwrap();
                    
                        // We do not want to send back ping/pong messages.
                        if msg.is_binary() || msg.is_text() {
                            websocket.write_message(msg).unwrap();
                        }
                    }
                });
            }
        }
        _ => error!("Unknow subcommand"),
    }
}
