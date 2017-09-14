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

#[test]
fn hivebin_cell_nk_001() {
    let file = File::open(".testdata/NTUSER_4128_144_CELL_NK.DAT").unwrap();
    let buf_reader = BufReader::new(file);
    let cell = hivebin::Cell::new(buf_reader, false).unwrap();

    assert_eq!(cell.size, -144);
    let signature = cell.signature.unwrap();
    assert_eq!(signature.0, 27502);
    match cell.data {
        hivebin::CellData::NodeKey(data) => {
            assert_eq!(data.flags.bits(), 44);
            assert_eq!(data.last_written.0, 130269705849853298);
            assert_eq!(data.access_bits, 2);
            assert_eq!(data.offset_parent_key, 1928);
            assert_eq!(data.num_sub_keys, 13);
            assert_eq!(data.num_volatile_sub_keys, 1);
            assert_eq!(data.offset_sub_key_list, 2587704);
            assert_eq!(data.offset_volatile_sub_key_list, 2147484264);
            assert_eq!(data.num_values, 0);
            assert_eq!(data.offset_value_list, 4294967295);
            assert_eq!(data.offset_security_key, 7568);
            assert_eq!(data.offset_class_name, 4294967295);
            assert_eq!(data.largest_sub_key_name_size, 42);
            assert_eq!(data.largest_sub_key_class_name_size, 0);
            assert_eq!(data.largest_value_name_size, 0);
            assert_eq!(data.largest_value_data_size, 0);
            assert_eq!(data.work_var, 3342392);
            assert_eq!(data.key_name_size, 57);
            assert_eq!(data.class_name_size, 0);
            assert_eq!(data.key_name, "CsiTool-CreateHive-{00000000-0000-0000-0000-000000000000}");
            // assert_eq!(format!("{:?}",data.padding), "\"00390031004500\"");
        },
        _ => panic!("Cell signature incorrect: {}",signature.0)
    }
}
