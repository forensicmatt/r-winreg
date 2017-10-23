use rwinstructs::timestamp::{WinTimestamp};
use byteorder::{ByteOrder,LittleEndian};
use errors::RegError;
use utils;

#[derive(Serialize,Debug)]
pub struct BaseBlock {
    #[serde(skip_serializing)]
    _offset: u64,
    signature: u32,
    primary_seq_num: u32,
    secondary_seq_num: u32,
    last_written: Box<WinTimestamp>,
    major_version: u32,
    minor_version: u32,
    file_type: u32,
    file_format: u32,
    root_cell_offset: u32,
    hive_bins_data_size: u32,
    clustering_factor: u32,
    file_name: String, //offset: 48;64
    #[serde(skip_serializing)]
    reserved1: Vec<u8>, //offset: 112;396
    checksum: u32,
    #[serde(skip_serializing)]
    reserved2:  Vec<u8>, //offset: 512;3576
    boot_type: u32,
    boot_recover: u32
}

impl BaseBlock {
    pub fn new(buffer: &[u8;4096], offset: u64)->Result<BaseBlock,RegError> {
        let _offset = offset;
        let signature = LittleEndian::read_u32(&buffer[0..4]);
        let primary_seq_num = LittleEndian::read_u32(&buffer[4..8]);
        let secondary_seq_num = LittleEndian::read_u32(&buffer[8..12]);
        let last_written = Box::new(
            WinTimestamp(
                LittleEndian::read_u64(&buffer[12..20])
            )
        );
        let major_version = LittleEndian::read_u32(&buffer[20..24]);
        let minor_version = LittleEndian::read_u32(&buffer[24..28]);
        let file_type = LittleEndian::read_u32(&buffer[28..32]);
        let file_format = LittleEndian::read_u32(&buffer[32..36]);
        let root_cell_offset = LittleEndian::read_u32(&buffer[36..40]);
        let hive_bins_data_size = LittleEndian::read_u32(&buffer[40..44]);
        let clustering_factor = LittleEndian::read_u32(&buffer[44..48]);

        // 64 bytes: 48..112
        let file_name = utils::read_string_u16_till_null(
            &buffer[48..112]
        )?;

        // 396 bytes: 112..508
        let reserved1 = buffer[112..508].to_vec();

        let checksum = LittleEndian::read_u32(&buffer[508..512]);

        // 3576 bytes: 512..4088
        let reserved2 = buffer[512..4088].to_vec();

        let boot_type = LittleEndian::read_u32(&buffer[4088..4092]);
        let boot_recover = LittleEndian::read_u32(&buffer[4092..4096]);

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

    pub fn root_cell_offset(&self)->u32{
        self.root_cell_offset
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use std::fs::File;

    #[test]
    fn baseblock() {
        let mut file = File::open(".testdata/NTUSER_0_4096_BASE_BLOCK.DAT").unwrap();
        let mut buffer_baseblock = [0; 4096];

        match file.read_exact(&mut buffer_baseblock){
            Err(error)=>panic!("{:?}",error),
            _ => {}
        }

        let baseblock = match BaseBlock::new(&buffer_baseblock,0){
            Ok(baseblock)=>baseblock,
            Err(error)=>panic!("{:?}",error)
        };

        assert_eq!(baseblock.signature, 1718052210);
        assert_eq!(baseblock.primary_seq_num, 2810);
        assert_eq!(baseblock.secondary_seq_num, 2809);
        assert_eq!(baseblock.last_written.0, 130216723045201708);
        assert_eq!(baseblock.major_version, 1);
        assert_eq!(baseblock.minor_version, 3);
        assert_eq!(baseblock.file_type, 0);
        assert_eq!(baseblock.file_format, 1);
        assert_eq!(baseblock.root_cell_offset, 32);
        assert_eq!(baseblock.hive_bins_data_size, 3563520);
        assert_eq!(baseblock.clustering_factor, 1);
        assert_eq!(baseblock.checksum, 1151707345);
        assert_eq!(baseblock.boot_type, 0);
        assert_eq!(baseblock.boot_recover, 0);
    }
}
