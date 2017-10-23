#[macro_use] extern crate log;
extern crate serde_json;
extern crate env_logger;
extern crate clap;
extern crate rwinreg;
use rwinreg::hive;
use clap::{App, Arg};
use std::fs;
use std::fs::File;

fn process_directory(directory: &str) {
    for dir_reader in fs::read_dir(directory) {
        for entry_result in dir_reader {
            match entry_result {
                Ok(entry) => {
                    let path = entry.path();
                    if path.is_file() {
                        let path_string = path.into_os_string().into_string().unwrap();
                        match hive::has_hive_signature(&path_string){
                            Ok(result) => {
                                match result {
                                    true => {
                                        process_file(&path_string);
                                    },
                                    false => {
                                        debug!("{} is not a hive file.",path_string);
                                    }
                                }
                            },
                            Err(error) => {
                                error!("Error testing signature for {} [{:?}]",path_string,error);
                            }
                        }
                    } else if path.is_dir(){
                        let path_string = path.into_os_string().into_string().unwrap();
                        process_directory(&path_string);
                    }
                },
                Err(error) => {
                    error!("Error reading {} [{:?}]",directory,error);
                }
            }
        }
    }
}

fn process_file(filename: &str) -> bool {
    info!("processing file: {}",filename);

    let hive_fh = match File::open(filename){
        Ok(fh) => fh,
        Err(error) => {
            error!("{} [error: {}]", filename, error);
            return false;
        }
    };

    let mut hive = match hive::Hive::from_source(hive_fh) {
        Ok(hive) => hive,
        Err(error) => {
            error!("{} [error: {}]", filename, error);
            return false;
        }
    };

    loop {
        let record = match hive.get_next_value(){
            Ok(option) => {
                match option {
                    Some(record) => record,
                    None => {
                        break;
                    }
                }
            },
            Err(error) => {
                panic!("error: {}",error);
            }
        };
        let json_str = serde_json::to_string(&record).unwrap();
        println!("{}",json_str);
    }

    // for value in hive {
    //     let json_str = serde_json::to_string(&value).unwrap();
    //     println!("{}",json_str);
    // }

    return true;
}

fn is_directory(source: &str)->bool{
    let metadata = fs::metadata(source).unwrap();

    let file_type = metadata.file_type();

    file_type.is_dir()
}

fn main() {
    env_logger::init().unwrap();

    let source_arg = Arg::with_name("source")
        .short("s")
        .long("source")
        .value_name("PATH")
        .help("Source.")
        .required_unless("pipe")
        .takes_value(true);

    let options = App::new("reg_parser")
        .version("for debug")
        .author("Matthew Seyer <https://github.com/forensicmatt/r-winreg>")
        .about("Registry Parser written in Rust.")
        .arg(source_arg)
        .get_matches();

    let source = options.value_of("source").unwrap();

    if is_directory(source) {
        process_directory(source);
    } else {
        process_file(source);
    }
}
