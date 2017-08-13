extern crate rwinreg;
use rwinreg::baseblock;
use std::io::BufReader;
use std::fs::File;

#[test]
fn baseblock_test_001() {
    let file = File::open(".testdata/NTUSER_0_4096_BASE_BLOCK.DAT").unwrap();
    let buf_reader = BufReader::new(file);
    let baseblock = baseblock::BaseBlock::new(buf_reader).unwrap();

    assert_eq!(baseblock.signature, 1718052210);
    assert_eq!(baseblock.primary_seq_num, 2810);
    assert_eq!(baseblock.secondary_seq_num, 2809);
    assert_eq!(baseblock.last_written.0, 130216723045201708);
    assert_eq!(baseblock.major_version, 1);
    assert_eq!(baseblock.minor_version, 3);
    assert_eq!(baseblock.file_type, 0);
    assert_eq!(baseblock.file_format, 1);
    assert_eq!(baseblock.root_cell_offset, 32);
    assert_eq!(baseblock.hive_bins_data_size, 3563520);
    assert_eq!(baseblock.clustering_factor, 1);
    assert_eq!(baseblock.file_name, "\\??\\C:\\Users\\Donald\\ntuser.dat");
    assert_eq!(baseblock.checksum, 1151707345);
    assert_eq!(baseblock.boot_type, 0);
    assert_eq!(baseblock.boot_recover, 0);
}
