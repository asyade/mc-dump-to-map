use std::collections::HashMap;
use std::{ops, io::{Seek, Read, SeekFrom}, io, collections::BTreeMap};
use nbt::CompoundTag;
use serde::{Serialize, Deserialize};
use byteorder::{BigEndian, ReadBytesExt};
use mc_varint::{VarInt, VarIntRead, VarLongRead};

const VERSION: &str = "1.15.0";
const LIGHT_SIZE: usize = 2048;
const CHUNK_HEIGHT: i32 = 256;
const SECTION_HEIGHT: i32 = 16;
const SECTION_WIDTH: i32 = 16;
const MAX_BITS_PER_BLOCK: u8 = 8;

type BlockId = i64;

lazy_static! {
    static ref PALETTE: GlobalPalette = {
        let mut file = std::fs::OpenOptions::new().read(true).open(std::env::var("PALETTE").expect("PALETTE")).expect("PALETTE File");
        GlobalPalette::parse(file)
    };
}


pub struct GlobalPalette {
    blocks: HashMap<i64, BlockDefinition>
}

impl GlobalPalette {
    /// Please be indulgent
    fn parse<T: Read + Sized>(mut read: T) -> Self {
        let mut blocks = HashMap::new();
        if let serde_json::Value::Object(map) = serde_json::from_reader(read).expect("Wrong palette file") {
            for (name, item) in map {
                let item = item.as_object().unwrap();
                for state in item.get("states").unwrap().as_array().unwrap().into_iter().map(|e| e.as_object().unwrap().get("id").unwrap()).map(|e| e.as_i64().unwrap()) {
                    blocks.insert(state, BlockDefinition{name: name.clone()});
                }
            }
        } else {
            panic!("Wrong palette file");
        }
        GlobalPalette{ blocks }
    }
}

#[derive(Clone, Debug)]
pub struct BlockDefinition {
    name: String,
}

impl ops::Index<BlockId> for GlobalPalette {
    type Output = BlockDefinition;
    fn index(&self, index: BlockId) -> &BlockDefinition {
        &self.blocks[&index]
    }
}

pub trait ReadArrayExt {
    fn read_u8_array(&mut self, size: usize) -> io::Result<Vec<u8>>;
    fn read_i32_array(&mut self, size: usize) -> io::Result<Vec<i32>>;
    fn read_i64_array(&mut self, size: usize) -> io::Result<Vec<i64>>;
    fn read_varint_array(&mut self, size: usize) -> io::Result<Vec<i32>>;
    fn read_varlong_array(&mut self, size: usize) -> io::Result<Vec<i64>>;
}

macro_rules! read_array {
    ($size:expr, $read:expr) => {{
        let mut array = Vec::with_capacity($size);
        for _ in (0..$size).into_iter() {
            array.push($read);
        }
        Ok(array)
    }};
}

impl <T: Read + Sized> ReadArrayExt for T {
    fn read_i32_array(&mut self, size: usize) -> io::Result<Vec<i32>> {
        read_array!(size, self.read_i32::<BigEndian>()?)
    }

    fn read_i64_array(&mut self, size: usize) -> io::Result<Vec<i64>> {
        read_array!(size, self.read_i64::<BigEndian>()?)
    }

    fn read_u8_array(&mut self, size: usize) -> io::Result<Vec<u8>> {
        read_array!(size, self.read_u8()?)
    }

    fn read_varint_array(&mut self, size: usize) -> io::Result<Vec<i32>> {
        read_array!(size, i32::from(self.read_var_int()?))
    }

    fn read_varlong_array(&mut self, size: usize) -> io::Result<Vec<i64>> {
        read_array!(size, i64::from(self.read_var_long()?))
    }
} 

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeightMaps {
    #[serde(rename(deserialize = "type"))]
    pub _type: String,
    pub name: String,
    pub value: HeightMapsValues,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeightMapsValues {
    #[serde(rename(deserialize = "MOTION_BLOCKING"))]
    motion_blocking: Option<HeightMapsValue>,
    #[serde(rename(deserialize = "MOTION_BLOCKING_NO_LEAVES"))]
    motion_blocking_no_leaves: Option<HeightMapsValue>,
    #[serde(rename(deserialize = "OCEAN_FLOOR"))]
    ocean_floor: Option<HeightMapsValue>,
    #[serde(rename(deserialize = "OCEAN_FLOOR_WG"))]
    ocean_floor_wg: Option<HeightMapsValue>,
    #[serde(rename(deserialize = "WORLD_SURFACE"))]
    world_surface: Option<HeightMapsValue>,
    #[serde(rename(deserialize = "WORLD_SURFACE_WG"))]
    world_surface_wg: Option<HeightMapsValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeightMapsValue {
    #[serde(rename(deserialize = "type"))]
    _type: String,
    value: Vec<Vec<i64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketChunk {
    pub x: i32,
    pub z: i32,
    pub groundUp: bool,
    pub bitMap: i32,
    pub heightmaps: serde_json::Value,
    pub biomes: Vec<i32>,
    pub chunkData: ChunkData,
    pub blockEntities: serde_json::Value,
}

impl Into<CompoundTag> for PacketChunk {
    fn into(self) -> CompoundTag {
        let mut chunk_compound_tag = CompoundTag::new();
        let mut level_compound_tag = CompoundTag::new();
        level_compound_tag.insert_str("Status", "full");
        level_compound_tag.insert_i32("zPos", self.z);
        level_compound_tag.insert_i64("LastUpdate", 3);
        level_compound_tag.insert_i32_vec("Biomes", self.biomes);
        level_compound_tag.insert_i64("InhabitedTime", 0);
        level_compound_tag.insert_i32("xPos", self.x);
        let mut heightmaps_compound = CompoundTag::new();
        level_compound_tag.insert_compound_tag("Heightmaps", heightmaps_compound);
        level_compound_tag.insert_compound_tag_vec("TileEntities", vec![]);
        level_compound_tag.insert_compound_tag_vec("Entities", vec![]);
        level_compound_tag.insert_i8("isLightOn", 1);
        level_compound_tag.insert_compound_tag_vec("TileTicks", vec![]);

        let sections = self.chunkData.read_data(self.bitMap, false).expect("Invalide packet").into();

        level_compound_tag.insert_compound_tag_vec("Sections", sections);
        level_compound_tag.insert_compound_tag_vec("PostProcessing", vec![]);
        level_compound_tag.insert_compound_tag("Structures", CompoundTag::new());
        level_compound_tag.insert_compound_tag_vec("LiquidTicks", vec![]);
        chunk_compound_tag.insert_compound_tag("Level", level_compound_tag);
        chunk_compound_tag.insert_i32("DataVersion", 2230);
        chunk_compound_tag
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkData {
    #[serde(rename(deserialize = "type"))]
    _type: String,
    data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct ParsedChunkData {
    chunks: BTreeMap<i32, Chunk>,
}

impl Into<Vec<CompoundTag>> for ParsedChunkData {
    fn into(self) -> Vec<CompoundTag> {
        self.chunks.into_iter().map(|(y, chunk)| {
            let mut tag = CompoundTag::new();
            tag.insert_i64_vec("BlockStates", chunk.block_states_compound());
            tag.insert_compound_tag_vec("Palette", chunk.palette_compound());
            tag.insert_i8_vec("SkyLight", vec![0; 2048]);
            tag.insert_i8("Y", y as i8);
            tag
        }).collect()
    }
}

#[derive(Debug, Clone)]
pub struct Chunk {
    palette: Vec<i32>,
    data: Vec<i64>,
}

impl Chunk {
    pub fn palette_compound(&self) -> Vec<CompoundTag> {
        self.palette.iter().map(|e| {
            let mut tag = CompoundTag::new();
            tag.insert_str("Name", &PALETTE[*e as i64].name);
            tag
        }).collect()
    }

    pub fn block_states_compound(&self) -> Vec<i64> {
        self.data.clone()
    }
}

impl ChunkData {
    pub fn read_data(&self, mask: i32, full: bool) -> io::Result<ParsedChunkData> {
        let mut buffer = std::io::Cursor::new(&self.data);
        let mut result = BTreeMap::new();
        for section_y in (0..(CHUNK_HEIGHT / SECTION_HEIGHT)).into_iter().filter(|section_y| ((mask >> section_y) & 1) != 0).map(|e| e & 0x0F) {
            let nbr_block = buffer.read_i16::<BigEndian>()?;
            let bits_per_block = buffer.read_u8()?;
            let palette = match bits_per_block {
                0..=MAX_BITS_PER_BLOCK => {
                    let palette_len = i32::from(buffer.read_var_int()?);
                    buffer.read_varint_array(palette_len as usize)?
                },
                _ => vec![],
            };
            let data_len = i32::from(buffer.read_var_int()?);
            let mut data = buffer.read_i64_array(data_len as usize).unwrap();
        //    buffer.read_exact(&mut data)?;
            // let mut data = buffer.read_varlong_array(data_len as usize)?;
            info!("mask {} blocks {}, data len {}, bpb {}", mask, nbr_block, data_len, bits_per_block);
            result.insert(section_y, Chunk {
                palette,
                data,
            });
        }
        Ok(ParsedChunkData {chunks: result})
    }
}