use rwinstructs::timestamp::{WinTimestamp};
use byteorder::{ReadBytesExt, LittleEndian};
use utils;
use errors::{RegError};
use serde::{ser};
use std::io::Read;
use std::io::{Seek,SeekFrom};
use std::io::{Cursor};
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
    UnhandledCellData(UnhandledCellData),
    // IndexLeaf(),
    // FastLeaf(),
    // HashLeaf(),
    // IndexRoot(),
    KeyNode(KeyNode),
    ValueKey(ValueKey),
    // KeySecurity(),
    // BigData()
}

#[derive(Serialize, Debug)]
pub struct Cell{
    #[serde(skip_serializing)]
    _offset: u64,
    pub size: i32,
    pub signature: u16,
    pub data: CellData
}
impl Cell {
    pub fn new<Rs: Read+Seek>(mut reader: Rs) -> Result<Cell,RegError> {
        let _offset = reader.seek(SeekFrom::Current(0))?;
        let size = reader.read_i32::<LittleEndian>()?;
        let signature = reader.read_u16::<LittleEndian>()?;

        let mut buffer = vec![0;(size.abs() - 6) as usize];
        reader.read_exact(buffer.as_mut_slice())?;

        match signature {
            27502 => { // 'nk'
                let data = CellData::KeyNode(
                    KeyNode::new(
                        Cursor::new(buffer),
                        _offset + 6
                    )?
                );
                Ok(
                    Cell {
                        _offset: _offset,
                        size: size,
                        signature: signature,
                        data: data
                    }
                )
            },
            27510 => { // 'vk'
                let data = CellData::ValueKey(
                    ValueKey::new(
                        Cursor::new(buffer),
                        _offset + 6
                    )?
                );
                Ok(
                    Cell {
                        _offset: _offset,
                        size: size,
                        signature: signature,
                        data: data
                    }
                )
            }
            _ => {
                let data = CellData::UnhandledCellData(
                    UnhandledCellData(buffer)
                );
                Ok(
                    Cell {
                        _offset: _offset,
                        size: size,
                        signature: signature,
                        data: data
                    }
                )
            }
        }
    }

    pub fn is_allocated(&self)->bool{
        if self.size.is_negative() {
            true
        } else {
            false
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
pub enum VkDataType {
    REG_NONE = 0x00000000,
    REG_SZ = 0x00000001,
    REG_EXPAND_SZ = 0x00000002,
    REG_BINARY = 0x00000003,
    REG_DWORD_LITTLE_ENDIAN = 0x00000004,
    REG_DWORD_BIG_ENDIAN = 0x00000005,
    REG_LINK = 0x00000006,
    REG_MULTI_SZ = 0x00000007,
    REG_RESOURCE_LIST = 0x00000008,
    REG_FULL_RESOURCE_DESCRIPTOR = 0x00000009,
    REG_RESOURCE_REQUIREMENTS_LIST = 0x0000000a,
    REG_QWORD_LITTLE_ENDIAN = 0x0000000b
}
// vk
#[derive(Serialize, Debug)]
pub struct ValueKey {
    #[serde(skip_serializing)]
    _offset: u64,
    pub value_name_size: u16,
    pub data_size: u32,
    pub data_offset: u32,
    pub data_type: u32,
    pub flags: VkFlags,
    unknown1: u16,
    // 18 bytes
    pub value_name: String,
    padding: utils::ByteArray
}
impl ValueKey {
    pub fn new<Rs: Read+Seek>(mut reader: Rs, offset: u64) -> Result<ValueKey,RegError> {
        let _offset = offset;
        let value_name_size = reader.read_u16::<LittleEndian>()?;
        let data_size = reader.read_u32::<LittleEndian>()?;
        let data_offset = reader.read_u32::<LittleEndian>()?;
        let data_type = reader.read_u32::<LittleEndian>()?;
        let flags = VkFlags::from_bits_truncate(
            reader.read_u16::<LittleEndian>()?
        );
        let unknown1 = reader.read_u16::<LittleEndian>()?;

        let mut name_buffer = vec![0; value_name_size as usize];
        reader.read_exact(name_buffer.as_mut_slice())?;
        let value_name = match flags.contains(VK_VALUE_COMP_NAME) {
            true => String::from_utf8(name_buffer)?,
            false => utils::uft16_from_u8_vec(&name_buffer)?
        };

        let pad_size = 8 - ((_offset + 18 + value_name_size as u64) % 8);
        let mut padding_buffer = vec![0; pad_size as usize];
        reader.read_exact(padding_buffer.as_mut_slice())?;
        let padding = utils::ByteArray(padding_buffer);

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
                padding: padding
            }
        )
    }
}

bitflags! {
    pub struct KeyNodeFlags: u16 {
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
impl fmt::Display for KeyNodeFlags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,"{}",self.bits())
    }
}
impl ser::Serialize for KeyNodeFlags {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: ser::Serializer
    {
        serializer.serialize_str(&format!("{:?}", self))
    }
}

// nk
#[derive(Serialize, Debug)]
pub struct KeyNode {
    #[serde(skip_serializing)]
    _offset: u64,
    pub flags: KeyNodeFlags,
    pub last_written: WinTimestamp,
    pub access_bits: u32,
    pub offset_parent_key: u32,
    pub num_sub_keys: u32,
    pub num_volatile_sub_keys: u32,
    pub offset_sub_key_list: u32, //0xffffffff = empty
    pub offset_volatile_sub_key_list: u32, //0xffffffff = empty
    pub num_values: u32,
    pub offset_value_list: u32, //0xffffffff = empty
    pub offset_security_key: u32, //0xffffffff = empty
    pub offset_class_name: u32, //0xffffffff = empty
    pub largest_sub_key_name_size: u32,
    pub largest_sub_key_class_name_size: u32,
    pub largest_value_name_size: u32,
    pub largest_value_data_size: u32,
    pub work_var: u32,
    pub key_name_size: u16,
    pub class_name_size: u16,
    // 74 bytes
    pub key_name: String,
    pub padding: utils::ByteArray // Padding due to 8 byte alignment of cell size. Sometimes contains remnant data
}
impl KeyNode {
    pub fn new<Rs: Read+Seek>(mut reader: Rs, offset: u64) -> Result<KeyNode,RegError> {
        let _offset = offset;
        let flags = KeyNodeFlags::from_bits_truncate(
            reader.read_u16::<LittleEndian>()?
        );
        let last_written = WinTimestamp(
            reader.read_u64::<LittleEndian>()?
        );
        let access_bits = reader.read_u32::<LittleEndian>()?;
        let offset_parent_key = reader.read_u32::<LittleEndian>()?;
        let num_sub_keys = reader.read_u32::<LittleEndian>()?;
        let num_volatile_sub_keys = reader.read_u32::<LittleEndian>()?;
        let offset_sub_key_list = reader.read_u32::<LittleEndian>()?;
        let offset_volatile_sub_key_list = reader.read_u32::<LittleEndian>()?;
        let num_values = reader.read_u32::<LittleEndian>()?;
        let offset_value_list = reader.read_u32::<LittleEndian>()?;
        let offset_security_key = reader.read_u32::<LittleEndian>()?;
        let offset_class_name = reader.read_u32::<LittleEndian>()?;
        let largest_sub_key_name_size = reader.read_u32::<LittleEndian>()?;
        let largest_sub_key_class_name_size = reader.read_u32::<LittleEndian>()?;
        let largest_value_name_size = reader.read_u32::<LittleEndian>()?;
        let largest_value_data_size = reader.read_u32::<LittleEndian>()?;
        let work_var = reader.read_u32::<LittleEndian>()?;
        let key_name_size = reader.read_u16::<LittleEndian>()?;
        let class_name_size = reader.read_u16::<LittleEndian>()?;

        let mut name_buffer = vec![0; key_name_size as usize];
        reader.read_exact(name_buffer.as_mut_slice())?;
        let key_name = match flags.contains(KEY_COMP_NAME) {
            true => String::from_utf8(name_buffer)?,
            false => utils::uft16_from_u8_vec(&name_buffer)?
        };

        // 8 byte alignment
        let pad_size = 8 - ((_offset + 74 + key_name_size as u64) % 8);

        // println!("pad_size: {}",pad_size);
        let mut padding_buffer = vec![0; pad_size as usize];
        reader.read_exact(padding_buffer.as_mut_slice())?;
        let padding = utils::ByteArray(padding_buffer);

        Ok(
            KeyNode {
                _offset: _offset,
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
                padding: padding
            }
        )
    }
}

pub struct UnhandledCellData(pub Vec<u8>);
impl fmt::Debug for UnhandledCellData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:?}",
            utils::to_hex_string(&self.0),
        )
    }
}
impl ser::Serialize for UnhandledCellData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: ser::Serializer
    {
        serializer.serialize_str(
            &format!("{}", utils::to_hex_string(&self.0))
        )
    }
}
