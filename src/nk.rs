use rwinstructs::timestamp::{WinTimestamp};
use cell::{Cell,CellData};
use errors::{RegError};
use sk::{SecurityKey};
use hive::HBIN_START_OFFSET;
use utils;
use byteorder::{ReadBytesExt, LittleEndian};
use serde::{ser};
use std::io::Read;
use std::io::{Seek,SeekFrom};
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

// nk
#[derive(Serialize, Debug, Clone)]
pub struct NodeKey {
    #[serde(skip_serializing)]
    _offset: u64,
    pub flags: NodeKeyFlags,
    pub last_written: WinTimestamp,
    pub access_bits: u32,
    pub offset_parent_key: u32,
    pub num_sub_keys: u32, // node keys
    pub num_volatile_sub_keys: u32,
    pub offset_sub_key_list: u32, //0xffffffff = empty
    pub offset_volatile_sub_key_list: u32, //0xffffffff = empty
    pub num_values: u32, // value keys
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
    // pub padding: utils::ByteArray // Padding due to 8 byte alignment of cell size. Sometimes contains remnant data

    // fields to maintain position of iteration
    #[serde(skip_serializing)]
    pub value_list: Option<Box<ValueKeyList>>,
    #[serde(skip_serializing)]
    pub sub_key_list: Option<Box<Cell>>,
    pub security_key: Option<SecurityKey>
}
impl NodeKey {
    pub fn new<Rs: Read+Seek>(mut reader: Rs, offset: u64) -> Result<NodeKey,RegError> {
        let _offset = offset;

        let flags = NodeKeyFlags::from_bits_truncate(
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

        let key_name = match flags.contains(NodeKeyFlags::KEY_COMP_NAME) {
            true => {
                utils::ascii_from_u8_vec(&name_buffer)?
            },
            false => {
                utils::uft16_from_u8_vec(&name_buffer)?
            }
        };

        // 8 byte alignment
        // let pad_size = 8 - ((_offset + 74 + key_name_size as u64) % 8);
        // println!("pad_size: {}",pad_size);
        // let mut padding_buffer = vec![0; pad_size as usize];
        // reader.read_exact(padding_buffer.as_mut_slice())?;
        // let padding = utils::ByteArray(padding_buffer);
        let value_list = None;
        let sub_key_list = None;
        let security_key = None;

        Ok(
            NodeKey {
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
                // padding: padding,
                value_list: value_list,
                sub_key_list: sub_key_list,
                security_key: security_key
            }
        )
    }

    pub fn get_name(&self)->String{
        self.key_name.clone()
    }

    pub fn get_offset(&self)->u64{
        self._offset
    }

    pub fn has_values(&self)->bool{
        if self.offset_value_list == 0xffffffff {
            false
        } else {
            true
        }
    }

    pub fn has_sub_keys(&self)->bool{
        if self.offset_sub_key_list == 0xffffffff {
            false
        } else {
            true
        }
    }

    pub fn has_sec_key(&self)->bool{
        if self.offset_security_key == 0xffffffff {
            false
        } else {
            true
        }
    }

    pub fn needs_value_list(&self)->bool{
        self.value_list.is_none()
    }

    pub fn needs_sub_key_list(&self)->bool{
        self.sub_key_list.is_none()
    }

    pub fn needs_sec_key(&self)->bool{
        if self.security_key.is_some() {
            false
        } else {
            true
        }
    }

    pub fn set_value_list<Rs: Read+Seek>(
        &mut self, mut reader: Rs
    )->Result<bool,RegError>{
        // seek to list
        reader.seek(SeekFrom::Start(
            HBIN_START_OFFSET + self.offset_value_list as u64
        ))?;

        let offset = reader.seek(SeekFrom::Current(0))?;

        // get value list
        self.value_list = Some(
            Box::new(
                ValueKeyList::new(
                    &mut reader,
                    offset,
                    self.num_values
                )?
            )
        );

        Ok(true)
    }

    pub fn set_sub_key_list<Rs: Read+Seek>(
        &mut self, mut reader: Rs
    )->Result<bool,RegError>{
        // seek to cell
        reader.seek(SeekFrom::Start(
            HBIN_START_OFFSET + self.offset_sub_key_list as u64
        ))?;

        let cell = Cell::new(
            &mut reader
        )?;

        debug!("NodeKey<{}>.set_sub_key_list(): {:?}",self._offset,cell);

        // get sub key list
        self.sub_key_list = Some(
            Box::new(cell)
        );

        Ok(true)
    }

    pub fn set_sec_key<Rs: Read+Seek>(
        &mut self, mut reader: Rs
    )->Result<bool,RegError>{
        if self.offset_security_key != 0xffffffff {
            // seek to cell
            reader.seek(SeekFrom::Start(
                HBIN_START_OFFSET + self.offset_security_key as u64
            ))?;

            let cell = Cell::new(
                &mut reader
            )?;

            self.security_key = match cell.data {
                CellData::SecurityKey(sk) => Some(
                    sk
                ),
                _ => {
                    panic!("Cell is not a security key.");
                }
            };

            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn get_next_value<Rs: Read+Seek>(
        &mut self, mut reader: Rs
    )->Result<Option<Cell>,RegError>{
        match self.value_list {
            Some(ref mut vl) => {
                let value = vl.get_next_value(
                    &mut reader
                )?;
                Ok(value)
            },
            None => Ok(
                None
            )
        }
    }

    pub fn get_next_sub_key<Rs: Read+Seek>(
        &mut self, mut reader: Rs
    )->Result<Option<NodeKey>,RegError>{
        if self.needs_sub_key_list(){
            self.set_sub_key_list(&mut reader)?;
        }

        match self.sub_key_list {
            Some(ref mut list) => {
                match list.data {
                    CellData::HashLeaf(ref mut hl) => {
                        let nk = hl.get_next_node(&mut reader)?;
                        Ok(nk)
                    },
                    CellData::FastLeaf(ref mut lf) => {
                        let nk = lf.get_next_node(&mut reader)?;
                        Ok(nk)
                    },
                    CellData::RootIndex(ref mut ri) => {
                        let nk = ri.get_next_node(&mut reader)?;
                        Ok(nk)
                    },
                    CellData::IndexLeaf(ref mut li) => {
                        let nk = li.get_next_node(&mut reader)?;
                        Ok(nk)
                    },
                    _ => {
                        panic!("Unhandled list type: {:?}",list);
                    }
                }
            },
            None => Ok(None)
        }
    }

    pub fn get_sec_key(&self)->Option<SecurityKey>{
            self.security_key.clone()
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct ValueKeyList{
    #[serde(skip_serializing)]
    _offset: u64,
    size: i32,
    offset_list: Vec<u32>,
    number_of_values: u32,
    next_index: usize
}
impl ValueKeyList {
    pub fn new<R: Read>(mut reader: R, offset: u64, number_of_values: u32)->Result<ValueKeyList,RegError>{
        let _offset = offset;

        let size = reader.read_i32::<LittleEndian>()?;
        let abs_size = size.abs();
        let mut offset_list: Vec<u32> = Vec::new();
        let next_index: usize = 0;

        let mut bytes_read: i32 = 4;
        loop {
            let offset = reader.read_u32::<LittleEndian>()?;
            offset_list.push(offset);
            bytes_read += 4;

            if bytes_read >= abs_size {
                break;
            }
        }

        let value_key_list = ValueKeyList{
            _offset: _offset,
            size: size,
            offset_list: offset_list,
            number_of_values: number_of_values,
            next_index: next_index
        };

        Ok(value_key_list)
    }

    pub fn get_next_value<Rs: Read+Seek>(&mut self, mut reader: Rs)->Result<Option<Cell>,RegError>{
        // More offsets can exist in the offset list than there are in the number of ValueKeyList
        // The extra offsets can some times point to valid data, othertimes it is garbage.
        // TODO: Look for a way to try and recover values.

        // Check if current index is within offset list range
        loop {
            let _offset = reader.seek(SeekFrom::Current(0))?;

            if self.next_index == self.number_of_values as usize {
                // For now excape out once we it the number of values
                return Ok(None);
            }

            if self.next_index + 1 > self.offset_list.len() {
                // Recovery is needed here... but for now we will exit if the index is past
                // the number of values
                if self.next_index + 1 == self.number_of_values as usize {
                    return Ok(None);
                }

                // More possible recoverable values here
                println!("I should not be here: {}",_offset);
                return Ok(None);
            } else {
                let cell_offset = self.offset_list[self.next_index];

                // Can we have cells that have 0 offset befor a cell that has a real offset?
                if cell_offset == 0 {
                    self.next_index += 1;
                    continue;
                }

                reader.seek(
                    SeekFrom::Start(HBIN_START_OFFSET + cell_offset as u64)
                )?;

                debug!("ValueKeyList<{}>.get_next_value()",self._offset);

                let mut cell = Cell::new(
                    &mut reader
                )?;

                match cell.data {
                    CellData::ValueKey(ref mut vk) => {
                        match vk.read_value_from_hive(
                            &mut reader
                        ){
                            Err(error) => {
                                error!(
                                    "ValueKeyList<{}>.read_value_from_hive() error: {:?}\n{:?}",
                                    self._offset,error,vk
                                );
                                panic!(
                                    "ValueKeyList<{}>.read_value_from_hive() error: {:?}\n{:?}",
                                    self._offset,error,vk
                                );
                            },
                            _ => {}
                        }
                    },
                    _ => {
                        error!(
                            "ValueKeyList<{}>.get_next_value() Unhandled data type: {:?}",
                            self._offset,cell
                        );
                        panic!(
                            "ValueKeyList<{}>.get_next_value() Unhandled data type: {:?}",
                            self._offset,cell
                        );
                    }
                }
                self.next_index += 1;

                return Ok(
                    Some(cell)
                );
            }
        }
    }
}
