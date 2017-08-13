extern crate rwinreg;
use rwinreg::baseblock;
use std::fs::File;

fn test_01() {
    let mut file = File::open(".testdata/NTUSER_0_4096_BASE_BLOCK.DAT").unwrap();
    let baseblock = baseblock::BaseBlock::new(file).unwrap();
    println!("{:#?}",baseblock);
}

fn main(){
    println!("{:?}",std::env::current_dir());
    test_01();
}
