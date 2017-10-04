use rwinstructs::timestamp::{WinTimestamp};
use rwinstructs::security::{SecurityDescriptor};
use byteorder::{ReadBytesExt, LittleEndian};
use utils;
use vk;
use hive::HBIN_START_OFFSET;
use errors::{RegError};
use serde::{ser};
use std::io::Read;
use std::io::{Seek,SeekFrom};
use std::io::{Cursor};
use std::fmt;


#[derive(Serialize,Debug)]
pub struct HiveBin {
    _offset: u64,
    header: HiveBinHeader,
    cells: Vec<Cell>
}
impl HiveBin{
    pub fn new<Rs: Read+Seek>(mut reader: Rs) -> Result<HiveBin,RegError> {
        let _offset = reader.seek(SeekFrom::Current(0))?;

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
            let cell = match Cell::new(&mut cell_cursor){
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
    Raw(Vec<u8>),
    IndexLeaf(IndexLeaf),
    FastLeaf(FastLeaf),
    HashLeaf(HashLeaf),
    RootIndex(RootIndex),
    NodeKey(NodeKey),
    ValueKey(vk::ValueKey),
    ValueData(vk::ValueData),
    SecurityKey(SecurityKey),
    DataBlock(DataBlock)
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
    pub fn new<Rs: Read+Seek>(mut reader: Rs) -> Result<Cell,RegError> {
        let _offset = reader.seek(SeekFrom::Current(0))?;
        let size = reader.read_i32::<LittleEndian>()?;

        // Create the cell data buffer
        let mut buffer = vec![0;(size.abs() - 4) as usize];
        reader.read_exact(
            buffer.as_mut_slice()
        )?;

        let signature = CellSignature(
            Cursor::new(&buffer[0..2]).read_u16::<LittleEndian>()?
        );

        let data = match signature.0 {
            26220 => { // 'lf'
                CellData::FastLeaf(
                    FastLeaf::new(
                        Cursor::new(&buffer[2..]),
                        _offset + 6
                    )?
                )
            },
            26732 => { // 'lh'
                CellData::HashLeaf(
                    HashLeaf::new(
                        Cursor::new(&buffer[2..]),
                        _offset + 6
                    )?
                )
            },
            26988 => { // 'li'
                CellData::IndexLeaf(
                    IndexLeaf::new(
                        Cursor::new(&buffer[2..]),
                        _offset + 6
                    )?
                )
            },
            26994 => { // 'ri'
                CellData::RootIndex(
                    RootIndex::new(
                        Cursor::new(&buffer[2..]),
                        _offset + 6
                    )?
                )
            },
            27502 => { // 'nk'
                CellData::NodeKey(
                    NodeKey::new(
                        Cursor::new(&buffer[2..]),
                        _offset + 6
                    )?
                )
            },
            27507 => { // 'sk'
                CellData::SecurityKey(
                    SecurityKey::new(
                        Cursor::new(&buffer[2..]),
                        _offset + 6
                    )?
                )
            },
            27510 => { // 'vk'
                let mut value_key = vk::ValueKey::new(
                    Cursor::new(&buffer[2..]),
                    _offset + 6
                )?;

                CellData::ValueKey(
                    value_key
                )
            },
            25188 => { // 'db'
                let mut db = DataBlock::new(
                    Cursor::new(&buffer[2..]),
                    _offset + 6
                )?;
                CellData::DataBlock(
                    db
                )
            },
            _ => {
                // Raw data
                CellData::Raw(
                    buffer
                )
            }
        };

        let cell = Cell {
            _offset: _offset,
            size: size,
            signature: Some(signature),
            data: data
        };

        debug!("Cell<{}>::new() => {:?}",_offset,cell);

        Ok(cell)
    }

    pub fn new_raw<Rs: Read+Seek>(mut reader: Rs) -> Result<Cell,RegError> {
        let _offset = reader.seek(SeekFrom::Current(0))?;
        let size = reader.read_i32::<LittleEndian>()?;

        // Create the cell data buffer
        let mut buffer = vec![0;(size.abs() - 4) as usize];
        reader.read_exact(
            buffer.as_mut_slice()
        )?;

        let data = CellData::Raw(
            buffer
        );

        let cell = Cell {
            _offset: _offset,
            size: size,
            signature: None,
            data: data
        };

        Ok(cell)
    }

    pub fn from_value_key<Rs: Read+Seek>(mut reader: Rs, value_key: &vk::ValueKey) -> Result<Cell,RegError> {
        let _offset = reader.seek(SeekFrom::Current(0))?;
        let size = reader.read_i32::<LittleEndian>()?;

        // Create the cell data buffer
        let mut buffer = vec![0;(size.abs() - 4) as usize];
        reader.read_exact(
            buffer.as_mut_slice()
        )?;

        let signature = CellSignature(
            Cursor::new(&buffer[0..2]).read_u16::<LittleEndian>()?
        );

        // We need to check if the size is greater than the current cell, if it is, we need
        // to see if it is a db cell, otherwise error out because not sure what to do.
        if value_key.data_size > size.abs() as u32 {
            // all the data for this value key is not contained in this cell and we should check
            // if it a db cell
            let data = match signature.0 {
                25188 => { // 'db'
                    let mut db = DataBlock::new(
                        Cursor::new(&buffer[2..]),
                        _offset + 6
                    )?;
                    CellData::DataBlock(
                        db
                    )
                },
                _ => {
                    // If the data of the value is greater than the cell, we should have a db cell,
                    // if its not a db cell, im not sure how to handle it.
                    panic!(
                        "Unhandled cell signature {} for Cell<{}>.from_value_key()",
                        signature,_offset
                    );
                }
            };

            let cell = Cell {
                _offset: _offset,
                size: size,
                signature: Some(signature),
                data: data
            };

            Ok(cell)
        } else {
            // Raw data
            let data = CellData::Raw(
                buffer
            );

            let cell = Cell {
                _offset: _offset,
                size: size,
                signature: Some(signature),
                data: data
            };

            Ok(cell)
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

// db
#[derive(Serialize, Debug, Clone)]
pub struct DataBlock{
    #[serde(skip_serializing)]
    pub _offset: u64,
    pub segment_count: u16,
    pub segments_offset: u32
}
impl DataBlock{
    pub fn new<Rs: Read+Seek>(mut reader: Rs, offset: u64) -> Result<DataBlock,RegError> {
        let _offset = offset;

        let segment_count = reader.read_u16::<LittleEndian>()?;
        let segments_offset = reader.read_u32::<LittleEndian>()?;

        let data_block = DataBlock{
            _offset: _offset,
            segment_count: segment_count,
            segments_offset: segments_offset
        };

        Ok(data_block)
    }

    pub fn get_data<Rs: Read+Seek>(&self, mut reader: Rs)->Result<Vec<u8>,RegError> {
        // This data could include slack!
        let mut raw_data: Vec<u8> = Vec::new();

        // Seek to the list offset
        reader.seek(
            SeekFrom::Start(HBIN_START_OFFSET + self.segments_offset as u64)
        )?;
        let mut segments_list: Vec<u32> = Vec::new();

        //The segment_list is a cell in itself of raw data.
        // the first 4 bytes are the cell size, followed by the offset list. This mean that
        // data padding in the list is possible to get though not currently handled
        let list_cell_size = reader.read_i32::<LittleEndian>()?;

        // read offsets into the segments_list
        for i in 0..self.segment_count {
            let offset = reader.read_u32::<LittleEndian>()?;
            debug!("DataBlock<{}> segment offset {}: {}",self._offset,i,offset);
            segments_list.push(
                offset
            );
        }

        for segment_offset in segments_list {
            // Seek to the data offset
            reader.seek(
                SeekFrom::Start(HBIN_START_OFFSET + segment_offset as u64)
            )?;

            // Read cell
            let mut cell = Cell::new_raw(
                &mut reader
            )?;

            match cell.data {
                CellData::Raw(ref mut cell_data) => {
                    raw_data.append(
                        cell_data
                    );
                },
                _ => {
                    panic!("Datablock cell should only be Raw.");
                }
            }
        }

        Ok(raw_data)
    }
}

// lh
#[derive(Serialize, Debug, Clone)]
pub struct HashLeaf{
    #[serde(skip_serializing)]
    _offset: u64,
    element_count: u16,
    elements: Vec<HashElement>,
    next_index: usize
}
impl HashLeaf{
    pub fn new<Rs: Read+Seek>(mut reader: Rs, offset: u64) -> Result<HashLeaf,RegError> {
        let _offset = offset;

        let element_count = reader.read_u16::<LittleEndian>()?;
        let mut elements: Vec<HashElement> = Vec::new();
        let next_index = 0;

        for i in 0..element_count{
            let element = HashElement::new(
                &mut reader
            )?;
            elements.push(element);
        }

        Ok(
            HashLeaf{
                _offset: _offset,
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

            let cell = Cell::new(&mut reader)?;
            match cell.data {
                CellData::NodeKey(nk) => {
                    Ok(Some(nk))
                }
                _ => {
                    panic!(
                        "unhandled data type in HashLeaf<{}>.get_next_node() => {:?}",
                        self._offset,cell.data
                    );
                }
            }
        }
    }
}
#[derive(Serialize, Debug, Clone)]
pub struct HashElement{
    node_offset: u32,
    hash: Vec<u8>
}
impl HashElement{
    pub fn new<Rs: Read+Seek>(mut reader: Rs) -> Result<HashElement,RegError> {
        let node_offset = reader.read_u32::<LittleEndian>()?;

        let mut hash = vec![0;4];
        reader.read_exact(&mut hash)?;

        Ok(
            HashElement{
                node_offset: node_offset,
                hash: hash
            }
        )
    }

    pub fn get_node_offset(&self)->u32{
        self.node_offset
    }
}

// lf
#[derive(Serialize, Debug, Clone)]
pub struct FastLeaf{
    #[serde(skip_serializing)]
    _offset: u64,
    element_count: u16,
    elements: Vec<FastElement>,
    next_index: usize
}
impl FastLeaf{
    pub fn new<Rs: Read+Seek>(mut reader: Rs, offset: u64) -> Result<FastLeaf,RegError> {
        let _offset = offset;

        let element_count = reader.read_u16::<LittleEndian>()?;
        let mut elements: Vec<FastElement> = Vec::new();
        let next_index = 0;

        for i in 0..element_count{
            let element = FastElement::new(&mut reader)?;
            elements.push(element);
        }

        Ok(
            FastLeaf{
                _offset: _offset,
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

            let cell = Cell::new(&mut reader)?;
            match cell.data {
                CellData::NodeKey(nk) => {
                    Ok(Some(nk))
                }
                _ => {
                    panic!(
                        "unhandled data type at FastLeaf<{}>.get_next_node() => {:?}",
                        self._offset,cell
                    );
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

// li
#[derive(Serialize, Debug, Clone)]
pub struct IndexLeaf{
    #[serde(skip_serializing)]
    _offset: u64,
    element_count: u16,
    elements: Vec<u32>,
    next_index: usize
}
impl IndexLeaf{
    pub fn new<Rs: Read+Seek>(mut reader: Rs, offset: u64) -> Result<IndexLeaf,RegError> {
        let _offset = offset;
        let element_count = reader.read_u16::<LittleEndian>()?;
        let mut elements: Vec<u32> = Vec::new();
        let next_index = 0;

        for i in 0..element_count{
            let element = reader.read_u32::<LittleEndian>()?;
            elements.push(element);
        }

        Ok(
            IndexLeaf{
                _offset: _offset,
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
            let cell_offset = self.elements[self.next_index];
            self.next_index += 1;

            reader.seek(
                SeekFrom::Start(HBIN_START_OFFSET + cell_offset as u64)
            )?;

            let cell = Cell::new(&mut reader)?;
            match cell.data {
                CellData::NodeKey(nk) => {
                    Ok(Some(nk))
                }
                _ => {
                    panic!(
                        "unhandled data type at IndexLeaf<{}>.get_next_node() => {:?}",
                        self._offset,cell
                    );
                }
            }
        }
    }
}

// ri
// This points to other lists of NK's
#[derive(Serialize, Debug, Clone)]
pub struct RootIndex{
    _offset: u64,
    element_count: u16,
    elements: Vec<u32>,
    next_index: usize,
    current_cell: Option<Box<Cell>>
}
impl RootIndex{
    pub fn new<Rs: Read+Seek>(mut reader: Rs, offset: u64) -> Result<RootIndex,RegError> {
        let _offset = offset;

        let element_count = reader.read_u16::<LittleEndian>()?;
        let mut elements: Vec<u32> = Vec::new();
        let next_index = 0;
        let current_cell = None;

        for i in 0..element_count{
            let element = reader.read_u32::<LittleEndian>()?;
            elements.push(element);
        }

        let root_index = RootIndex{
            _offset: _offset,
            element_count: element_count,
            elements: elements,
            next_index: next_index,
            current_cell: current_cell
        };

        Ok(root_index)
    }

    pub fn increment_current_cell<Rs: Read+Seek>(&mut self, mut reader: Rs)->Result<bool,RegError>{
        if self.next_index + 1 > self.elements.len(){
            return Ok(false);
        }

        // Get the cell offset for a node list
        let cell_offset = self.elements[self.next_index];

        // Seek to offset
        reader.seek(
            SeekFrom::Start(HBIN_START_OFFSET + cell_offset as u64)
        )?;

        let cell = Cell::new(&mut reader)?;

        // Read cell
        self.current_cell = Some(
            Box::new(cell)
        );

        self.next_index += 1;

        Ok(true)
    }

    pub fn get_next_node<Rs: Read+Seek>(&mut self, mut reader: Rs)->Result<Option<NodeKey>,RegError>{
        loop {
            // Check if current index is within offset list range
            if self.next_index + 1 > self.elements.len() {
                return Ok(None);
            } else {
                // Check if we need to set the current cell
                if self.current_cell.is_none(){
                    match self.increment_current_cell(&mut reader)? {
                        false => {
                            // No more indexes
                            break;
                        },
                        true => {}
                    }
                }

                let mut get_next_cell_flag = false;
                match self.current_cell {
                    Some(ref mut current_cell) => {
                        match current_cell.data {
                            CellData::HashLeaf(ref mut hl) => {
                                let nk_option = hl.get_next_node(&mut reader)?;
                                match nk_option {
                                    Some(nk) => {
                                        return Ok(Some(nk));
                                    },
                                    None => {
                                        get_next_cell_flag = true
                                    }
                                }
                            },
                            _ => {
                                panic!("Unhandled cell data type: {:?}",current_cell.data);
                            }
                        }
                    },
                    None => {
                        panic!("self.current_cell should contain something...");
                    }
                }

                if get_next_cell_flag {
                    // No more nodes in the current list, lets go to the next list in the ri
                    match self.increment_current_cell(&mut reader)? {
                        false => {
                            // No more indexes
                            break;
                        },
                        true => {}
                    }
                }
            }
        }

        Ok(None)
    }
}

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

    pub fn set_value_list<Rs: Read+Seek>(&mut self, mut reader: Rs)->Result<bool,RegError>{
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

    pub fn set_sub_key_list<Rs: Read+Seek>(&mut self, mut reader: Rs)->Result<bool,RegError>{
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

    pub fn set_sec_key<Rs: Read+Seek>(&mut self, mut reader: Rs)->Result<bool,RegError>{
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

    pub fn get_next_value<Rs: Read+Seek>(&mut self, mut reader: Rs)->Result<Option<Cell>,RegError>{
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

    pub fn get_next_sub_key<Rs: Read+Seek>(&mut self, mut reader: Rs)->Result<Option<NodeKey>,RegError>{
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
