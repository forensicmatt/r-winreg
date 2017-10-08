use errors::{RegError};
use cell::{Cell};
use rwinstructs::timestamp::{WinTimestamp};
use byteorder::{ReadBytesExt, LittleEndian};
use std::io::Read;
use std::io::{Seek,SeekFrom};
use std::io::{Cursor};


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
