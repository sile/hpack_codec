use std::io::{Read, Write};
use std::u16;
use byteorder::{WriteBytesExt, ReadBytesExt};

use {Result, ErrorKind};

macro_rules! track_io {
    ($e:expr) => {
        $e.map_err(|e| {
            use ::trackable::error::ErrorKindExt;
            ::ErrorKind::Io.cause(e)
        })
    }
}

pub fn encode_u16<W: Write>(
    mut writer: W,
    prepended_value: u8,
    prefix_bits: u8,
    value: u16,
) -> Result<()> {
    debug_assert!(1 <= prefix_bits && prefix_bits <= 8);
    let max_prefix_value: u16 = (1 << prefix_bits) - 1;
    if value < max_prefix_value {
        let first_octet = (((prepended_value as u16) << prefix_bits) | value) as u8;
        track_io!(writer.write_u8(first_octet))?;
    } else {
        let first_octet = (prepended_value << prefix_bits) | (max_prefix_value as u8);
        track_io!(writer.write_u8(first_octet))?;
        let mut value = value - max_prefix_value;
        while value >= 128 {
            track_io!(writer.write_u8((value % 128 + 128) as u8))?;
            value /= 128;
        }
        track_io!(writer.write_u8(value as u8))?;
    }
    Ok(())
}

pub fn decode_u16<R: Read>(mut reader: R, prefix_bits: u8) -> Result<(u8, u16)> {
    debug_assert!(1 <= prefix_bits && prefix_bits <= 8);
    let max_prefix_value: u16 = (1 << prefix_bits) - 1;
    let first_octet = track_io!(reader.read_u8())?;
    let prepended_value = ((first_octet as u16) >> prefix_bits) as u8;
    let mut value = first_octet as u16 & max_prefix_value;
    if value == max_prefix_value {
        let mut offset = 0;
        let mut octet = 128;
        while octet & 128 == 128 {
            octet = track_io!(reader.read_u8())?;

            let addition = (octet as u16 & 127) << offset;
            value = track_assert_some!(
                value.checked_add(addition),
                ErrorKind::InvalidInput,
                "Too large integer: {}",
                value as u32 + addition as u32
            );
            offset += 7;
        }
    }
    Ok((prepended_value, value))
}

pub fn encode_raw_octets<W: Write>(mut writer: W, octets: &[u8]) -> Result<()> {
    track_assert!(
        octets.len() <= u16::MAX as usize,
        ErrorKind::InvalidInput,
        "Too long octets: length={}",
        octets.len()
    );
    track!(encode_u16(&mut writer, 0, 7, octets.len() as u16))?;
    track_io!(writer.write_all(octets))?;
    Ok(())
}

pub fn encode_huffman_octets<W: Write>(mut _writer: W, _octets: &[u8]) -> Result<()> {
    unimplemented!()
}

pub fn decode_octets<R: Read>(mut reader: R) -> Result<Vec<u8>> {
    let (is_huffman_encoded, data_len) = track!(decode_u16(&mut reader, 1))?;
    if is_huffman_encoded == 1 {
        unimplemented!()
    } else {
        let mut data = vec![0; data_len as usize];
        track_io!(reader.read_exact(&mut data[..]))?;
        Ok(data)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    /// https://tools.ietf.org/html/rfc7541#appendix-C.1.1
    fn encoding_10_using_a_5bit_prefix() {
        let mut buf = [0; 1];
        track_try_unwrap!(encode_u16(&mut buf[..], 0b110, 5, 10));
        assert_eq!(buf, [0b110_01010]);

        let (prepended, value) = track_try_unwrap!(decode_u16(&buf[..], 5));
        assert_eq!(prepended, 0b110);
        assert_eq!(value, 10);
    }

    #[test]
    /// https://tools.ietf.org/html/rfc7541#appendix-C.1.2
    fn encoding_1337_using_a_5bit_prefix() {
        let mut buf = [0; 3];
        track_try_unwrap!(encode_u16(&mut buf[..], 0b110, 5, 1337));
        assert_eq!(buf, [0b110_11111, 0b10011010, 0b00001010]);

        let (prepended, value) = track_try_unwrap!(decode_u16(&buf[..], 5));
        assert_eq!(prepended, 0b110);
        assert_eq!(value, 1337);
    }

    #[test]
    /// https://tools.ietf.org/html/rfc7541#appendix-C.1.3
    fn encoding_42_starting_at_an_octet_boundary() {
        let mut buf = [0; 1];
        track_try_unwrap!(encode_u16(&mut buf[..], 0, 8, 42));
        assert_eq!(buf, [0b00101010]);

        let (prepended, value) = track_try_unwrap!(decode_u16(&buf[..], 8));
        assert_eq!(prepended, 0);
        assert_eq!(value, 42);
    }
}
