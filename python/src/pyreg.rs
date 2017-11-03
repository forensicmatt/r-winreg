use rpyfio::pyfio::PyFio;
use cpython::{Python, PyObject, PyResult, PyErr, PyString};
use std::io::{Read,Seek};
use std::cell::RefCell;

use rwinreg::hive::Hive;
use rwinreg::baseblock::BaseBlock;
use rwinreg::cell::Cell;
use rwinreg::cell::CellData;
use rwinreg::nk::NodeKey;
use rwinreg::record::Record;
use rwinreg::errors::RegError;
use serde_json;

pub const HBIN_START_OFFSET: u64 = 4096;

py_module_initializer!(pyrreg, initpyrreg, PyInit_pyrreg, |py, m| {
    m.add(py, "__doc__", "Module documentation string")?;
    m.add_class::<RegClass>(py)?;
    Ok(())
});

py_class!(class RegClass | py | {
    data hive: RefCell<PyHive>;

    def __new__(_cls, fileio: PyObject) -> PyResult<RegClass> {
        let py_fio = PyFio::new(
            fileio
        );

        let hive = PyHive::from_source(
            py_fio
        ).unwrap();

        RegClass::create_instance(
            py, RefCell::new(hive)
        )
    }

    def test(&self) -> PyResult<PyString> {
        let gil = Python::acquire_gil();
        let py = gil.python();
        Ok(PyString::new(py, "test"))
    }

    def get_next_record(&self) -> PyResult<Option<PyString>> {
        let gil = Python::acquire_gil();
        let py = gil.python();

        let mut mut_hive = self.hive(py).borrow_mut();
        let result = mut_hive.get_next_value();

        match result {
            Ok(option) => {
                match option {
                    Some(record) => {
                        let json_str = match serde_json::to_string(&record) {
                            Ok(js) => js,
                            Err(err) => {
                                return Ok(
                                    Some(PyString::new(py, "Error")));
                            }
                        };
                        return Ok(
                            Some(PyString::new(py, &json_str)));
                    },
                    None => {
                        return Ok(None);
                    }
                }
            },
            Err(error) => {
                return Ok(
                    Some(PyString::new(py, &format!("Error: {:?}",error))));
            }
        }

        Ok(None)
    }
});

#[derive(Debug)]
pub struct PyHive {
    source: PyFio,
    baseblock: Box<BaseBlock>,
    node_stack: Vec<Box<NodeKey>>,
    current_path: String
}

impl PyHive {
    pub fn from_source(mut source: PyFio) -> Result<PyHive,RegError>{
        let mut buffer_baseblock = [0; 4096];
        source.read_exact(&mut buffer_baseblock)?;

        let baseblock = BaseBlock::new(
            &buffer_baseblock, 0
        )?;

        Ok(
            PyHive {
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
                    // debug!("[{}] key: {}",index,key.key_name());
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
