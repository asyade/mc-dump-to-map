#[macro_use]
extern crate lazy_static;
extern crate pretty_env_logger;
#[macro_use] extern crate log;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate serde;
use std::env;
use std::{io::Read, fs, fs::read_dir, path::PathBuf};
use std::process::exit;
use anvil_region::AnvilChunkProvider;
use clap::{Arg, App, SubCommand};
use std::collections::{BTreeMap, btree_map, VecDeque};
use std::path::Path;
use std::io;
use std::thread;

mod utils;
mod models;

use models::*;
use utils::copy;

pub struct World {
    pub path: PathBuf,
}

impl World {
    pub fn new(path: PathBuf) -> Self {
        if !path.exists() {
            let _ = std::fs::create_dir_all(&path);
        }
        Self {path}
    }

    pub fn region_path(&self, x: i32, z: i32) -> PathBuf {
        let mut path = self.path.clone();
        path.push(format!("r.{}.{}.mca", x, z));
        path
    }

    pub fn region_path_from(mut path: PathBuf, x: i32, z: i32) -> PathBuf {
        path.push(format!("r.{}.{}.mca", x, z));
        path
    }


    /// Copy the current world to another path and return the new world
    pub fn dup(self, path: PathBuf) -> std::io::Result<World> {
        copy(self.path, &path)?;
        Ok(Self {
            path
        })
    }

    /// Fill a rectangle of regions with `source_region`
    pub fn fill_copy(&self, xmin: i32, zmin: i32, xmax: i32, zmax: i32, source_region: PathBuf) -> std::io::Result<()> {
        for x in (xmin..xmax).into_iter() {
            for z in (zmin..zmax).into_iter() {
                let target = self.region_path(x, z);
                let _ = std::fs::remove_file(&target);
                let _ = std::fs::copy(&source_region, target);
            }
        }
        Ok(())
    }

    pub fn dup_region(&self, source: (i32, i32), dest: (i32, i32)) -> std::io::Result<()> {
        // let _ = std::fs::remove_file(self.region_path(dest.0, dest.1));
        // std::fs::copy(self.region_path(source.0, source.1), self.region_path(dest.0, dest.1))?;
        Ok(())
    }
}


fn fname_xz(fname: &str) -> Option<(i32, i32)> {
    let mut fname = fname.split('_');
    let x: i32 = fname.next()?.parse().ok()?;
    let y: i32 = fname.next()?.parse().ok()?;
    Some((x, y))
}

fn get_chunks_fmap<T: AsRef<Path>>(dir: T) -> io::Result<Vec<Vec<PathBuf>>> {
    let mut ret: BTreeMap<i32, BTreeMap<i32, Vec<PathBuf>>> = BTreeMap::new();
    let mut total: usize = 0;
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
    for (x, item) in ret {
        for (z, item) in item {
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
        let world = World::new(path);
        let src = world.region_path(0, 0);
        let mut file= std::fs::OpenOptions::new().read(true).open(&world.path).ok()?;
        self.buffer.clear();
        file.read_to_string(&mut self.buffer).ok()?;
        let chunk: PacketChunk = serde_json::from_str(&self.buffer).ok()?;
        let region_path = world.region_path(chunk.x >> 5, chunk.z >> 5);
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
    for id in (0..(NBR_THREAD - 1)).into_iter() {
        if (chunks.len() > nbr_chunk_per_thread) {
            let h = WorkHandler::spawn(chunks.drain(0 .. nbr_chunk_per_thread).collect(), output.clone());
            join.push(h);
        } else {
            break;
        }
    }
    join.push(WorkHandler::spawn(chunks, output.clone()));
    let mut cptr = join.len();
    for join in join.into_iter() {
        join.join();
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
                        .short("p")
                        .required(true)
                        .takes_value(true)
                )
        )
        .get_matches();
    let output = matches.value_of("output").unwrap();
    let patch = matches.value_of("patch").unwrap();
    if let Err(e) = run( output, patch) {
        eprintln!("{}", e);
    }
}
