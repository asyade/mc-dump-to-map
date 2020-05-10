use nbt::CompoundTag;
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

impl Into<CompoundTag> for PacketChunk {
    fn into(self) -> CompoundTag {
        let mut chunk_compound_tag = CompoundTag::new();
        let mut level_compound_tag = CompoundTag::new();
        level_compound_tag.insert_i32("xPos", self.x);
        level_compound_tag.insert_i32("zPos", self.z);
        chunk_compound_tag.insert_compound_tag("Level", level_compound_tag);
        chunk_compound_tag
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ChunkData {
    Buffer {
        data: Vec<i32>,
    }
}