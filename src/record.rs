use hivebin::{ValueKey};
use rwinstructs::security::SecurityDescriptor;

#[derive(Serialize,Debug)]
pub struct Record {
    fullpath: String,
    security: Option<SecurityDescriptor>,
    value: ValueKey
}
impl Record {
    pub fn new(value: ValueKey, sec_descriptor: Option<SecurityDescriptor>)->Record{
        let fullpath = "".to_string();

        Record {
            fullpath: fullpath,
            security: sec_descriptor,
            value: value,
        }
    }

    pub fn set_fullpath(&mut self, path: String){
        let name = self.value.get_name();
        self.fullpath = vec![path,self.value.get_name()].join("/");
    }
}
