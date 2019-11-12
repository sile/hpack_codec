//! Literal types.
use crate::huffman;
use crate::io::SliceReader;
use crate::Result;
use byteorder::{ReadBytesExt, WriteBytesExt};
use std::borrow::Cow;
use std::io::{Read, Write};
use std::u16;
use trackable::error::Failed;

pub(crate) fn encode_u16<W: Write>(
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

pub(crate) fn decode_u16<R: Read>(mut reader: R, prefix_bits: u8) -> Result<(u8, u16)> {
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
                Failed,
                "Too large integer: {}",
                value as u32 + addition as u32
            );
            offset += 7;
        }
    }
    Ok((prepended_value, value))
}

/// HPACK String type.
///
/// A string literal is encoded as a sequence of
/// octets, either by directly encoding the string literal's octets or by
/// using a Huffman code.
///
/// See: [5.2.  String Literal Representation](https://tools.ietf.org/html/rfc7541#section-5.2)
#[derive(Debug)]
#[allow(missing_docs)]
pub enum HpackString<'a> {
    Plain(Cow<'a, [u8]>),
    Huffman(Cow<'a, [u8]>),
}
impl<'a> HpackString<'a> {
    pub(crate) fn to_plain_bytes(&self) -> Result<Cow<[u8]>> {
        match *self {
            HpackString::Plain(ref x) => Ok(Cow::Borrowed(x.as_ref())),
            HpackString::Huffman(ref x) => Ok(Cow::Owned(track!(huffman::decode(x))?)),
        }
    }
    pub(crate) fn into_plain_bytes(self) -> Result<Cow<'a, [u8]>> {
        match self {
            HpackString::Plain(x) => Ok(x),
            HpackString::Huffman(x) => Ok(Cow::Owned(track!(huffman::decode(x.as_ref()))?)),
        }
    }
    pub(crate) fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        let (encoding, octets) = match *self {
            HpackString::Plain(ref x) => (0, x.as_ref()),
            HpackString::Huffman(ref x) => (1, x.as_ref()),
        };
        debug_assert!(octets.len() <= u16::MAX as usize);
        track!(encode_u16(&mut writer, encoding, 7, octets.len() as u16))?;
        track_io!(writer.write_all(octets))?;
        Ok(())
    }
    pub(crate) fn decode(mut reader: &mut SliceReader<'a>) -> Result<Self> {
        let (encoding, octets_len) = track!(decode_u16(&mut reader, 7))?;
        let octets = Cow::Borrowed(track!(reader.read_slice(octets_len as usize))?);
        if encoding == 0 {
            Ok(HpackString::Plain(octets))
        } else {
            Ok(HpackString::Huffman(octets))
        }
    }
}

#[cfg(test)]
mod tests {
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
