use nbt::CompoundTag;
use serde::{Serialize, Deserialize};

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
    pub heightmaps: HeightMaps,
    pub biomes: Vec<i32>,
    pub chunkData: ChunkData,
    pub blockEntities: Vec<i64>,
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

        self.chunkData.read_data(self.bitMap, false);

        level_compound_tag.insert_compound_tag_vec("Sections", vec![]);
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

const CHUNK_HEIGHT: i32 = 128;
const SECTION_HEIGHT: i32 = 16;


impl ChunkData {
    pub fn read_data(&self, mask: i32, full: bool) {
        let mut buffer = std::io::Cursor::new(&self.data);
    for sectionY in (0..(CHUNK_HEIGHT / SECTION_HEIGHT)).into_iter().filter(|sectionY| mask & (1 << sectionY) != 0) {
            dbg!(sectionY);    
        // byte bitsPerBlock = ReadByte(data);
            
            // Palette palette = ChoosePalette(bitsPerBlock);
            // palette.Read(data);
// 
          ////  A bitmask that contains bitsPerBlock set bits
            // uint individualValueMask = (uint)((1 << bitsPerBlock) - 1);
// 
            // int dataArrayLength = ReadVarInt(data);
            // UInt64[] dataArray = ReadUInt64Array(data, dataArrayLength);
// 
            // ChunkSection section = new ChunkSection();
// 
            // for (int y = 0; y < SECTION_HEIGHT; y++) {
                // for (int z = 0; z < SECTION_WIDTH; z++) {
                    // for (int x = 0; x < SECTION_WIDTH; x++) {
                        // int blockNumber = (((blockY * SECTION_HEIGHT) + blockZ) * SECTION_WIDTH) + blockX;
                        // int startLong = (blockNumber * bitsPerBlock) / 64;
                        // int startOffset = (blockNumber * bitsPerBlock) % 64;
                        // int endLong = ((blockNumber + 1) * bitsPerBlock - 1) / 64;
// 
                        // uint data;
                        // if (startLong == endLong) {
                            // data = (uint)(dataArray[startLong] >> startOffset);
                        // } else {
                            // int endOffset = 64 - startOffset;
                            // data = (uint)(dataArray[startLong] >> startOffset | dataArray[endLong] << endOffset);
                        // }
                        // data &= individualValueMask;
// 
                  ////      data should always be valid for the palette
                  ////      If you're reading a power of 2 minus one (15, 31, 63, 127, etc...) that's out of bounds,
                  ////      you're probably reading light data instead
// 
                        // BlockState state = palette.StateForId(data);
                        // section.SetState(x, y, z, state);
                    // }
                // }
            // }
// 
            // for (int y = 0; y < SECTION_HEIGHT; y++) {
                // for (int z = 0; z < SECTION_WIDTH; z++) {
                    // for (int x = 0; x < SECTION_WIDTH; x += 2) {
                  ////      Note: x += 2 above; we read 2 values along x each time
                        // byte value = ReadByte(data);
// 
                        // section.SetBlockLight(x, y, z, value & 0xF);
                        // section.SetBlockLight(x + 1, y, z, (value >> 4) & 0xF);
                    // }
                // }
            // }
// 
            // if (currentDimension.HasSkylight()) { // IE, current dimension is overworld / 0
                // for (int y = 0; y < SECTION_HEIGHT; y++) {
                    // for (int z = 0; z < SECTION_WIDTH; z++) {
                        // for (int x = 0; x < SECTION_WIDTH; x += 2) {
                        ////    Note: x += 2 above; we read 2 values along x each time
                            // byte value = ReadByte(data);
// 
                            // section.SetSkyLight(x, y, z, value & 0xF);
                            // section.SetSkyLight(x + 1, y, z, (value >> 4) & 0xF);
                        // }
                    // }
                // }
            // }
// 
          //  May replace an existing section or a null one
            // chunk.Sections[SectionY] = section;
    }

    // for (int z = 0; z < SECTION_WIDTH; z++) {
        // for (int x = 0; x < SECTION_WIDTH; x++) {
            // chunk.SetBiome(x, z, ReadInt(data));
        // }
    // }
}    
}