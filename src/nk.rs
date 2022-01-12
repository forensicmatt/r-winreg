use byteorder::{ByteOrder,LittleEndian};
use winstructs::timestamp::{WinTimestamp};
use errors::RegError;
use hive::HBIN_START_OFFSET;
use cell::Cell;
use cell::CellData;
use vk::ValueKey;
use vk::ValueKeyList;
use sk::SecurityKey;
use utils;
use serde::ser;
use serde::Serialize;
use std::io::{Read,Seek};
use std::fmt;

bitflags! {
    pub struct NodeKeyFlags: u16 {
        const KEY_IS_VOLATILE = 0x0001;
        const KEY_HIVE_EXIT = 0x0002;
        const KEY_HIVE_ENTRY = 0x0004;
        const KEY_NO_DELETE = 0x0008;
        const KEY_SYM_LINK = 0x0010;
        const KEY_COMP_NAME = 0x0020;
        const KEY_PREFEF_HANDLE = 0x0040;
        const KEY_VIRT_MIRRORED = 0x0080;
        const KEY_VIRT_TARGET = 0x0100;
        const KEY_VIRTUAL_STORE = 0x0200;
        const KEY_UNKNOWN1 = 0x1000;
        const KEY_UNKNOWN2 = 0x4000;
    }
}
impl fmt::Display for NodeKeyFlags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,"{}",self.bits())
    }
}
impl ser::Serialize for NodeKeyFlags {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: ser::Serializer
    {
        serializer.serialize_str(&format!("{:?}", self))
    }
}

#[derive(Serialize, Debug)]
pub struct NodeKey {
    #[serde(skip_serializing)]
    _offset: u64,
    signature: u16,
    flags: NodeKeyFlags,
    last_written: WinTimestamp,
    access_bits: u32,
    offset_parent_key: u32,
    num_sub_keys: u32, // node keys
    num_volatile_sub_keys: u32,
    offset_sub_key_list: u32, //0xffffffff = empty
    offset_volatile_sub_key_list: u32, //0xffffffff = empty
    num_values: u32, // value keys
    offset_value_list: u32, //0xffffffff = empty
    offset_security_key: u32, //0xffffffff = empty
    offset_class_name: u32, //0xffffffff = empty
    largest_sub_key_name_size: u32,
    largest_sub_key_class_name_size: u32,
    largest_value_name_size: u32,
    largest_value_data_size: u32,
    work_var: u32,
    key_name_size: u16,
    class_name_size: u16,
    // 76 bytes
    key_name: String,
    padding: Vec<u8>,

    value_key_list: Option<Box<ValueKeyList>>,
    sub_key_list: Option<Box<CellData>>,
    security_key: Option<Box<SecurityKey>>
}
impl NodeKey {
    pub fn new(buffer: &[u8], offset: u64) -> Result<NodeKey,RegError> {
        let _offset = offset;
        let signature = LittleEndian::read_u16(&buffer[0..2]);
        let flags = NodeKeyFlags::from_bits_truncate(
            LittleEndian::read_u16(&buffer[2..4])
        );
        let last_written = WinTimestamp::from(
            LittleEndian::read_u64(&buffer[4..12])
        );
        let access_bits = LittleEndian::read_u32(&buffer[12..16]);
        let offset_parent_key = LittleEndian::read_u32(&buffer[16..20]);
        let num_sub_keys = LittleEndian::read_u32(&buffer[20..24]);
        let num_volatile_sub_keys = LittleEndian::read_u32(&buffer[24..28]);
        let offset_sub_key_list = LittleEndian::read_u32(&buffer[28..32]);
        let offset_volatile_sub_key_list = LittleEndian::read_u32(&buffer[32..36]);
        let num_values = LittleEndian::read_u32(&buffer[36..40]);
        let offset_value_list = LittleEndian::read_u32(&buffer[40..44]);
        let offset_security_key = LittleEndian::read_u32(&buffer[44..48]);
        let offset_class_name = LittleEndian::read_u32(&buffer[48..52]);
        let largest_sub_key_name_size = LittleEndian::read_u32(&buffer[52..56]);
        let largest_sub_key_class_name_size = LittleEndian::read_u32(&buffer[56..60]);
        let largest_value_name_size = LittleEndian::read_u32(&buffer[60..64]);
        let largest_value_data_size = LittleEndian::read_u32(&buffer[64..68]);
        let work_var = LittleEndian::read_u32(&buffer[68..72]);
        let key_name_size = LittleEndian::read_u16(&buffer[72..74]);
        let class_name_size = LittleEndian::read_u16(&buffer[74..76]);

        let key_name = match flags.contains(NodeKeyFlags::KEY_COMP_NAME) {
            true => utils::read_ascii(&buffer[76..(76 + key_name_size) as usize])?,
            false => utils::read_utf16(&buffer[76..(76 + key_name_size) as usize])?
        };

        let padding = buffer[(76 + key_name_size) as usize..].to_vec();
        let value_key_list = None;
        let sub_key_list = None;
        let security_key = None;

        Ok(
            NodeKey {
                _offset: _offset,
                signature: signature,
                flags: flags,
                last_written: last_written,
                access_bits: access_bits,
                offset_parent_key: offset_parent_key,
                num_sub_keys: num_sub_keys,
                num_volatile_sub_keys: num_volatile_sub_keys,
                offset_sub_key_list: offset_sub_key_list,
                offset_volatile_sub_key_list: offset_volatile_sub_key_list,
                num_values: num_values,
                offset_value_list: offset_value_list,
                offset_security_key: offset_security_key,
                offset_class_name: offset_class_name,
                largest_sub_key_name_size: largest_sub_key_name_size,
                largest_sub_key_class_name_size: largest_sub_key_class_name_size,
                largest_value_name_size: largest_value_name_size,
                largest_value_data_size: largest_value_data_size,
                work_var: work_var,
                key_name_size: key_name_size,
                class_name_size: class_name_size,
                key_name: key_name,
                padding: padding,
                value_key_list: value_key_list,
                sub_key_list: sub_key_list,
                security_key: security_key
            }
        )
    }

    pub fn key_name(&self)->&String{
        &self.key_name
    }

    pub fn get_next_value<Rs: Read+Seek>(&mut self, reader: &mut Rs)->Result<Option<ValueKey>,RegError>{
        if self.offset_value_list == 4294967295 {
            return Ok(None);
        }

        if self.value_key_list.is_none(){
            let cell = Cell::at_offset(
                reader,
                self.offset_value_list as u64 + HBIN_START_OFFSET
            )?;
            self.value_key_list = Some(
                Box::new(
                    ValueKeyList::new(
                        &cell.data,
                        self.num_values,
                        self.offset_value_list as u64
                    )?
                )
            );
        }

        match self.value_key_list{
            Some(ref mut value_key_list)=>{
                match value_key_list.get_next_value(reader)?{
                    Some(vk) => {
                        Ok(Some(vk))
                    },
                    None => {
                        Ok(None)
                    }
                }
            },
            _ => panic!("Should already have a value")
        }
    }

    pub fn get_next_key<Rs: Read+Seek>(&mut self, reader: &mut Rs)->Result<Option<NodeKey>,RegError>{
        if self.offset_sub_key_list == 4294967295 {
            return Ok(None);
        }

        if self.sub_key_list.is_none(){
            let cell = Cell::at_offset(
                reader,
                self.offset_sub_key_list as u64 + HBIN_START_OFFSET
            )?;
            self.sub_key_list = Some(
                Box::new(cell.get_data()?)
            );
        }

        match self.sub_key_list {
            Some(ref mut cell_data) => {
                match **cell_data {
                    CellData::RootIndex(ref mut ri) => {
                        return Ok(
                            ri.get_next_key(reader)?
                        );
                    },
                    CellData::FastLeaf(ref mut lf) => {
                        return Ok(
                            lf.get_next_key(reader)?
                        );
                    },
                    CellData::HashLeaf(ref mut lh) => {
                        return Ok(
                            lh.get_next_key(reader)?
                        );
                    },
                    CellData::IndexLeaf(ref mut li) => {
                        return Ok(
                            li.get_next_key(reader)?
                        );
                    },
                    ref other => {
                        panic!("Unhandled sub key list: {:?}",other);
                    }
                }
            },
            None => {
                panic!("Subkey List is None.")
            }
        }
    }

    pub fn set_security_key<Rs: Read+Seek>(&mut self, reader: &mut Rs)->Result<(),RegError>{
        if self.offset_security_key == 4294967295 {
            return Ok(());
        }

        let cell = Cell::at_offset(reader, self.offset_security_key as u64 + HBIN_START_OFFSET)?;
        match cell.get_data()?{
            CellData::SecurityKey(sk) => {
                self.security_key = Some(Box::new(sk));
            },
            other => {
                panic!("Unexpected SK type: {:?}",other);
            }
        }

        Ok(())
    }

    pub fn get_security_key(&self)->&Option<Box<SecurityKey>>{
        &self.security_key
    }

    pub fn get_last_written(&self)->&WinTimestamp{
        &self.last_written
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cell::Cell;
    use std::io::Read;
    use std::io::Cursor;
    use std::fs::File;

    #[test]
    fn nodekey() {
        let mut file = File::open(".testdata/NTUSER_4128_144_CELL_NK.DAT").unwrap();
        let mut buffer = Vec::new();

        match file.read_to_end(&mut buffer){
            Err(error)=>panic!("{:?}",error),
            _ => {}
        }

        let cell = match Cell::new(&mut Cursor::new(&buffer),0){
            Ok(cell)=>cell,
            Err(error)=>panic!("{:?}",error)
        };

        let nk = match NodeKey::new(&cell.data,4){
            Ok(nk)=>nk,
            Err(error)=>panic!("{:?}",error)
        };

        assert_eq!(nk.signature, 27502);
        assert_eq!(nk.flags.bits(), 44);
        assert_eq!(nk.last_written.value(), 130269705849853298);
        assert_eq!(nk.access_bits, 2);
        assert_eq!(nk.offset_parent_key, 1928);
        assert_eq!(nk.num_sub_keys, 13);
        assert_eq!(nk.num_volatile_sub_keys, 1);
        assert_eq!(nk.offset_sub_key_list, 2587704);
        assert_eq!(nk.offset_volatile_sub_key_list, 2147484264);
        assert_eq!(nk.num_values, 0);
        assert_eq!(nk.offset_value_list, 4294967295);
        assert_eq!(nk.offset_security_key, 7568);
        assert_eq!(nk.offset_class_name, 4294967295);
        assert_eq!(nk.largest_sub_key_name_size, 42);
        assert_eq!(nk.largest_sub_key_class_name_size, 0);
        assert_eq!(nk.largest_value_name_size, 0);
        assert_eq!(nk.largest_value_data_size, 0);
        assert_eq!(nk.work_var, 3342392);
        assert_eq!(nk.key_name_size, 57);
        assert_eq!(nk.class_name_size, 0);
        assert_eq!(nk.key_name, String::from("CsiTool-CreateHive-{00000000-0000-0000-0000-000000000000}"));

        let known_data: &[u8] = &[
            0x00,0x39,0x00,0x31,0x00,0x45,0x00
        ];
        assert_eq!(&nk.padding[..], known_data);
    }
}
