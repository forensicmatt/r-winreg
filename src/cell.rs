use errors::{RegError};
// Cell Types
use lists::{DataBlock,HashLeaf,FastLeaf,IndexLeaf,RootIndex};
use vk::{ValueKey};
use nk::{NodeKey};
use sk::{SecurityKey};

use serde::{ser};
use byteorder::{ReadBytesExt, LittleEndian};
use std::io::Read;
use std::io::{Seek,SeekFrom};
use std::io::{Cursor};
use std::fmt;

#[derive(Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum CellData{
    Raw(Vec<u8>),
    IndexLeaf(IndexLeaf),
    FastLeaf(FastLeaf),
    HashLeaf(HashLeaf),
    RootIndex(RootIndex),
    NodeKey(NodeKey),
    ValueKey(ValueKey),
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
                let mut value_key = ValueKey::new(
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

    pub fn from_value_key<Rs: Read+Seek>(
        mut reader: Rs, value_key: &ValueKey
    ) -> Result<Cell,RegError> {
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
