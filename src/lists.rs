use errors::{RegError};
use cell::{Cell,CellData};
use nk::{NodeKey};
use hive::{HBIN_START_OFFSET};

use byteorder::{ReadBytesExt, LittleEndian};
use std::io::Read;
use std::io::{Seek,SeekFrom};

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
    pub fn new<Rs: Read+Seek>(
        mut reader: Rs, offset: u64
    ) -> Result<HashLeaf,RegError> {
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

    pub fn get_next_node<Rs: Read+Seek>(
        &mut self, mut reader: Rs
    )->Result<Option<NodeKey>,RegError>{
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

    pub fn get_next_node<Rs: Read+Seek>(
        &mut self, mut reader: Rs
    )->Result<Option<NodeKey>,RegError>{
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

    pub fn get_next_node<Rs: Read+Seek>(
        &mut self, mut reader: Rs
    )->Result<Option<NodeKey>,RegError>{
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

    pub fn get_next_node<Rs: Read+Seek>(
        &mut self, mut reader: Rs
    )->Result<Option<NodeKey>,RegError>{
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
