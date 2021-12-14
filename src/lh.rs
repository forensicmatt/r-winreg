use byteorder::{ByteOrder,LittleEndian};
use errors::RegError;
use hive::HBIN_START_OFFSET;
use cell::Cell;
use cell::CellData;
use nk::NodeKey;
use std::io::{Read,Seek};
use serde::Serialize;

#[derive(Serialize, Debug)]
struct HashElement(u32,u32);
impl HashElement {
    pub fn new(buffer: &[u8]) -> HashElement {
        HashElement(
            LittleEndian::read_u32(&buffer[0..4]),
            LittleEndian::read_u32(&buffer[4..8])
        )
    }

    pub fn get_offset(&self)->&u32{
        &self.0
    }
}

// lh
#[derive(Serialize, Debug)]
pub struct HashLeaf{
    #[serde(skip_serializing)]
    _offset: u64,
    signature: u16,
    element_count: u16,
    elements: Vec<HashElement>,
    next_index: usize
}
impl HashLeaf{
    pub fn new(buffer: &[u8], offset: u64) -> Result<HashLeaf,RegError> {
        let signature = LittleEndian::read_u16(&buffer[0..2]);
        let element_count = LittleEndian::read_u16(&buffer[2..4]);
        let mut elements: Vec<HashElement> = Vec::new();
        let next_index: usize = 0;

        for i in 0..element_count {
            let o = (4 + (i*8)) as usize;
            let element = HashElement::new(
                &buffer[o..o+8]
            );
            elements.push(element);
        }

        Ok(
            HashLeaf{
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
            let cell_offset = (*self.elements[self.next_index].get_offset() as u64) + HBIN_START_OFFSET;
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
