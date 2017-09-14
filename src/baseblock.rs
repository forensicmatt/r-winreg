use rwinstructs::timestamp::{WinTimestamp};
use byteorder::{ReadBytesExt, LittleEndian};
use errors::{RegError};
use utils;
use std::io::Read;
use std::io::{Seek,SeekFrom};

#[derive(Serialize,Debug)]
pub struct BaseBlock {
    #[serde(skip_serializing)]
    _offset: u64,
    pub signature: u32,
    pub primary_seq_num: u32,
    pub secondary_seq_num: u32,
    pub last_written: WinTimestamp,
    pub major_version: u32,
    pub minor_version: u32,
    pub file_type: u32,
    pub file_format: u32,
    pub root_cell_offset: u32,
    pub hive_bins_data_size: u32,
    pub clustering_factor: u32,
    pub file_name: String, //offset: 48;64
    #[serde(skip_serializing)]
    pub reserved1: utils::ByteArray, //offset: 112;396
    pub checksum: u32,
    #[serde(skip_serializing)]
    pub reserved2:  utils::ByteArray, //offset: 512;3576
    pub boot_type: u32,
    pub boot_recover: u32
}
impl BaseBlock {
    pub fn new<Rs: Read+Seek>(mut reader: Rs) -> Result<BaseBlock,RegError> {
        let _offset = reader.seek(SeekFrom::Current(0))?;
        let signature = reader.read_u32::<LittleEndian>()?;

        if signature != 1718052210 {
            return Err(
                RegError::validation_error(
                    format!("Invalid signature {} in BaseBlock at offset {}.",signature,_offset)
                )
            )
        }

        let primary_seq_num = reader.read_u32::<LittleEndian>()?;
        let secondary_seq_num = reader.read_u32::<LittleEndian>()?;
        let last_written = WinTimestamp(
            reader.read_u64::<LittleEndian>()?
        );
        let major_version = reader.read_u32::<LittleEndian>()?;
        let minor_version = reader.read_u32::<LittleEndian>()?;
        let file_type = reader.read_u32::<LittleEndian>()?;
        let file_format = reader.read_u32::<LittleEndian>()?;
        let root_cell_offset = reader.read_u32::<LittleEndian>()?;
        let hive_bins_data_size = reader.read_u32::<LittleEndian>()?;
        let clustering_factor = reader.read_u32::<LittleEndian>()?;

        let mut name_buffer = vec![0; 64];
        reader.read_exact(name_buffer.as_mut_slice())?;
        let file_name = utils::read_string_u16_till_null(
            name_buffer.as_slice()
        )?;

        let mut reserved1_buff = vec![0; 396];
        reader.read_exact(reserved1_buff.as_mut_slice())?;
        let reserved1 = utils::ByteArray(reserved1_buff);

        let checksum = reader.read_u32::<LittleEndian>()?;

        let mut reserved2_buff = vec![0; 3576];
        reader.read_exact(reserved2_buff.as_mut_slice())?;
        let reserved2 = utils::ByteArray(reserved2_buff);

        let boot_type = reader.read_u32::<LittleEndian>()?;
        let boot_recover = reader.read_u32::<LittleEndian>()?;

        Ok(
            BaseBlock {
                _offset: _offset,
                signature: signature,
                primary_seq_num: primary_seq_num,
                secondary_seq_num: secondary_seq_num,
                last_written: last_written,
                major_version: major_version,
                minor_version: minor_version,
                file_type: file_type,
                file_format: file_format,
                root_cell_offset: root_cell_offset,
                hive_bins_data_size: hive_bins_data_size,
                clustering_factor: clustering_factor,
                file_name: file_name,
                reserved1: reserved1,
                checksum: checksum,
                reserved2:  reserved2,
                boot_type: boot_type,
                boot_recover: boot_recover
            }
        )
    }

    pub fn get_root_offset(&self)->u32{
        self.root_cell_offset
    }

    pub fn hbin_size(&self)->u32{
        self.hive_bins_data_size
    }
}
