use encoding::{Encoding, DecoderTrap};
use encoding::all::UTF_16LE;
use encoding::all::ASCII;
use errors::RegError;

pub fn read_ascii(buffer: &[u8]) -> Result<String,RegError> {
    let mut end_index = buffer.len();

    // We need to check if the end of the buffer has null
    let buff_len = buffer.len();
    if buff_len >= 1 {
        // We have at least one bytes
        if buffer[buff_len-1] == 0 {
            // last byte is null terminator
            end_index -= 1;
        }
    }

    let ascii_string = match ASCII.decode(&buffer[0..end_index],DecoderTrap::Ignore) {
        Ok(ascii) => ascii,
        Err(error) => {
            return Err(
                RegError::ascii_decode_error(
                    format!("Error decoding ascii. [{}]",error)
                )
            )
        }
    };

    Ok(ascii_string)
}

pub fn read_utf16(buffer: &[u8]) -> Result<String,RegError> {
    let mut end_index = buffer.len();

    // We need to check if the end of the buffer has null
    let buff_len = buffer.len();
    if buff_len >= 2 {
        // We have at least two bytes
        if buffer[buff_len-1] == 0 && buffer[buff_len-2] == 0 {
            // last byte is null terminator
            end_index -= 2;
        }
    }

    let utf16_string = match UTF_16LE.decode(&buffer[0..end_index],DecoderTrap::Ignore) {
        Ok(utf16) => utf16,
        Err(error) => return Err(
            RegError::utf16_decode_error(
                format!("{}",error)
            )
        )
    };
    Ok(utf16_string)
}

pub fn read_string_u16_till_null(buffer: &[u8])->Result<String,RegError> {
    let mut end_index: usize = 0;
    let buf_len = buffer.len();
    let mut i = 0;
    while i < buf_len {
        if buffer[i] == 0x00 && buffer[i+1] == 0x00 {
            break
        } else {
            i += 2;
            end_index = i;
        }
    }

    let utf16_string = match UTF_16LE.decode(
        &buffer[0..end_index],
        DecoderTrap::Ignore
    ){
        Ok(utf16_string) => utf16_string,
        Err(error)=>{
            return Err(
                RegError::utf16_decode_error(format!("{:?}",error))
            )
        }
    };

    Ok(utf16_string)
}

pub fn to_hex_string(bytes: &[u8]) -> String {
    let strs: Vec<String> = bytes.iter()
        .map(|b| format!("{:02X}", b))
        .collect();
    strs.join("")
}
