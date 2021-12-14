use byteorder::{ByteOrder,LittleEndian,BigEndian};
use errors::RegError;
use hive::HBIN_START_OFFSET;
use cell::Cell;
use cell::CellData;
use utils;
use serde::ser::{SerializeStruct};
use serde::ser;
use std::fmt;
use std::io::Read;
use std::io::Seek;
use std::mem::transmute;
use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum Data {
    None,
    String(String),
    Int32(i32),
}

#[derive(Serialize, Debug)]
pub struct ValueKeyList{
    #[serde(skip_serializing)]
    _offset: u64,
    value_offsets: Vec<u32>,
    next_index: usize
}
impl ValueKeyList{
    pub fn new(buffer: &[u8], value_count: u32, offset: u64) -> Result<ValueKeyList,RegError> {
        let mut value_offsets: Vec<u32> = Vec::new();
        let next_index: usize = 0;

        for i in 0..value_count {
            let o = (i*4) as usize;
            if o+4 > buffer.len(){
                panic!("error: value_count: {}; buffer: {:?}",value_count,buffer)
            }
            let offset = LittleEndian::read_u32(&buffer[o..o+4]);
            value_offsets.push(offset);
        }

        Ok(
            ValueKeyList{
                _offset: offset,
                value_offsets: value_offsets,
                next_index: next_index
            }
        )
    }

    pub fn get_next_value<Rs: Read+Seek>(&mut self, reader: &mut Rs)->Result<Option<ValueKey>,RegError>{
        if self.next_index >= self.value_offsets.len(){
            Ok(None)
        }
        else {
            let cell_offset = self.value_offsets[self.next_index] as u64 + HBIN_START_OFFSET;
            match Cell::at_offset(reader, cell_offset)?.get_data()?{
                CellData::ValueKey(vk)=>{
                    self.next_index += 1;
                    Ok(Some(vk))
                },
                other => panic!("CellData is not type ValueKey: {:?}",other)
            }
        }
    }
}

bitflags! {
    pub struct VkFlags: u16 {
        const VK_VALUE_COMP_NAME = 0x0001;
    }
}
impl fmt::Display for VkFlags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,"{}",self.bits())
    }
}
impl ser::Serialize for VkFlags {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: ser::Serializer
    {
        serializer.serialize_str(&format!("{:?}", self))
    }
}

pub struct VkDataType(u32);
impl VkDataType {
    pub fn new(value: u32) -> VkDataType {
        VkDataType(value)
    }

    pub fn as_string(&self)->String{
        match self.0 {
            0x00000000 => "REG_NONE".to_string(),
            0x00000001 => "REG_SZ".to_string(),
            0x00000002 => "REG_EXPAND_SZ".to_string(),
            0x00000003 => "REG_BINARY".to_string(),
            0x00000004 => "REG_DWORD_LITTLE_ENDIAN".to_string(),
            0x00000005 => "REG_DWORD_BIG_ENDIAN".to_string(),
            0x00000006 => "REG_LINK".to_string(),
            0x00000007 => "REG_MULTI_SZ".to_string(),
            0x00000008 => "REG_RESOURCE_LIST".to_string(),
            0x00000009 => "REG_FULL_RESOURCE_DESCRIPTOR".to_string(),
            0x0000000a => "REG_RESOURCE_REQUIREMENTS_LIST".to_string(),
            0x0000000b => "REG_QWORD_LITTLE_ENDIAN".to_string(),
            _ => format!("REG_TYPE: 0x{:08X}",self.0)
        }
    }

    pub fn as_u32(&self)->u32{
        self.0
    }
}
impl fmt::Display for VkDataType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,"{}",self.as_string())
    }
}
impl fmt::Debug for VkDataType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,"{}",self.as_string())
    }
}
impl ser::Serialize for VkDataType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: ser::Serializer
    {
        serializer.serialize_str(&self.as_string())
    }
}

// vk
#[derive(Debug)]
pub struct ValueKey {
    _offset: u64,
    signature: u16,
    value_name_size: u16,
    data_size: u32,
    data_offset: u32,
    data_type: VkDataType,
    flags: VkFlags,
    unknown1: u16,
    value_name: String,
    padding: Vec<u8>,
    data: Vec<u8>,
    data_slack: Vec<u8>
}
impl ValueKey {
    pub fn new(buffer: &[u8], offset: u64)->Result<ValueKey,RegError> {
        let signature = LittleEndian::read_u16(&buffer[0..2]);
        let value_name_size = LittleEndian::read_u16(&buffer[2..4]);
        let data_size = LittleEndian::read_u32(&buffer[4..8]);
        let data_offset = LittleEndian::read_u32(&buffer[8..12]);
        let data_type = VkDataType::new(
            LittleEndian::read_u32(&buffer[12..16])
        );
        let flags = VkFlags::from_bits_truncate(
            LittleEndian::read_u16(&buffer[16..18])
        );
        let unknown1 = LittleEndian::read_u16(&buffer[18..20]);

        let value_name = match flags.contains(VkFlags::VK_VALUE_COMP_NAME) {
            true => utils::read_ascii(&buffer[20..(20 + value_name_size) as usize])?,
            false => utils::read_utf16(&buffer[20..(20 + value_name_size) as usize])?
        };

        let padding = buffer[(20 + value_name_size) as usize..].to_vec();
        let data = Vec::new();
        let data_slack = Vec::new();

        Ok(
            ValueKey {
                _offset: offset,
                signature: signature,
                value_name_size: value_name_size,
                data_size: data_size,
                data_offset: data_offset,
                data_type: data_type,
                flags: flags,
                unknown1: unknown1,
                value_name: value_name,
                padding: padding,
                data: data,
                data_slack: data_slack
            }
        )
    }

    pub fn get_name(&self)->&str {
        &self.value_name
    }

    pub fn data_is_resident(&self)->bool {
        if self.data_size >> 31 == 0 {
            false
        } else {
            true
        }
    }

    pub fn get_size(&self) -> u32 {
        if self.data_is_resident(){
            self.data_size - 0x80000000
        } else {
            self.data_size
        }
    }

    pub fn read_value<Rs: Read+Seek>(&mut self, reader: &mut Rs)->Result<bool,RegError>{
        //check most significant bit if data resides in offset
        if !self.data_is_resident() {
            // data is not stored in offset, so lets seek to the offset
            // seek to data value
            let cell = Cell::at_offset(reader, self.data_offset as u64 + HBIN_START_OFFSET)?;

            match cell.get_data()? {
                CellData::DataBlock(data_block) => {
                    let data = data_block.get_data(reader)?;
                    self.data = data;
                },
                CellData::Raw(rd) => {
                    self.data = rd;
                },
                other => {
                    error!("read_value_from_hive unhandled cell data type: {:?}",other);
                    panic!("read_value_from_hive unhandled cell data type: {:?}",other);
                }
            }

            Ok(true)
        } else {
            let raw_buffer: [u8; 4] = unsafe {
                transmute(self.data_offset.to_le())
            };

            // set data
            self.data = raw_buffer.to_vec();

            Ok(true)
        }
    }

    pub fn decode_data(&self)->Result<Option<Data>,RegError>{
        // Check if data is a db record
        // If it is, we will need to jump to multiple places to read data.
        let data_len = self.data.len();
        if data_len > 0 {
            if self.get_size() > data_len as u32 {
                panic!("Size is greater than data: {} > {}\n{:?}",self.get_size(),data_len,self);
            }

            match self.data_type.0 {
                0x00000000 => { //REG_NONE
                    return Ok(None);
                },
                0x00000001 => { //REG_SZ
                    let d_size = self.get_size();

                    if d_size == 0 {
                        return Ok(None);
                    }

                    let value = utils::read_utf16(
                        &self.data[0..d_size as usize]
                    )?;

                    return Ok(
                        Some(Data::String(value))
                    );
                },
                0x00000002 => { //REG_EXPAND_SZ
                    let d_size = self.get_size();

                    if d_size == 0 {
                        return Ok(None);
                    }

                    let value = utils::read_utf16(
                        &self.data[0..d_size as usize]
                    )?;

                    return Ok(
                        Some(Data::String(value))
                    );
                },
                0x00000003 => { //REG_BINARY
                    let value = utils::to_hex_string(
                        &self.data
                    );
                    return Ok(Some(Data::String(value)));
                }
                0x00000004 => { //REG_DWORD_LITTLE_ENDIAN
                    let value = LittleEndian::read_i32(
                        &self.data[0..4]
                    );

                    return Ok(Some(Data::Int32(value)));
                },
                0x00000005 => { //REG_DWORD_BIG_ENDIAN
                    let value = BigEndian::read_i32(
                        &self.data[0..4]
                    );

                    return Ok(Some(Data::Int32(value)));
                },
                _ => {
                    let value = utils::to_hex_string(
                        &self.data[0..self.get_size() as usize]
                    );
                    return Ok(Some(Data::String(value)));
                }
            }
        }
        else {
            return Ok(None);
        }
    }
}
impl ser::Serialize for ValueKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: ser::Serializer
    {
        let mut state = serializer.serialize_struct("ValueKey", 5)?;
        state.serialize_field("data_size", &self.data_size)?;
        state.serialize_field("data_type", &self.data_type)?;
        state.serialize_field("flags", &self.flags)?;
        state.serialize_field("value_name", &self.value_name)?;
        let data = match self.decode_data() {
            Ok(data)=>data,
            Err(error)=>{
                panic!("{:?}",error)
            }
        };
        state.serialize_field("data", &data)?;
        state.end()
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
    fn valuekey() {
        let mut file = File::open(".testdata/NTUSER_4680_40_CELL_VK.DAT").unwrap();
        let mut buffer = Vec::new();

        match file.read_to_end(&mut buffer){
            Err(error)=>panic!("{:?}",error),
            _ => {}
        }

        let cell = match Cell::new(&mut Cursor::new(&buffer),0){
            Ok(cell)=>cell,
            Err(error)=>panic!("{:?}",error)
        };

        let vk = match ValueKey::new(&cell.data,4){
            Ok(vk)=>vk,
            Err(error)=>panic!("{:?}",error)
        };

        assert_eq!(vk.signature, 27510);
        assert_eq!(vk.value_name_size, 10);
        assert_eq!(vk.data_size, 84);
        assert_eq!(vk.data_offset, 12416);
        assert_eq!(vk.data_type.as_string(), String::from("REG_SZ"));
        assert_eq!(format!("{:?}",vk.flags), String::from("VK_VALUE_COMP_NAME"));
        assert_eq!(vk.unknown1, 25970);
        assert_eq!(vk.value_name, String::from("User Agent"));

        let known_data: &[u8] = &[
            0x00,0x00,0x6C,0x66,0x01,0x00
        ];
        assert_eq!(&vk.padding[..], known_data);
    }
}
