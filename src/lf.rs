use byteorder::{ByteOrder,LittleEndian};
use errors::RegError;
use utils;
use hive::HBIN_START_OFFSET;
use cell::Cell;
use cell::CellData;
use nk::NodeKey;
use std::io::{Read,Seek};

#[derive(Serialize, Debug)]
struct FastElement(u32,String);
impl FastElement {
    pub fn new(buffer: &[u8]) -> Result<FastElement,RegError> {
        Ok(
            FastElement(
                LittleEndian::read_u32(&buffer[0..4]),
                utils::read_ascii(&buffer[4..8])?
            )
        )
    }

    pub fn get_offset(&self)->&u32{
        &self.0
    }
}

// lf
#[derive(Serialize, Debug)]
pub struct FastLeaf{
    #[serde(skip_serializing)]
    _offset: u64,
    signature: u16,
    element_count: u16,
    elements: Vec<FastElement>,
    next_index: usize
}

impl FastLeaf{
    pub fn new(buffer: &[u8], offset: u64) -> Result<FastLeaf,RegError> {
        let signature = LittleEndian::read_u16(&buffer[0..2]);
        let element_count = LittleEndian::read_u16(&buffer[2..4]);
        let mut elements: Vec<FastElement> = Vec::new();
        let next_index: usize = 0;

        for i in 0..element_count {
            let o = (4 + (i*8)) as usize;
            let element = FastElement::new(
                &buffer[o..o+8]
            )?;
            elements.push(element);
        }

        Ok(
            FastLeaf{
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
