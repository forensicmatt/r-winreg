extern crate rwinreg;
extern crate serde_json;
use rwinreg::hivebin;
use std::fs::File;

fn test_nk_01() {
    let file = File::open(".testdata/NTUSER_4128_144_CELL_NK.DAT").unwrap();

    let key_node = hivebin::Cell::new(file).unwrap();
    println!("{:#?}",key_node);

    let json_str = serde_json::to_string(&key_node).unwrap();
    println!("{}",json_str);
}

fn main(){
    test_nk_01();
}
