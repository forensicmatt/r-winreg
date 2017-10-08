use errors::{RegError};
use rwinstructs::security::{SecurityDescriptor};
use byteorder::{ReadBytesExt, LittleEndian};
use std::io::Read;
use std::io::{Seek,SeekFrom};
use std::io::{Cursor};

// sk
#[derive(Serialize, Debug, Clone)]
pub struct SecurityKey {
    #[serde(skip_serializing)]
    _offset: u64,
    #[serde(skip_serializing)]
    pub unknown1: u16,
    #[serde(skip_serializing)]
    pub previous_sec_key_offset: u32,
    #[serde(skip_serializing)]
    pub next_sec_key_offset: u32,
    #[serde(skip_serializing)]
    pub reference_count: u32,
    #[serde(skip_serializing)]
    pub descriptor_size: u32,
    pub descriptor: SecurityDescriptor
}
impl SecurityKey {
    pub fn new<Rs: Read+Seek>(mut reader: Rs, offset: u64) -> Result<SecurityKey,RegError> {
        let _offset = offset;

        let unknown1 = reader.read_u16::<LittleEndian>()?;
        let previous_sec_key_offset = reader.read_u32::<LittleEndian>()?;
        let next_sec_key_offset = reader.read_u32::<LittleEndian>()?;
        let reference_count = reader.read_u32::<LittleEndian>()?;
        let descriptor_size = reader.read_u32::<LittleEndian>()?;

        let mut descriptor_buffer = vec![0; descriptor_size as usize];
        reader.read_exact(descriptor_buffer.as_mut_slice())?;

        let descriptor = SecurityDescriptor::new(
            Cursor::new(descriptor_buffer)
        )?;

        let security_key = SecurityKey {
            _offset: _offset,
            unknown1: unknown1,
            previous_sec_key_offset: previous_sec_key_offset,
            next_sec_key_offset: next_sec_key_offset,
            reference_count: reference_count,
            descriptor_size: descriptor_size,
            descriptor: descriptor
        };

        Ok(security_key)
    }

    pub fn get_descriptor(&self)->SecurityDescriptor{
        self.descriptor.clone()
    }
}
