use byteorder::{ReadBytesExt, LittleEndian};
use seek_bufread::BufReader;
use baseblock::BaseBlock;
use record::Record;
use hivebin::HiveBin;
use hivebin::{Cell,CellData};
use hivebin::{NodeKey};
use hivebin::{SecurityKey};
use rwinstructs::security::SecurityDescriptor;
use errors::RegError;
use std::io::Read;
use std::io::{Seek,SeekFrom};
use std::fs::File;

pub const HBIN_START_OFFSET: u64 = 4096;

pub fn has_hive_signature(filename: &str)->Result<bool,RegError>{
    let mut hive_fh = File::open(filename)?;
    let signature = hive_fh.read_u32::<LittleEndian>()?;
    if signature != 1718052210 {
        Ok(false)
    } else {
        Ok(true)
    }
}

#[derive(Serialize,Debug)]
pub struct Hive {
    #[serde(skip_serializing)]
    pub source: BufReader<File>,
    baseblock: BaseBlock,
    next_node_offset: u64,
    next_hbin_offset: u64,
    current_node: Option<NodeKey>,
    node_stack: Vec<NodeKey>,
    path_stack: Vec<String>
}
impl Hive {
    pub fn new(filename: &str) -> Result<Hive,RegError>{
        let hive_fh = File::open(filename)?;
        let mut source = BufReader::with_capacity(
            1048576,
            hive_fh
        );

        let baseblock = BaseBlock::new(
            &mut source
        )?;

        // First hbin will start at 4096
        let next_hbin_offset: u64 = HBIN_START_OFFSET;
        let next_node_offset: u64 = baseblock.get_root_offset() as u64;

        // Get first node
        let current_node = None;

        let node_stack: Vec<NodeKey> = Vec::new();
        let path_stack: Vec<String> = Vec::new();

        Ok(
            Hive {
                source: source,
                baseblock: baseblock,
                next_node_offset: next_node_offset,
                next_hbin_offset: next_hbin_offset,
                current_node: current_node,
                node_stack: node_stack,
                path_stack: path_stack
            }
        )
    }

    pub fn set_next_node_offset(&mut self, offset: u64){
        self.next_node_offset = offset - HBIN_START_OFFSET;
    }

    pub fn get_cell_at_offset(&mut self, offset: u64)->Result<Cell, RegError>{
        let absolute_offset = HBIN_START_OFFSET + offset as u64;

        // Seek to offset
        self.source.seek(
            SeekFrom::Start(absolute_offset)
        )?;

        Ok(
            Cell::new(&mut self.source, false)?
        )
    }

    pub fn get_next_hbin(&mut self)->Result<Option<HiveBin>,RegError>{
        // Get hbin
        let next_offset = self.next_hbin_offset;
        let hbin = match self.get_hbin_at_offset(next_offset){
            Ok(hbin) => hbin,
            Err(error) => {
                return Err(error)
            }
        };

        // Set next offset
        self.next_hbin_offset = self.next_hbin_offset + hbin.get_size() as u64;

        // Check if we have reached the end of hbin data
        if self.next_hbin_offset >= self.baseblock.hbin_size() as u64 + HBIN_START_OFFSET {
            // No more hbins
            Ok(None)
        } else {
            Ok(Some(hbin))
        }
    }

    fn get_hbin_at_offset(&mut self, absolute_offset: u64)->Result<HiveBin,RegError>{
        self.source.seek(
            SeekFrom::Start(absolute_offset)
        )?;

        Ok(
            HiveBin::new(&mut self.source)?
        )
    }

    fn get_full_path(&self)->String{
        self.path_stack.join("/")
    }
}

impl Iterator for Hive {
    type Item = Record;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_node.is_none() {
            let node_offset = self.next_node_offset as u64;
            let cell_key_node = match self.get_cell_at_offset(node_offset) {
                Ok(cell_key_node) => cell_key_node,
                Err(error) => {
                    panic!("{}",error);
                }
            };

            match cell_key_node.data {
                CellData::NodeKey(nk) => {
                    self.path_stack.push(
                        nk.get_name()
                    );
                    self.current_node = Some(
                        nk
                    );
                },
                _ => {
                    panic!("Unhandled CellData type for root key: {:?}",cell_key_node);
                }
            }
        }

        loop {
            let mut next_node: Option<NodeKey> = None;
            let fullpath = self.get_full_path();

            match self.current_node {
                Some(ref mut current_node) => {
                    // Check if security key has been read
                    if current_node.has_sec_key(){
                        if current_node.needs_sec_key() {
                            current_node.set_sec_key(
                                &mut self.source
                            );
                        }
                    }

                    // Check if the current node had values
                    if current_node.has_values(){
                        if current_node.needs_value_list() {
                            current_node.set_value_list(
                                &mut self.source
                            );
                        }

                        let value_cell = match current_node.get_next_value(&mut self.source) {
                            Ok(option) => {
                                match option{
                                    Some(cell) => {
                                        match cell.data {
                                            CellData::ValueKey(value) => {
                                                let descriptor: Option<SecurityDescriptor> = match current_node.get_sec_key() {
                                                    Some(sec_key) => {
                                                        Some(sec_key.get_descriptor())
                                                    },
                                                    None => None
                                                };
                                                let mut record = Record::new(
                                                    value,
                                                    descriptor
                                                );
                                                record.set_fullpath(
                                                    fullpath
                                                );
                                                return Some(record);
                                            },
                                            _ => {
                                                panic!("Unhandled cell data type: {:?}",cell);
                                            }
                                        }
                                    },
                                    None => {}
                                }
                            },
                            Err(error) => {
                                panic!("{}",error);
                            }
                        };
                    }

                    // Check if the current node has sub keys
                    if current_node.has_sub_keys(){
                        if current_node.needs_sub_key_list() {
                            current_node.set_sub_key_list(
                                &mut self.source
                            );
                        }

                        let sub_node = match current_node.get_next_sub_key(&mut self.source) {
                            Ok(sub_node) => sub_node,
                            Err(error) => {
                                panic!("{}",error);
                            }
                        };

                        if sub_node.is_some(){
                            let nk = sub_node.unwrap();
                            self.path_stack.push(
                                nk.get_name()
                            );
                            self.node_stack.push(
                                current_node.clone()
                            );

                            next_node = Some(nk);
                        } else {
                            next_node = self.node_stack.pop();
                            self.path_stack.pop();
                        }
                    } else {
                        next_node = self.node_stack.pop();
                        self.path_stack.pop();
                    }
                },
                None => {
                    break;
                }
            }

            self.current_node = next_node;
        }

        None
    }
}
