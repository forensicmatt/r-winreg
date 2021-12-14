use byteorder::{ByteOrder,ReadBytesExt,LittleEndian};
use errors::RegError;
use hive::HBIN_START_OFFSET;
use cell::Cell;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use serde::Serialize;

// db
#[derive(Serialize, Debug)]
pub struct DataBlock{
    #[serde(skip_serializing)]
    _offset: u64,
    signature: u16,
    segment_count: u16,
    segments_offset: u32
}
impl DataBlock{
    pub fn new(buffer: &[u8], offset: u64)->Result<DataBlock,RegError> {
        let signature = LittleEndian::read_u16(&buffer[0..2]);
        let segment_count = LittleEndian::read_u16(&buffer[2..4]);
        let segments_offset = LittleEndian::read_u32(&buffer[4..8]);

        Ok(
            DataBlock{
                _offset: offset,
                signature: signature,
                segment_count: segment_count,
                segments_offset: segments_offset
            }
        )
    }

    pub fn get_data<Rs: Read+Seek>(&self, reader: &mut Rs)->Result<Vec<u8>,RegError> {
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
        let _list_cell_size = reader.read_i32::<LittleEndian>()?;

        // read offsets into the segments_list
        for i in 0..self.segment_count {
            let offset = reader.read_u32::<LittleEndian>()?;
            debug!("DataBlock<{}> segment offset {}: {}",self._offset,i,offset);
            segments_list.push(
                offset
            );
        }

        for segment_offset in segments_list {
            // Read cell
            let mut cell = Cell::at_offset(
                reader, segment_offset as u64 + HBIN_START_OFFSET
            )?;

            raw_data.append(
                &mut cell.data
            );
        }

        Ok(raw_data)
    }
}
