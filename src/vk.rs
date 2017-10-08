use byteorder::{ReadBytesExt, LittleEndian, BigEndian};
use serde::ser::{SerializeStruct};
use utils;
use hive::HBIN_START_OFFSET;
use cell::{Cell,CellData};
use errors::{RegError};
use serde::{ser};
use std::io::Read;
use std::io::{Seek,SeekFrom};
use std::io::{Cursor};
use std::fmt;
use std::mem::transmute;

#[derive(Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum Data {
    None,
    String(String),
    Int32(i32),
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

#[derive(Clone)]
pub struct VkDataType(pub u32);
impl VkDataType {
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
// TODO: Take into account db cell for data value!
#[derive(Debug, Clone)]
pub struct ValueKey {
    _offset: u64,
    pub value_name_size: u16,
    pub data_size: u32,
    pub data_offset: u32,
    pub data_type: VkDataType,
    pub flags: VkFlags,
    unknown1: u16,
    // 18 bytes
    pub value_name: String,
    // padding: utils::ByteArray
    pub data: Option<Vec<u8>>,
    pub data_slack: Option<Vec<u8>>
}
impl ValueKey {
    pub fn new<Rs: Read+Seek>(mut reader: Rs, offset: u64)->Result<ValueKey,RegError> {
        let _offset = offset;
        let value_name_size = reader.read_u16::<LittleEndian>()?;
        let data_size = reader.read_u32::<LittleEndian>()?;
        let data_offset = reader.read_u32::<LittleEndian>()?;
        let data_type = VkDataType(reader.read_u32::<LittleEndian>()?);
        let flags = VkFlags::from_bits_truncate(
            reader.read_u16::<LittleEndian>()?
        );
        let unknown1 = reader.read_u16::<LittleEndian>()?;

        let mut name_buffer = vec![0; value_name_size as usize];
        reader.read_exact(name_buffer.as_mut_slice())?;
        let value_name = match flags.contains(VkFlags::VK_VALUE_COMP_NAME) {
            true => String::from_utf8(name_buffer)?,
            false => utils::uft16_from_u8_vec(&name_buffer)?
        };

        // let pad_size = 8 - ((_offset + 18 + value_name_size as u64) % 8);
        // let mut padding_buffer = vec![0; pad_size as usize];
        // reader.read_exact(padding_buffer.as_mut_slice())?;
        // let padding = utils::ByteArray(padding_buffer);

        let data = None;
        let data_slack = None;

        Ok(
            ValueKey {
                _offset: _offset,
                value_name_size: value_name_size,
                data_size: data_size,
                data_offset: data_offset,
                data_type: data_type,
                flags: flags,
                unknown1: unknown1,
                value_name: value_name,
                // padding: padding
                data: data,
                data_slack: data_slack
            }
        )
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

    pub fn read_value_from_hive<Rs: Read+Seek>(&mut self, mut reader: Rs)->Result<bool,RegError>{
        //check most significant bit if data resides in offset
        if !self.data_is_resident() {
            // data is not stored in offset, so lets seek to the offset
            // seek to data value
            reader.seek(SeekFrom::Start(
                HBIN_START_OFFSET + self.data_offset as u64
            ))?;

            let cell = match Cell::from_value_key(
                &mut reader,
                self
            ){
                Ok(cell) => cell,
                Err(error) => {
                    error!(
                        "ValueKey<{}>.read_value_from_hive() Error Cell::from_value_key at offset {}. Error: {:?}\n{:?}",
                        self._offset, HBIN_START_OFFSET + self.data_offset as u64,
                        error, self
                    );
                    return Err(error);
                }
            };

            match cell.data {
                CellData::DataBlock(data_block) => {
                    let data = match data_block.get_data(&mut reader){
                        Ok(data) => data,
                        Err(error) => {
                            panic!(
                                "Unable to DataBlock<{}>.get_data() at ValueKey<{}>.read_value_from_hive(). Error: {:?}\n{:?}",
                                data_block._offset,self._offset,error,self
                            );
                        }
                    };

                    self.data = Some(data);
                    // panic!("read_value_from_hive unhandled cell data type: {:?}",cell);
                },
                CellData::Raw(rd) => {
                    self.data = Some(rd);
                },
                _ => {
                    error!("read_value_from_hive unhandled cell data type: {:?}",cell);
                    panic!("read_value_from_hive unhandled cell data type: {:?}",cell);
                }
            }

            Ok(true)
        } else {
            let raw_buffer: [u8; 4] = unsafe {
                transmute(self.data_offset.to_le())
            };

            // set data
            self.data = Some(raw_buffer.to_vec());

            Ok(true)
        }
    }

    pub fn get_name(&self)->String{
        self.value_name.clone()
    }

    pub fn decode_data(&self)->Option<Data>{
        // Check if data is a db record
        // If it is, we will need to jump to multiple places to read data.
        match self.data {
            Some(ref data) => {
                if self.get_size() > data.len() as u32 {
                    panic!("Size is less than data: {} > {}\n{:?}",self.get_size(),data.len(),self);
                }

                match self.data_type.0 {
                    0x00000000 => { //REG_NONE
                        return Some(
                            Data::None
                        );
                    },
                    0x00000001 => { //REG_SZ
                        // Value has possible null terminator, but is not guaranteed
                        if self.flags.contains(VkFlags::VK_VALUE_COMP_NAME) {
                            let d_size = self.get_size();

                            if d_size == 0 {
                                return None;
                            }

                            let value = match utils::read_utf16(
                                    &data[0..d_size as usize].to_vec()
                                )
                            {
                                Ok(value) => {
                                    value
                                },
                                Err(error) => {
                                    panic!("{:?} {} {}",self,error,backtrace!())
                                }
                            };

                            return Some(
                                Data::String(value)
                            );
                        } else {
                            let d_size = self.get_size();

                            if d_size == 0 {
                                return None;
                            }

                            let value = match utils::read_string_u8_till_null(
                                Cursor::new(
                                    &data[0..d_size as usize].to_vec()
                                ))
                            {
                                Ok(value) => {
                                    value
                                },
                                Err(error) => {
                                    panic!("{:?}",error)
                                }
                            };

                            return Some(
                                Data::String(value)
                            );
                        }
                    },
                    0x00000002 => { //REG_EXPAND_SZ
                        if self.flags.contains(VkFlags::VK_VALUE_COMP_NAME) {
                            let d_size = self.get_size();

                            if d_size == 0 {
                                return None;
                            }

                            let value = match utils::read_utf16(
                                    &data[0..d_size as usize].to_vec()
                                )
                            {
                                Ok(value) => {
                                    value
                                },
                                Err(error) => {
                                    panic!("{:?} {} {}",self,error,backtrace!())
                                }
                            };

                            return Some(
                                Data::String(value)
                            );
                        } else {
                            let d_size = self.get_size();

                            if d_size == 0 {
                                return None;
                            }

                            let value = match utils::read_string_u8_till_null(
                                Cursor::new(
                                    &data[0..d_size as usize].to_vec()
                                ))
                            {
                                Ok(value) => {
                                    value
                                },
                                Err(error) => {
                                    panic!("{:?}",error)
                                }
                            };

                            return Some(
                                Data::String(value)
                            );
                        }
                    },
                    0x00000003 => { //REG_BINARY
                        let value = utils::to_hex_string(
                            &data[0..self.get_size() as usize].to_vec()
                        );
                        return Some(Data::String(value));
                    }
                    0x00000004 => { //REG_DWORD_LITTLE_ENDIAN
                        let value = match Cursor::new(
                            &data[0..self.get_size() as usize].to_vec()
                        ).read_i32::<LittleEndian>(){
                            Ok(value) => {
                                value
                            },
                            Err(error) => {
                                panic!("{:?}",error)
                            }
                        };

                        return Some(Data::Int32(value));
                    },
                    0x00000005 => { //REG_DWORD_BIG_ENDIAN
                        let value = match Cursor::new(
                            &data[0..self.get_size() as usize].to_vec()
                        ).read_i32::<BigEndian>(){
                            Ok(value) => {
                                value
                            },
                            Err(error) => {
                                panic!("{:?}",error)
                            }
                        };

                        return Some(Data::Int32(value));
                    },
                    _ => {
                        let value = utils::to_hex_string(
                            &data[0..self.get_size() as usize].to_vec()
                        );
                        return Some(Data::String(value));
                    }
                }
            },
            None => {
                return None;
            }
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
        state.serialize_field("data", &self.decode_data())?;
        state.end()
    }
}

#[derive(Clone)]
pub struct ValueData(pub Vec<u8>);
impl fmt::Display for ValueData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:?}",
            utils::to_hex_string(&self.0),
        )
    }
}
impl fmt::Debug for ValueData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:?}",
            utils::to_hex_string(&self.0),
        )
    }
}
impl ser::Serialize for ValueData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: ser::Serializer
    {
        serializer.serialize_str(
            &format!("{}", utils::to_hex_string(&self.0))
        )
    }
}
