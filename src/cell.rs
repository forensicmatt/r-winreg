use byteorder::{ReadBytesExt,ByteOrder,LittleEndian};
use nk::NodeKey;
use vk::ValueKey;
use sk::SecurityKey;
use lf::FastLeaf;
use lh::HashLeaf;
use li::IndexLeaf;
use ri::RootIndex;
use db::DataBlock;
use errors::RegError;
use serde::ser;
use std::io::Read;
use std::io::{Seek,SeekFrom};
use std::fmt;
use serde::Serialize;

#[derive(Serialize, Debug)]
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

pub struct CellSignature(u16);
impl CellSignature {
    pub fn new(value: u16) -> CellSignature {
        CellSignature(value)
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
            _ => format!("0x{:04x}",self.0)
        }
    }

    pub fn as_u16(&self)->u16{
        self.0
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

#[derive(Serialize, Debug)]
pub struct Cell{
    #[serde(skip_serializing)]
    _offset: u64,
    pub size: i32,
    pub data: Vec<u8>
}
impl Cell {
    pub fn new<R: Read>(reader: &mut R, offset: u64) -> Result<Cell,RegError> {
        let size = reader.read_i32::<LittleEndian>()?;
        let mut data = vec![0; (size.abs() - 4) as usize];
        reader.read_exact(
            data.as_mut_slice()
        )?;

        Ok(
            Cell {
                _offset: offset,
                size: size,
                data: data
            }
        )
    }

    /// Get a cell at a given absolute offset.
    ///
    /// # Examples
    ///
    /// Getting a cell at a certain absolute offset.
    ///
    /// ```
    /// use std::fs::File;
    /// use rwinreg::cell::Cell;
    /// use rwinreg::hive::HBIN_START_OFFSET;
    ///
    /// # fn test_get_cell_at_offset() {
    /// let mut f = match File::open(".testdata/NTUSER.DAT"){
    ///     Ok(f) => f,
    ///     Err(e) => panic!(e)
    /// };
    /// let cell = match Cell::at_offset(&mut f, 32 + HBIN_START_OFFSET) {
    ///     Ok(c) => c,
    ///     Err(e) => panic!(e)
    /// };
    /// # }
    /// ```
    pub fn at_offset<Rs: Read+Seek>(reader: &mut Rs, offset: u64) -> Result<Cell,RegError> {
        // Seek to offset
        reader.seek(
            SeekFrom::Start(offset)
        )?;
        let size = reader.read_i32::<LittleEndian>()?;
        debug!("cell at offset {} with size {}",offset,size);
        let mut data = vec![0; (size.abs() - 4) as usize];
        reader.read_exact(
            data.as_mut_slice()
        )?;

        Ok(
            Cell {
                _offset: offset,
                size: size,
                data: data
            }
        )
    }

    pub fn get_signature(&self)->CellSignature{
        CellSignature::new(
            LittleEndian::read_u16(&self.data[0..2])
        )
    }

    pub fn get_data(&self)->Result<CellData,RegError>{
        match self.get_signature().as_u16() {
            26220 => { //lf
                Ok(
                    CellData::FastLeaf(
                        FastLeaf::new(&self.data[..], self._offset + 4)?
                    )
                )
            },
            26732 => { //lh
                Ok(
                    CellData::HashLeaf(
                        HashLeaf::new(&self.data[..], self._offset + 4)?
                    )
                )
            },
            26988 => { //li
                Ok(
                    CellData::IndexLeaf(
                        IndexLeaf::new(&self.data[..], self._offset + 4)?
                    )
                )
            },
            26994 => { //ri
                Ok(
                    CellData::RootIndex(
                        RootIndex::new(&self.data[..], self._offset + 4)?
                    )
                )
            },
            27502 => { //nk
                Ok(
                    CellData::NodeKey(
                        NodeKey::new(&self.data[..], self._offset + 4)?
                    )
                )
            },
            27507 => { //sk
                Ok(
                    CellData::SecurityKey(
                        SecurityKey::new(&self.data[..], self._offset + 4)?
                    )
                )

            },
            27510 => { //vk
                Ok(
                    CellData::ValueKey(
                        ValueKey::new(&self.data[..], self._offset + 4)?
                    )
                )
            },
            25188 => { //db
                Ok(
                    CellData::DataBlock(
                        DataBlock::new(&self.data[..], self._offset + 4)?
                    )
                )
            },
            _ => {
                Ok(
                    CellData::Raw(
                        self.data.to_vec()
                    )
                )
            }
        }
    }

    pub fn get_raw_data(&self)->CellData {
        CellData::Raw(
            self.data.to_vec()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::io::Read;
    use std::fs::File;

    #[test]
    fn cell() {
        let mut file = File::open(".testdata/NTUSER_4680_40_CELL_VK.DAT").unwrap();
        let mut buffer = Vec::new();

        match file.read_to_end(&mut buffer){
            Err(error)=>panic!("{:?}",error),
            _ => {}
        }

        let cell = match Cell::new(&mut Cursor::new(&buffer),0){
            Ok(cell)=>cell,
            Err(error)=>panic!("{:?}",error)
        };

        assert_eq!(cell.size, -40);
        assert_eq!(cell.get_signature().as_string(), String::from("vk"));

        let known_data: &[u8] = &[
            0x76,0x6B,0x0A,0x00,0x54,0x00,0x00,0x00,0x80,0x30,0x00,0x00,0x01,0x00,0x00,0x00,
            0x01,0x00,0x72,0x65,0x55,0x73,0x65,0x72,0x20,0x41,0x67,0x65,0x6E,0x74,0x00,0x00,
            0x6C,0x66,0x01,0x00
        ];
        assert_eq!(&cell.data[..], known_data);
    }
}
