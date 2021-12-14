use byteorder::{ByteOrder,LittleEndian};
use errors::RegError;
use hive::HBIN_START_OFFSET;
use cell::Cell;
use cell::CellData;
use nk::NodeKey;
use std::io::{Read,Seek};
use serde::Serialize;

// li
#[derive(Serialize, Debug)]
pub struct IndexLeaf{
    #[serde(skip_serializing)]
    _offset: u64,
    signature: u16,
    element_count: u16,
    elements: Vec<u32>,
    next_index: usize
}

impl IndexLeaf{
    pub fn new(buffer: &[u8], offset: u64) -> Result<IndexLeaf,RegError> {
        let signature = LittleEndian::read_u16(&buffer[0..2]);
        let element_count = LittleEndian::read_u16(&buffer[2..4]);
        let mut elements: Vec<u32> = Vec::new();
        let next_index: usize = 0;

        for i in 0..element_count {
            let o = (4 + (i*4)) as usize;
            let element = LittleEndian::read_u32(&buffer[o..o+4]);
            elements.push(element);
        }

        Ok(
            IndexLeaf{
                _offset: offset,
                signature: signature,
                element_count: element_count,
                elements: elements,
                next_index: next_index
            }
        )
    }

    pub fn get_next_key<Rs: Read+Seek>(&mut self, reader: &mut Rs)->Result<Option<NodeKey>,RegError>{
        if self.next_index >= self.elements.len(){
            self.next_index = 0;
            Ok(None)
        }
        else {
            let cell_offset = (self.elements[self.next_index] as u64) + HBIN_START_OFFSET;
            match Cell::at_offset(reader,cell_offset)?.get_data()?{
                CellData::NodeKey(nk)=>{
                    self.next_index += 1;
                    Ok(Some(nk))
                },
                other => panic!("CellData is not type NodeKey: {:?}",other)
            }
        }
    }
}
