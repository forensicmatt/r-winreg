use byteorder::{ByteOrder,LittleEndian};
use errors::RegError;
use hive::HBIN_START_OFFSET;
use cell::Cell;
use cell::CellData;
use nk::NodeKey;
use std::io::{Read,Seek};
use serde::Serialize;

// ri
#[derive(Serialize, Debug)]
pub struct RootIndex{
    #[serde(skip_serializing)]
    _offset: u64,
    signature: u16,
    element_count: u16,
    elements: Vec<u32>,
    next_index: usize,
    current_cell_data: Option<Box<CellData>>
}
impl RootIndex{
    pub fn new(buffer: &[u8], offset: u64) -> Result<RootIndex,RegError> {
        let signature = LittleEndian::read_u16(&buffer[0..2]);
        let element_count = LittleEndian::read_u16(&buffer[2..4]);
        let mut elements: Vec<u32> = Vec::new();
        let next_index: usize = 0;
        let current_cell_data = None;

        for i in 0..element_count {
            let o = (4 + (i*4)) as usize;
            let element = LittleEndian::read_u32(&buffer[o..o+4]);
            elements.push(element);
        }

        Ok(
            RootIndex{
                _offset: offset,
                signature: signature,
                element_count: element_count,
                elements: elements,
                next_index: next_index,
                current_cell_data: current_cell_data
            }
        )
    }

    pub fn increment_current_cell_data<Rs: Read+Seek>(&mut self, reader: &mut Rs)->Result<bool,RegError>{
        if self.next_index + 1 > self.elements.len() {
            return Ok(false);
        }

        // Get the cell offset for a node list
        let cell_offset = self.elements[self.next_index] as u64 + HBIN_START_OFFSET;

        let cell = Cell::at_offset(
            reader, cell_offset
        )?;

        // Read cell
        self.current_cell_data = Some(
            Box::new(cell.get_data()?)
        );

        self.next_index += 1;

        Ok(true)
    }

    pub fn get_next_key<Rs: Read+Seek>(&mut self, reader: &mut Rs)->Result<Option<NodeKey>,RegError>{
        loop {
            if self.next_index + 1 > self.elements.len() {
                // No more lists to iterate through
                return Ok(None);
            }

            // Check if we need to set the current cell
            if self.current_cell_data.is_none(){
                match self.increment_current_cell_data(reader)? {
                    false => {
                        // No more indexes
                        break;
                    },
                    true => {}
                }
            }

            let mut get_next_cell_flag = false;
            match self.current_cell_data {
                Some(ref mut cell_data) => {
                    match **cell_data {
                        CellData::HashLeaf(ref mut hl) => {
                            let nk_option = hl.get_next_key(reader)?;
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
                            panic!("Unhandled cell data type: {:?}",cell_data);
                        }
                    }
                }
                None => {}
            }

            if get_next_cell_flag {
                // No more nodes in the current list, lets go to the next list in the ri
                match self.increment_current_cell_data(reader)? {
                    false => {
                        // No more indexes
                        return Ok(None);
                    },
                    true => {
                        continue;
                    }
                }
            }
        }

        Ok(None)
    }
}
