extern crate rwinreg;
use rwinreg::hive::Hive;
use std::fs::File;

fn get_root_node() {
    let file = File::open(".testdata/NTUSER.DAT").unwrap();

    let mut hive = match Hive::from_source(file){
        Ok(h) => h,
        Err(e) => panic!("{}", e)
    };

    let node = match hive.get_root_node(){
        Ok(n) => n,
        Err(e) => panic!("{}", e)
    };

    println!("{:?}",node);
}

fn main(){
    get_root_node();
}
