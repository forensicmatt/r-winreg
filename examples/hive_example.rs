extern crate rwinreg;
extern crate env_logger;
extern crate serde_json;
extern crate seek_bufread;
use rwinreg::hive;
use rwinreg::hive::Hive;

fn test_hive_get_key(){
    let mut hive = Hive::new(".testdata/NTUSER.DAT").unwrap();
    let key_name = "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\RunMRU";

}

fn test_hive_01() {
    let mut hive = Hive::new(".testdata/NTUSER.DAT").unwrap();
    println!("{:#?}",hive);
    println!("{}",serde_json::to_string(&hive).unwrap());

    let mut hbin_result = hive.get_next_hbin();
    loop {
        match hbin_result {
            Ok(possible_hbin) => {
                match possible_hbin {
                    Some(hbin) => {
                        println!("{}",serde_json::to_string(&hbin).unwrap());
                    },
                    None => {
                        break;
                    }
                }
            },
            Err(error) => {
                println!("{:?}",error);
            }
        }
            hbin_result = hive.get_next_hbin();
    }
}

fn test_hive_02() {
    let mut hive = Hive::new(".testdata/NTUSER.DAT").unwrap();
    println!("{}",serde_json::to_string(&hive).unwrap());

    for value in hive {
        println!("{}",serde_json::to_string(&value).unwrap());
    }
}

fn test_sig_01() {
    let result1 = hive::has_hive_signature(".testdata/NTUSER.DAT").unwrap();
    println!("result: {}",result1);

    let result2 = hive::has_hive_signature(".testdata/NTUSER_4096_4096_HIVEBIN.DAT").unwrap();
    println!("result: {}",result2);
}

fn main(){
    env_logger::init().unwrap();
    // test_hive_get_key();
    // test_sig_01();
    test_hive_02();
    // test_hive_01();
}
