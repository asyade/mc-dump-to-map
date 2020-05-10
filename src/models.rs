use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketChunk {
    pub x: i32,
    pub z: i32,
    pub groundUp: bool,
    pub bitMap: i64,
    pub heightmaps: serde_json::Value,
    pub biomes: Vec<i64>,
    pub chunkData: ChunkData,
    pub blockEntities: Vec<i64>,
}

impl PacketChunk {
    pub fn copy_into_compound_tag(self, tag: &mut nbt::CompoundTag) {

    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ChunkData {
    Buffer {
        data: Vec<i32>,
    }
}