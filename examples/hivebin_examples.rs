extern crate rwinreg;
extern crate env_logger;
extern crate serde_json;
use rwinreg::hivebin;
use rwinreg::cell::{Cell};
use std::fs::File;

fn hivebin_example_01() {
    let file = File::open(".testdata/NTUSER_4096_4096_HIVEBIN.DAT").unwrap();
    let hbin = hivebin::HiveBin::new(file).unwrap();
    println!("{:#?}",hbin);
    let json_str = serde_json::to_string(&hbin).unwrap();
    println!("{}",json_str);
}

fn test_nk_01() {
    let file = File::open(".testdata/NTUSER_4128_144_CELL_NK.DAT").unwrap();

    let key_node = Cell::new(file).unwrap();
    println!("{:#?}",key_node);

    let json_str = serde_json::to_string(&key_node).unwrap();
    println!("{}",json_str);
}

fn test_vk_01() {
    let file = File::open(".testdata/NTUSER_4680_40_CELL_VK.DAT").unwrap();

    let key_node = Cell::new(file).unwrap();
    println!("{:#?}",key_node);

    let json_str = serde_json::to_string(&key_node).unwrap();
    println!("{}",json_str);
}

fn main(){
    env_logger::init().unwrap();
    // test_nk_01();
    // test_vk_01();
    hivebin_example_01();
}
