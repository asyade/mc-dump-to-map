use actix::prelude::*;
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

struct FindActor {
    target: Vec<String>,
    path: String,
}

impl FindActor {
    fn new(target: Vec<String>, path: String) -> Self {
        Self {
            target,
            path,
        }
    }
}

#[derive(Message, Debug)]
#[rtype(result = "()")]
struct FindRequest(RegionFile);

impl Handler<FindRequest> for FindActor {
    type Result = ();

    fn handle(&mut self, msg: FindRequest, ctx: &mut SyncContext<Self>) {
        let provider =  AnvilChunkProvider::new(&self.path);
        let region = msg.0;
        for cx in (0..32).into_iter().map(|cx| cx + (region.x * 32)) {
            for cz in (0..32).into_iter().map(|cz| cz + (region.z * 32)) {
                if let Ok(chunk) = provider.load_chunk(cx, cz) {
                    find(cx * 16, cz * 16,chunk, &self.target[..]);
                }
            }
        }
    }
}

impl actix::Actor for FindActor {
    type Context = SyncContext<Self>;
}



#[actix_rt::main]
async fn main() {
    if std::env::var("PALETTE").is_err() {
        std::env::set_var("PALETTE", "./res/blocks-1.15.0.json");
    }
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
                        .long("patch")
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
                        .long("port")
                        .takes_value(true)
                )
        )
        .subcommand(
            SubCommand::with_name("find")
                .about("Find coords of a block")
                .arg(
                    Arg::with_name("block")
                        .help("Block name")
                        .default_value("minecraft:diamond_block")
                        .short("b")
                        .long("block")
                        .multiple(true)
                        .takes_value(true)
                )
                .arg(
                    Arg::with_name("list")
                        .long("list")
                        .help("List available blocks")
                        .short("l")
                )
                .arg(
                    Arg::with_name("force")
                        .help("Rescue from crash on corupted file but extremly slow")
                        .short("f")
                        .long("force")
                )
        )
        .get_matches();
    let output = matches.value_of("output").unwrap().to_string();
    match  matches.subcommand() {
        ("bulk", Some(matches)) => {
            let patch = matches.value_of("patch").unwrap();
            if let Err(e) = run(&output, patch) {
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
                    let provider = AnvilChunkProvider::new(&path);
                    let mut websocket = accept(stream.unwrap()).unwrap();
                    loop {
                        if let Ok(msg) = websocket.read_message() {
                            if msg.is_binary() || msg.is_text() {
                                if let Ok(chunk) = serde_json::from_slice(&msg.into_data()) {
                                    let chunk: PacketChunk = chunk;
                                    let chunk_x = chunk.x;
                                    let chunk_z = chunk.z;
                                    let chunk = chunk.into();
                                    match provider.save_chunk(chunk_x, chunk_z, chunk) {
                                        Ok(_) => info!("{}:{} Patched !", chunk_x, chunk_z),
                                        Err(e) => error!("{}:{} Failed to patch: {:?}", chunk_x, chunk_z, e),
                                    }    
                                } else {
                                    warn!("Invalide packet received !");
                                }
                            } else {
                                info!("Connection closed !");
                                break
                            }
                        }
                    }
                });
            }
        },
        ("find", Some(matches)) => {
            if matches.is_present("list") {
                for item in models::PALETTE.blocks.values() {
                    println!("{}", item.name);
                }
            } else {
                let oc = output.clone();
                let target: Vec<String> = matches.values_of("block").expect("Block to find").into_iter().map(|e| e.to_string()).collect();
                let mut handles = vec![];
                let addr = SyncArbiter::start(16, move || {
                    let output = output.clone();
                    let target =  target.clone();
                    FindActor::new(target, output)
                });
                for dir in fs::read_dir(&oc).expect("Wrong map directory").filter(|e| e.is_ok()).map(|e| e.unwrap()) {
                    if let Some(region) = RegionFile::new(dir.path()) {
                       handles.push( addr.send(FindRequest(region)));
                    }
                }
                for handle in handles {
                    let _ = handle.await;
                }
            }
        },
        _ => error!("Unknow subcommand"),
    }
}

fn find(x: i32, z: i32, tag: nbt::CompoundTag, target: &[String]) -> Option<()> {
    fn find_in_section(x: i32, z: i32, section: &nbt::CompoundTag, target: &[String]) -> Option<()> {
        let palette = section.get_compound_tag_vec("Palette").ok()?;
        let mut found = false;
        for item in palette {
            for target in target {
            if item.get_str("Name").ok() == Some(target) {
                found = true;
            }
        }
        }
        if found {
            info!("{} 0 {}", x, z);
        }
        Some(())
    }

    let level = tag.get_compound_tag("Level").ok()?;
    let sections = level.get_compound_tag_vec("Sections").ok()?;
    for section in sections {
        let _ = find_in_section( x, z,section, target);
    }
    Some(())
}

#[derive(Debug)]
struct RegionFile {
    x: i32,
    z: i32,
    path: PathBuf,
}

impl RegionFile {
    fn new(path: PathBuf) -> Option<RegionFile> {
        let s: &str = path.file_stem()?.to_str()?;
        let mut sp = s.split('.');
        sp.next()?;
        let x = sp.next()?.parse().ok()?;
        let z = sp.next()?.parse().ok()?;
        Some(RegionFile {
            x,
            z,
            path,
        })
    }
}
