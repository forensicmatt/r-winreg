use seek_bufread::BufReader;
use baseblock::BaseBlock;
use hivebin::HiveBin;
use hivebin::{Cell,CellData};
use hivebin::{NodeKey};
use errors::RegError;
use std::io::Read;
use std::io::{Seek,SeekFrom};
use std::fs::File;

pub const HBIN_START_OFFSET: u64 = 4096;

#[derive(Serialize,Debug)]
pub struct Hive {
    #[serde(skip_serializing)]
    pub source: BufReader<File>,
    baseblock: BaseBlock,
    next_node_offset: u64,
    next_hbin_offset: u64,
    current_node: Option<Box<NodeKey>>,
    parent_node: Option<Box<NodeKey>>
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
        let parent_node = None;

        Ok(
            Hive {
                source: source,
                baseblock: baseblock,
                next_node_offset: next_node_offset,
                next_hbin_offset: next_hbin_offset,
                current_node: current_node,
                parent_node: parent_node
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

    pub fn get_next_value(&mut self)->Result<Option<Cell>,RegError>{

        Ok(None)
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

    fn set_current_node(&mut self, nk: NodeKey){
        self.current_node = Some(Box::new(nk));
    }
}

impl Iterator for Hive {
    type Item = Cell;

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
                    self.current_node = Some(
                        Box::new(nk)
                    );
                },
                _ => {
                    panic!("Unhandled CellData type for root key: {:?}",cell_key_node);
                }
            }
        }

        loop {
            let mut next_node: Option<Box<NodeKey>> = None;
            match self.current_node {
                Some(ref mut current_node) => {
                    println!(
                        "node name: {}; offset: {}",
                        current_node.key_name,
                        current_node.get_offset()
                    );

                    // Check if the current node had values
                    if current_node.has_values(){
                        if current_node.needs_value_list() {
                            current_node.set_value_list(
                                &mut self.source
                            );
                        }

                        let value_cell = match current_node.get_next_value(&mut self.source) {
                            Ok(value_cell) => value_cell,
                            Err(error) => {
                                panic!("{}",error);
                            }
                        };

                        match value_cell{
                            Some(cell) => {
                                return Some(cell);
                            },
                            None => {}
                        }
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

                        match sub_node {
                            Some(nk) => {
                                next_node = Some(Box::new(nk));
                            },
                            None => {}
                        }
                    }
                },
                None => {}
            }

            if next_node.is_some() {
                self.parent_node = self.current_node.clone();
                self.current_node = next_node;
            } else {
                self.current_node = self.parent_node.clone();
            }
        }

        None
    }
}
