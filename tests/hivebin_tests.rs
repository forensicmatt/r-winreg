extern crate rwinreg;
use rwinreg::hivebin;
use std::io::BufReader;
use std::fs::File;

#[test]
fn hivebin_header_test_001() {
    let file = File::open(".testdata/NTUSER_4096_32_HIVEBIN_HEADER.DAT").unwrap();
    let buf_reader = BufReader::new(file);
    let hb_header = hivebin::HiveBinHeader::new(buf_reader).unwrap();

    assert_eq!(hb_header.signature, 1852400232);
    assert_eq!(hb_header.hb_offset, 0);
    assert_eq!(hb_header.size, 4096);
    assert_eq!(hb_header.reserved1, 0);
    assert_eq!(hb_header.timestamp.0, 130216723045201708);
    assert_eq!(hb_header.spare, 0);
}
