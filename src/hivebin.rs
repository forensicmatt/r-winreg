use rwinstructs::timestamp::{WinTimestamp};
use rwinstructs::security::{SecurityDescriptor};
use byteorder::{ReadBytesExt, LittleEndian};
use utils;
use hive::Hive;
use hive::HBIN_START_OFFSET;
use errors::{RegError};
use serde::{ser};
use std::io::Read;
use std::io::{Seek,SeekFrom};
use std::io::{Cursor};
use std::fmt;
use std::slice::Iter;

#[derive(Serialize,Debug)]
pub struct HiveBin {
    _offset: u64,
    header: HiveBinHeader,
    cells: Vec<Cell>
}
impl HiveBin{
    pub fn new<Rs: Read+Seek>(mut reader: Rs) -> Result<HiveBin,RegError> {
        let _offset = reader.seek(SeekFrom::Current(0))?;
        debug!("reading hivebin at offset: {}", _offset);

        let header = HiveBinHeader::new(
            &mut reader
        )?;
        let mut cells: Vec<Cell> = Vec::new();

        // make a cell buffer
        let mut raw_cell_buffer = vec![0; (header.get_size() - 32) as usize];
        reader.read_exact(
            raw_cell_buffer.as_mut_slice()
        )?;
        let mut cell_cursor = Cursor::new(
            raw_cell_buffer
        );

        let mut count = 0;
        loop {
            let cell = match Cell::new(&mut cell_cursor, false){
                Ok(cell) => cell,
                Err(error) => {
                    error!("{:?}",error);
                    break;
                }
            };

            cells.push(cell);
            count += 1;
        }

        Ok(
            HiveBin{
                _offset: _offset,
                header: header,
                cells: cells
            }
        )
    }

    pub fn next<Rs: Read+Seek>(&self, mut reader: Rs)->Result<HiveBin,RegError>{
        // Get the offest of the next hbin
        let next_hbin_offset = self._offset +
                               self.header.get_size() as u64;

        // Seek to that hbin offset
        reader.seek(
            SeekFrom::Start(next_hbin_offset)
        )?;

        // Parse the next hbin
        let hbin = HiveBin::new(&mut reader)?;

        Ok(
            hbin
        )
    }

    pub fn get_size(&self)->u32{
        self.header.get_size()
    }
}

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

    pub fn get_size(&self)->u32{
        self.size
    }
}

#[derive(Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum CellData{
    UnhandledCellData(UnhandledCellData),
    UnknownData(UnhandledCellData),
    // IndexLeaf(),
    FastLeaf(FastLeaf),
    // HashLeaf(),
    // IndexRoot(),
    NodeKey(NodeKey),
    ValueKey(ValueKey),
    ValueData(ValueData),
    SecurityKey(SecurityKey),
    // BigData(),
    None
}

#[derive(Clone)]
pub struct CellSignature(pub u16);
impl CellSignature {
    pub fn new<R: Read>(mut reader: R) -> Result<CellSignature,RegError> {
        let value = reader.read_u16::<LittleEndian>()?;
        Ok(
            CellSignature(value)
        )
    }

    pub fn as_string(&self)->String{
        match self.0 {
            26220 => "lf".to_string(),
            26732 => "lh".to_string(),
            26988 => "li".to_string(),
            26994 => "ri".to_string(),
            27502 => "nk".to_string(),
            27507 => "sk".to_string(),
            27510 => "vk".to_string(),
            25188 => "db".to_string(),
            _ => format!("UNHANDLED_TYPE: 0x{:04X}",self.0)
        }
    }
}
impl fmt::Display for CellSignature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,"{}",self.as_string())
    }
}
impl fmt::Debug for CellSignature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,"{}",self.as_string())
    }
}
impl ser::Serialize for CellSignature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: ser::Serializer
    {
        serializer.serialize_str(&self.as_string())
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct Cell{
    #[serde(skip_serializing)]
    _offset: u64,
    pub size: i32,
    pub signature: Option<CellSignature>,
    pub data: CellData
}
impl Cell {
    pub fn new<Rs: Read+Seek>(mut reader: Rs, is_value_data: bool) -> Result<Cell,RegError> {
        let _offset = reader.seek(SeekFrom::Current(0))?;
        let size = reader.read_i32::<LittleEndian>()?;

        // Create the cell data buffer
        let mut buffer = vec![0;(size.abs() - 4) as usize];
        reader.read_exact(
            buffer.as_mut_slice()
        )?;

        if is_value_data {
            let data = CellData::ValueData(
                ValueData(buffer)
            );

            Ok(
                Cell {
                    _offset: _offset,
                    size: size,
                    signature: None,
                    data: data
                }
            )
        } else {
            let mut sig_buffer = vec![
                buffer.remove(0),
                buffer.remove(0)
            ];
            let signature = CellSignature::new(
                Cursor::new(sig_buffer)
            )?;

            let data = match signature.0 {
                26220 => { // 'lf'
                    CellData::FastLeaf(
                        FastLeaf::new(
                            Cursor::new(buffer)
                        )?
                    )
                },
                26732 => { // 'lh'
                    CellData::UnhandledCellData(
                        UnhandledCellData(buffer)
                    )
                },
                26988 => { // 'li'
                    CellData::UnhandledCellData(
                        UnhandledCellData(buffer)
                    )
                },
                26994 => { // 'ri'
                    CellData::UnhandledCellData(
                        UnhandledCellData(buffer)
                    )
                },
                27502 => { // 'nk'
                    CellData::NodeKey(
                        NodeKey::new(
                            Cursor::new(buffer),
                            _offset + 6
                        )?
                    )
                },
                27507 => { // 'sk'
                    CellData::SecurityKey(
                        SecurityKey::new(
                            Cursor::new(buffer),
                            _offset + 6
                        )?
                    )
                },
                27510 => { // 'vk'
                    let mut value_key = ValueKey::new(
                        Cursor::new(buffer),
                        _offset + 6
                    )?;

                    CellData::ValueKey(
                        value_key
                    )
                },
                25188 => { // 'db'
                    CellData::UnhandledCellData(
                        UnhandledCellData(buffer)
                    )
                },
                _ => {
                    CellData::UnknownData(
                        UnhandledCellData(buffer)
                    )
                }
            };

            Ok(
                Cell {
                    _offset: _offset,
                    size: size,
                    signature: Some(signature),
                    data: data
                }
            )
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

// lf
#[derive(Serialize, Debug, Clone)]
pub struct FastLeaf{
    element_count: u16,
    elements: Vec<FastElement>,
    next_index: usize
}
impl FastLeaf{
    pub fn new<Rs: Read+Seek>(mut reader: Rs) -> Result<FastLeaf,RegError> {
        let _offset = reader.seek(SeekFrom::Current(0))?;
        let element_count = reader.read_u16::<LittleEndian>()?;
        let mut elements: Vec<FastElement> = Vec::new();
        let next_index = 0;

        for i in 0..element_count{
            let element = FastElement::new(&mut reader)?;
            elements.push(element);
        }

        Ok(
            FastLeaf{
                element_count: element_count,
                elements: elements,
                next_index: next_index
            }
        )
    }

    pub fn get_next_node<Rs: Read+Seek>(&mut self, mut reader: Rs)->Result<Option<NodeKey>,RegError>{
        // Check if current index is within offset list range
        if self.next_index + 1 > self.elements.len() {
            Ok(None)
        } else {
            let cell_offset = self.elements[self.next_index].get_node_offset();
            self.next_index += 1;

            reader.seek(
                SeekFrom::Start(HBIN_START_OFFSET + cell_offset as u64)
            )?;

            let mut cell = Cell::new(&mut reader, false)?;
            match cell.data {
                CellData::NodeKey(nk) => {
                    Ok(Some(nk))
                }
                _ => {
                    panic!("Not sure what to do here...")
                }
            }
        }
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct FastElement{
    node_offset: u32,
    hint: String
}
impl FastElement{
    pub fn new<Rs: Read+Seek>(mut reader: Rs) -> Result<FastElement,RegError> {
        let _offset = reader.seek(SeekFrom::Current(0))?;
        let node_offset = reader.read_u32::<LittleEndian>()?;

        let mut utf8_buffer = vec![0;4];
        reader.read_exact(&mut utf8_buffer)?;
        let hint = String::from_utf8(
            utf8_buffer
        )?;

        Ok(
            FastElement{
                node_offset: node_offset,
                hint: hint
            }
        )
    }

    pub fn get_node_offset(&self)->u32{
        self.node_offset
    }
}

// sk
#[derive(Serialize, Debug, Clone)]
pub struct SecurityKey {
    #[serde(skip_serializing)]
    _offset: u64,
    pub unknown1: u16,
    pub previous_sec_key_offset: u32,
    pub next_sec_key_offset: u32,
    pub reference_count: u32,
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

        Ok(
            SecurityKey {
                _offset: _offset,
                unknown1: unknown1,
                previous_sec_key_offset: previous_sec_key_offset,
                next_sec_key_offset: next_sec_key_offset,
                reference_count: reference_count,
                descriptor_size: descriptor_size,
                descriptor: descriptor
            }
        )
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

#[derive(Clone)]
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
#[derive(Serialize, Debug, Clone)]
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

    pub fn read_value_from_hive<Rs: Read+Seek>(&mut self, mut reader: Rs)->Result<bool,RegError>{
        // seek to data value
        reader.seek(SeekFrom::Start(
            HBIN_START_OFFSET + self.data_offset as u64
        ))?;

        // read data
        // datasize is the firt 4 bytes.
        let data_size = reader.read_i32::<LittleEndian>()?.abs() - 4;
        // lets verify that datasize here matches datasize in the struct
        if data_size as u32 != self.data_size {
            RegError::validation_error(
                format!("data_size [{}] is not equal to the ValueKey.data_size [{}].",
                data_size,
                self.data_size)
            );
        }
        let mut raw_buffer = vec![0; data_size as usize];
        reader.read_exact(&mut raw_buffer)?;

        // set data
        self.data = Some(raw_buffer);

        Ok(true)
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

#[derive(Serialize, Debug, Clone)]
pub struct ValueKeyList{
    size: i32,
    offset_list: Vec<u32>,
    number_of_values: u32,
    next_index: usize
}
impl ValueKeyList {
    pub fn new<R: Read>(mut reader: R, number_of_values: u32)->Result<ValueKeyList,RegError>{
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

        Ok(
            ValueKeyList{
                size: size,
                offset_list: offset_list,
                number_of_values: number_of_values,
                next_index: next_index
            }
        )
    }

    pub fn get_next_value<Rs: Read+Seek>(&mut self, mut reader: Rs)->Result<Option<Cell>,RegError>{
        // More offsets can exist in the offset list than there are in the number of ValueKeyList
        // The extra offsets can some times point to valid data, othertimes it is garbage.
        // TODO: Look for a way to try and recover values.

        // Check if current index is within offset list range
        loop {
            let _offset = reader.seek(SeekFrom::Current(0))?;

            println!("found value offset at offset: {}",_offset);
            if self.next_index == self.number_of_values as usize {
                // For now excape out once we it the number of values
                println!("I should be leaving now at offset: {}",_offset);
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
                println!("looking for value at offset: {}",cell_offset);

                // Can we have cells that have 0 offset befor a cell that has a real offset?
                if cell_offset == 0 {
                    self.next_index += 1;
                    continue;
                }

                reader.seek(
                    SeekFrom::Start(HBIN_START_OFFSET + cell_offset as u64)
                )?;

                let mut cell = Cell::new(&mut reader, false)?;
                match cell.data {
                    CellData::ValueKey(ref mut vk) => {
                        vk.read_value_from_hive(&mut reader);
                    }
                    _ => {}
                }
                self.next_index += 1;

                return Ok(
                    Some(cell)
                );
            }
        }
    }
}

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
    value_list: Option<Box<ValueKeyList>>,
    #[serde(skip_serializing)]
    sub_key_list: Option<Box<Cell>>
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
        let key_name = match flags.contains(KEY_COMP_NAME) {
            true => String::from_utf8(name_buffer)?,
            false => utils::uft16_from_u8_vec(&name_buffer)?
        };

        // 8 byte alignment
        // let pad_size = 8 - ((_offset + 74 + key_name_size as u64) % 8);
        // println!("pad_size: {}",pad_size);
        // let mut padding_buffer = vec![0; pad_size as usize];
        // reader.read_exact(padding_buffer.as_mut_slice())?;
        // let padding = utils::ByteArray(padding_buffer);

        let value_list = None;
        let sub_key_list = None;

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
                sub_key_list: sub_key_list
            }
        )
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

    pub fn needs_value_list(&self)->bool{
        self.value_list.is_none()
    }

    pub fn needs_sub_key_list(&self)->bool{
        self.sub_key_list.is_none()
    }

    pub fn set_value_list<Rs: Read+Seek>(&mut self, mut reader: Rs)->Result<bool,RegError>{
        // seek to list
        reader.seek(SeekFrom::Start(
            HBIN_START_OFFSET + self.offset_value_list as u64
        ))?;

        // get value list
        self.value_list = Some(
            Box::new(ValueKeyList::new(&mut reader, self.num_values)?)
        );

        Ok(true)
    }

    pub fn set_sub_key_list<Rs: Read+Seek>(&mut self, mut reader: Rs)->Result<bool,RegError>{
        // seek to cell
        reader.seek(SeekFrom::Start(
            HBIN_START_OFFSET + self.offset_sub_key_list as u64
        ))?;

        // get sub key list
        self.sub_key_list = Some(
            Box::new(Cell::new(&mut reader, false)?)
        );

        Ok(true)
    }

    pub fn get_next_value<Rs: Read+Seek>(&mut self, mut reader: Rs)->Result<Option<Cell>,RegError>{
        match self.value_list {
            Some(ref mut vl) => {
                Ok(
                    vl.get_next_value(&mut reader)?
                )
            },
            None => Ok(None)
        }
    }

    pub fn get_next_sub_key<Rs: Read+Seek>(&mut self, mut reader: Rs)->Result<Option<NodeKey>,RegError>{
        match self.sub_key_list {
            Some(ref mut list) => {
                match list.data {
                    CellData::FastLeaf(ref mut lf) => {
                        let nk = lf.get_next_node(&mut reader)?;
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
}

#[derive(Clone)]
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
