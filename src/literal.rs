use std::borrow::Cow;
use std::io::{Read, Write};
use std::u16;
use byteorder::{WriteBytesExt, ReadBytesExt};
use trackable::error::Failed;

use Result;
use huffman;
use io::SliceReader;

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

// TODO: private
#[derive(Debug, Clone, Copy)]
pub enum Encoding {
    Raw = 0,
    Huffman = 1,
}

#[derive(Debug)]
pub struct HpackString<B> {
    encoding: Encoding,
    octets: B, // TODO: cow
}
impl<B> HpackString<B>
where
    B: AsRef<[u8]>,
{
    pub fn encoding(&self) -> Encoding {
        self.encoding
    }
    // TODO: #[cfg(test)]
    pub fn octets(&self) -> &[u8] {
        self.octets.as_ref()
    }
    pub fn new_raw(octets: B) -> Self {
        HpackString {
            encoding: Encoding::Raw,
            octets,
        }
    }
    pub fn new_huffman(octets: B) -> HpackString<Vec<u8>> {
        let encoded = huffman::encode(octets.as_ref());
        HpackString {
            encoding: Encoding::Huffman,
            octets: encoded,
        }
    }
    pub fn as_ref(&self) -> HpackString<&[u8]> {
        HpackString {
            encoding: self.encoding,
            octets: self.octets.as_ref(),
        }
    }
    pub fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        debug_assert!(self.octets.as_ref().len() <= u16::MAX as usize);
        track!(encode_u16(
            &mut writer,
            self.encoding as u8,
            7,
            self.octets.as_ref().len() as u16,
        ))?;
        track_io!(writer.write_all(self.octets.as_ref()))?;
        Ok(())
    }
    pub fn to_vec(&self) -> Result<Vec<u8>> {
        if let Encoding::Raw = self.encoding {
            Ok(self.octets.as_ref().to_owned())
        } else {
            track!(huffman::decode(self.octets.as_ref()))
        }
    }
}
impl HpackString<Vec<u8>> {
    pub fn into_cow(self) -> HpackString<Cow<'static, [u8]>> {
        HpackString {
            encoding: self.encoding,
            octets: Cow::Owned(self.octets),
        }
    }
}
impl<'a> HpackString<&'a [u8]> {
    pub fn new(octets: &'a [u8], encoding: Encoding) -> HpackString<Cow<'a, [u8]>> {
        let octets = match encoding {
            Encoding::Raw => Cow::Borrowed(octets),
            Encoding::Huffman => Cow::Owned(huffman::encode(octets)),
        };
        HpackString { encoding, octets }
    }
}
impl<'a> HpackString<Cow<'a, [u8]>> {
    pub fn into_raw(self) -> Result<Cow<'a, [u8]>> {
        if let Encoding::Raw = self.encoding {
            Ok(self.octets)
        } else {
            let octets = track!(huffman::decode(self.octets.as_ref()))?;
            Ok(Cow::Owned(octets))
        }
    }
    pub fn decode(mut reader: &mut SliceReader<'a>) -> Result<Self> {
        let (encoding, octets_len) = track!(decode_u16(&mut reader, 7))?;
        let octets = Cow::Borrowed(track!(reader.read_slice(octets_len as usize))?);
        let encoding = if encoding == 0 {
            Encoding::Raw
        } else {
            Encoding::Huffman
        };
        Ok(HpackString { encoding, octets })
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
