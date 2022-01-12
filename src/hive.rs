use byteorder::{ReadBytesExt,LittleEndian};
use baseblock::BaseBlock;
use record::Record;
use cell::Cell;
use cell::CellData;
use nk::NodeKey;
use errors::RegError;
use std::fs::File;
use std::io::Read;
use std::io::Seek;
use serde::Serialize;

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
pub struct Hive<Rs> {
    #[serde(skip_serializing)]
    source: Rs,
    baseblock: Box<BaseBlock>,
    node_stack: Vec<Box<NodeKey>>,
    current_path: String
}
impl <Rs: Read + Seek> Hive<Rs> {
    pub fn from_source(mut source: Rs) -> Result<Hive<Rs>,RegError>{
        let mut buffer_baseblock = [0; 4096];
        source.read_exact(&mut buffer_baseblock)?;

        let baseblock = BaseBlock::new(&buffer_baseblock,0)?;

        Ok(
            Hive {
                source: source,
                baseblock: Box::new(baseblock),
                node_stack: Vec::new(),
                current_path: String::from("")
            }
        )
    }

    /// Get the root node.
    ///
    /// # Examples
    ///
    /// Get the root node of a hive.
    ///
    /// ```
    /// use std::fs::File;
    /// use rwinreg::hive::Hive;
    ///
    /// # fn test_get_root_node() {
    /// let mut file = File::open(".testdata/NTUSER.DAT").unwrap();
    ///
    /// let mut hive = match Hive::from_source(file){
    ///     Ok(h) => h,
    ///     Err(e) => panic!(e)
    /// };
    ///
    /// let node = match hive.get_root_node(){
    ///     Ok(n) => n,
    ///     Err(e) => panic!(e)
    /// };
    /// assert_eq!(node.key_name(), "CsiTool-CreateHive-{00000000-0000-0000-0000-000000000000}");
    /// # }
    /// ```
    pub fn get_root_node(&mut self)->Result<NodeKey, RegError>{
        let offset = self.baseblock.root_cell_offset() as u64 + HBIN_START_OFFSET;
        match Cell::at_offset(&mut self.source, offset)?.get_data()? {
            CellData::NodeKey(nk) => Ok(nk),
            other => {
                panic!("Root node is not NodeKey: {:?}",other)
            }
        }
    }

    pub fn get_next_value(&mut self)->Result<Option<Record>, RegError>{
        if self.node_stack.len() == 0 {
            let node = self.get_root_node()?;

            self.current_path.push_str(&format!("\\{}",node.key_name()));

            self.node_stack.push(
                Box::new(node)
            );
        }

        loop {
            let index = self.node_stack.len() - 1;
            match self.node_stack[index].get_next_value(&mut self.source)?{
                Some(mut vk) => {
                    vk.read_value(&mut self.source)?;
                    let record = Record::new(
                        &self.current_path,
                        &self.node_stack[index],
                        vk
                    );
                    return Ok(
                        Some(record)
                    );
                },
                None => {}
            }

            match self.node_stack[index].get_next_key(&mut self.source)?{
                Some(mut key) => {
                    debug!("[{}] key: {}",index,key.key_name());
                    key.set_security_key(&mut self.source)?;
                    self.current_path.push_str(
                        &format!("\\{}",key.key_name())
                    );
                    self.node_stack.push(
                        Box::new(key)
                    );
                },
                None => {
                    self.node_stack.pop();

                    let new_path = match self.current_path.rfind("\\"){
                        Some(index) => {
                            self.current_path[0..index].to_string()
                        },
                        None => "".to_string()
                    };
                    self.current_path = new_path;

                    if self.node_stack.len() == 0 {
                        break;
                    }
                }
            };
        }

        Ok(None)
    }
}
