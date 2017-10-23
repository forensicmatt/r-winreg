use vk::ValueKey;
use nk::NodeKey;
use rwinstructs::security::SecurityDescriptor;
use rwinstructs::timestamp::{WinTimestamp};

#[derive(Serialize,Debug)]
pub struct Record {
    pub fullpath: String,
    pub nk_last_written: WinTimestamp,
    pub valuekey: ValueKey,
    pub security: Option<Box<SecurityDescriptor>>
}
impl Record {
    pub fn new(path: &str, nk: &NodeKey, vk: ValueKey)->Record{
        let mut fullpath = path.to_string();
        fullpath.push_str(&format!("\\{}",vk.get_name()));
        let mut security = None;

        let nk_last_written = nk.get_last_written().clone();

        match *nk.get_security_key() {
            Some(ref sk) => {
                security = Some(
                    Box::new(
                        sk.get_descriptor().clone()
                    )
                )
            },
            None => {}
        }

        Record {
            fullpath: fullpath,
            nk_last_written: nk_last_written,
            valuekey: vk,
            security: security
        }
    }
}
