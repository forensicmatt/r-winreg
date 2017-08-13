use rwinstructs::timestamp::{WinTimestamp};
use byteorder::{ReadBytesExt, LittleEndian};
use utils::to_hex_string;
use errors::{RegError};
use serde::{ser};
use std::io::Read;
use std::io::{Seek,SeekFrom};
use std::fmt;

#[derive(Serialize,Debug)]
pub struct HiveBinHeader {
    #[serde(skip_serializing)]
    _offset: u64,
    pub signature: u32,
    pub hb_offset: u32,
    pub size: u32,
    pub reserved1: u64,
    pub timestamp: WinTimestamp,
    pub spare: u32
}

impl HiveBinHeader {
    pub fn new<Rs: Read+Seek>(mut reader: Rs) -> Result<HiveBinHeader,RegError> {
        let _offset = reader.seek(SeekFrom::Current(0))?;
        let signature = reader.read_u32::<LittleEndian>()?;

        if signature != 1852400232 {
            return Err(
                RegError::validation_error(
                    format!(
                        "Invalid signature {} in HiveBinHeader at offset {}.",
                        signature,_offset
                    )
                )
            )
        }

        let hb_offset = reader.read_u32::<LittleEndian>()?;
        let size = reader.read_u32::<LittleEndian>()?;
        let reserved1 = reader.read_u64::<LittleEndian>()?;
        let timestamp = WinTimestamp(
            reader.read_u64::<LittleEndian>()?
        );
        let spare = reader.read_u32::<LittleEndian>()?;

        Ok(
            HiveBinHeader {
                _offset: _offset,
                signature: signature,
                hb_offset: hb_offset,
                size: size,
                reserved1: reserved1,
                timestamp: timestamp,
                spare: spare
            }
        )
    }
}

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum CellData{
    UnhandledCellData(UnhandledCellData)
}

pub struct Cell{
    _offset: u64,
    pub size: i32,
    pub signature: u16,
    pub data: CellData
}

pub struct UnhandledCellData(pub Vec<u8>);
impl fmt::Debug for UnhandledCellData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:?}",
            to_hex_string(&self.0),
        )
    }
}
impl ser::Serialize for UnhandledCellData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: ser::Serializer
    {
        serializer.serialize_str(
            &format!("{}", to_hex_string(&self.0))
        )
    }
}
