#[macro_use] extern crate log;
extern crate serde_json;
extern crate env_logger;
extern crate clap;
extern crate rwinreg;
use rwinreg::hive;
use clap::{App, Arg};
use std::fs;

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

    let hive = match hive::Hive::new(filename) {
        Ok(hive) => hive,
        Err(error) => {
            error!("{} [error: {}]", filename, error);
            return false;
        }
    };

    for value in hive {
        let json_str = serde_json::to_string(&value).unwrap();
        println!("{}",json_str);
    }

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

    let options = App::new("RusyReg")
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
