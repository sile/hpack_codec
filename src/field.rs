use std::cmp;
use std::io::{self, Read, Write};
use byteorder::{ReadBytesExt, WriteBytesExt};

use {Result, ErrorKind};
use literal::{self, HpackString};

#[derive(Debug, Clone, Copy)]
pub struct Index(u16);
impl Index {
    pub fn as_u16(&self) -> u16 {
        self.0
    }
}

#[derive(Debug, Clone, Copy)]
pub enum LiteralFieldForm {
    WithIndexing,
    WithoutIndexing,
    NeverIndexed,
}

#[derive(Debug)]
pub enum FieldName<B> {
    Index(Index),
    Name(HpackString<B>),
}

#[derive(Debug)]
pub struct Reader<'a> {
    octets: &'a [u8],
    offset: usize,
}
impl<'a> Reader<'a> {
    pub fn peek_u8(&mut self) -> Result<u8> {
        let value = track_io!(self.read_u8())?;
        self.unread();
        Ok(value)
    }
    pub fn consume(&mut self, size: usize) {
        self.offset = cmp::min(self.offset + size, self.octets.len());
    }
    pub fn read_slice(&mut self, size: usize) -> Result<&'a [u8]> {
        track_assert!(
            self.offset + size <= self.octets.len(),
            ErrorKind::InvalidInput,
            "offset={}, size={}, octets.len={}",
            self.offset,
            size,
            self.octets.len()
        );
        let slice = &self.octets[self.offset..self.offset + size];
        self.offset += size;
        Ok(slice)
    }

    fn unread(&mut self) {
        debug_assert!(self.offset > 0);
        self.offset -= 1;
    }
}
impl<'a> Read for Reader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let size = (&self.octets[self.offset..]).read(buf)?;
        self.offset += size;
        Ok(size)
    }
}

#[derive(Debug)]
pub enum HeaderField<N, V> {
    Indexed(IndexedHeaderField),
    Literal(LiteralHeaderField<N, V>),
    Update(DynamicTableSizeUpdate), // TODO
}
impl<N, V> HeaderField<N, V>
where
    N: AsRef<[u8]>,
    V: AsRef<[u8]>,
{
    pub fn encode<W: Write>(&self, writer: W) -> Result<()> {
        match *self {
            HeaderField::Indexed(ref f) => track!(f.encode(writer)),
            HeaderField::Literal(ref f) => track!(f.encode(writer)),
            HeaderField::Update(ref f) => track!(f.encode(writer)),
        }
    }
}
impl<'a> HeaderField<&'a [u8], &'a [u8]> {
    pub fn decode(reader: &mut Reader<'a>) -> Result<Self> {
        let octet = track_io!(reader.peek_u8())?;
        if octet >> 7 == 0b1 {
            track!(IndexedHeaderField::decode(reader)).map(HeaderField::Indexed)
        } else if octet >> 5 == 0b001 {
            track!(DynamicTableSizeUpdate::decode(reader).map(
                HeaderField::Update,
            ))
        } else {
            track!(LiteralHeaderField::decode(reader, octet).map(
                HeaderField::Literal,
            ))
        }
    }
}

#[derive(Debug)]
pub struct DynamicTableSizeUpdate {
    max_size: u16,
}
impl DynamicTableSizeUpdate {
    pub fn encode<W: Write>(&self, writer: W) -> Result<()> {
        track!(literal::encode_u16(writer, 0b001, 5, self.max_size))
    }
    pub fn decode(reader: &mut Reader) -> Result<Self> {
        let max_size = track!(literal::decode_u16(reader, 5))?.1;
        Ok(DynamicTableSizeUpdate { max_size })
    }
}

#[derive(Debug)]
pub struct IndexedHeaderField {
    index: Index,
}
impl IndexedHeaderField {
    pub fn encode<W: Write>(&self, writer: W) -> Result<()> {
        track!(literal::encode_u16(writer, 1, 7, self.index.as_u16()))
    }
    pub fn decode(reader: &mut Reader) -> Result<Self> {
        let index = Index(track!(literal::decode_u16(reader, 7))?.1);
        Ok(IndexedHeaderField { index })
    }
}

#[derive(Debug)]
pub struct LiteralHeaderField<N, V> {
    name: FieldName<N>,
    value: HpackString<V>,
    form: LiteralFieldForm,
}
impl<N, V> LiteralHeaderField<N, V>
where
    N: AsRef<[u8]>,
    V: AsRef<[u8]>,
{
    pub fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        track!(self.encode_name(&mut writer))?;
        track!(self.value.encode(writer))
    }

    fn encode_name<W: Write>(&self, mut writer: W) -> Result<()> {
        use self::FieldName::*;
        use self::LiteralFieldForm::*;
        match (self.form, &self.name) {
            (WithIndexing, &Index(index)) => {
                track!(literal::encode_u16(writer, 0b01, 6, index.as_u16()))
            }
            (WithIndexing, &Name(ref name)) => {
                track_io!(writer.write_u8(0b01_000000))?;
                track!(name.encode(writer))
            }
            (WithoutIndexing, &Index(index)) => {
                track!(literal::encode_u16(writer, 0b0000, 4, index.as_u16()))
            }
            (WithoutIndexing, &Name(ref name)) => {
                track_io!(writer.write_u8(0b0000_0000))?;
                track!(name.encode(writer))
            }
            (NeverIndexed, &Index(index)) => {
                track!(literal::encode_u16(writer, 0b0001, 4, index.as_u16()))
            }
            (NeverIndexed, &Name(ref name)) => {
                track_io!(writer.write_u8(0b0001_0000))?;
                track!(name.encode(writer))
            }
        }
    }
}
impl<'a> LiteralHeaderField<&'a [u8], &'a [u8]> {
    pub fn decode(reader: &mut Reader<'a>, first_octet: u8) -> Result<Self> {
        let (name, form) = track!(Self::decode_name_and_form(reader, first_octet))?;
        let value = track!(HpackString::decode(reader))?;
        Ok(LiteralHeaderField { name, value, form })
    }

    fn decode_name_and_form(
        mut reader: &mut Reader<'a>,
        first_octet: u8,
    ) -> Result<(FieldName<&'a [u8]>, LiteralFieldForm)> {
        if first_octet >> 6 == 0b01 {
            let name = if first_octet & 0b11_1111 == 0 {
                let name = track!(HpackString::decode(reader))?;
                FieldName::Name(name)
            } else {
                let index = track!(literal::decode_u16(&mut reader, 6))?.1;
                FieldName::Index(Index(index))
            };
            Ok((name, LiteralFieldForm::WithIndexing))
        } else if first_octet == 0b0001_0000 {
            let name = if first_octet & 0b1111 == 0 {
                let name = track!(HpackString::decode(reader))?;
                FieldName::Name(name)
            } else {
                let index = track!(literal::decode_u16(&mut reader, 4))?.1;
                FieldName::Index(Index(index))
            };
            Ok((name, LiteralFieldForm::NeverIndexed))
        } else {
            let name = if first_octet & 0b1111 == 0 {
                let name = track!(HpackString::decode(reader))?;
                FieldName::Name(name)
            } else {
                let index = track!(literal::decode_u16(&mut reader, 4))?.1;
                FieldName::Index(Index(index))
            };
            Ok((name, LiteralFieldForm::WithoutIndexing))
        }
    }
}
