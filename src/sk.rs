use winstructs::security::{SecurityDescriptor};
use winstructs::err::Error;
use byteorder::{ByteOrder,LittleEndian};
use errors::{RegError};
use std::io::Cursor;
use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct SecurityKey {
    _offset: u64,
    signature: u16,
    unknown1: u16,
    previous_sec_key_offset: u32,
    next_sec_key_offset: u32,
    reference_count: u32,
    descriptor_size: u32,
    descriptor: SecurityDescriptor
}

impl SecurityKey {
    pub fn new(buffer: &[u8], offset: u64) -> Result<SecurityKey,RegError> {
        let signature = LittleEndian::read_u16(&buffer[0..2]);
        let unknown1 = LittleEndian::read_u16(&buffer[2..4]);
        let previous_sec_key_offset = LittleEndian::read_u32(&buffer[4..8]);
        let next_sec_key_offset = LittleEndian::read_u32(&buffer[8..12]);
        let reference_count = LittleEndian::read_u32(&buffer[12..16]);
        let descriptor_size = LittleEndian::read_u32(&buffer[16..20]);

        let mut cursor = Cursor::new(&buffer[20..]);
        let descriptor = match SecurityDescriptor::from_stream(
            &mut cursor
        ) {
            Ok(descriptor) => descriptor,
            Err(why) => {
                return Err(RegError::from(why));
            }
        };

        Ok(
            SecurityKey {
                _offset: offset,
                signature: signature,
                unknown1: unknown1,
                previous_sec_key_offset: previous_sec_key_offset,
                next_sec_key_offset: next_sec_key_offset,
                reference_count: reference_count,
                descriptor_size: descriptor_size,
                descriptor: descriptor
            }
        )
    }

    pub fn get_descriptor(&self)->&SecurityDescriptor {
        &self.descriptor
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cell::Cell;
    use std::io::Cursor;
    use std::io::Read;
    use std::fs::File;

    #[test]
    fn securitykey() {
        let mut file = File::open(".testdata/NTUSER_4768_184_CELL_SK.DAT").unwrap();
        let mut buffer = Vec::new();

        match file.read_to_end(&mut buffer){
            Err(error)=>panic!("{:?}",error),
            _ => {}
        }

        let cell = match Cell::new(&mut Cursor::new(&buffer),0){
            Ok(cell)=>cell,
            Err(error)=>panic!("{:?}",error)
        };

        let sk = match SecurityKey::new(&cell.data,4){
            Ok(sk)=>sk,
            Err(error)=>panic!("{:?}",error)
        };

        assert_eq!(sk.signature, 27507);
        assert_eq!(sk.unknown1, 0);
        assert_eq!(sk.previous_sec_key_offset, 1876000);
        assert_eq!(sk.next_sec_key_offset, 7568);
        assert_eq!(sk.reference_count, 84);
        assert_eq!(sk.descriptor_size, 160);
    }
}
