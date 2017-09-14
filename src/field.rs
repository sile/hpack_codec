use std::borrow::Cow;
use std::io::Write;
use byteorder::WriteBytesExt;
use trackable::error::Failed;

use Result;
use io::SliceReader;
use literal::{self, HpackString, Encoding};
use table::{Index, StaticEntry};

#[derive(Debug)]
pub struct PlainHeaderField<'a> {
    pub name: Cow<'a, [u8]>,
    pub value: Cow<'a, [u8]>,
}

#[derive(Debug)]
pub enum HeaderField<'a> {
    Indexed(IndexedHeaderField),
    Literal(LiteralHeaderField<'a>),
}
impl<'a> HeaderField<'a> {
    pub fn encode<W: Write>(&self, writer: W) -> Result<()> {
        match *self {
            HeaderField::Indexed(ref field) => track!(field.encode(writer)),
            HeaderField::Literal(ref field) => track!(field.encode(writer)),
        }
    }
    pub fn decode(reader: &mut SliceReader<'a>) -> Result<Self> {
        let octet = track_io!(reader.peek_u8())?;
        if octet >> 7 == 0b1 {
            track!(IndexedHeaderField::decode(reader)).map(HeaderField::Indexed)
        } else if octet >> 5 == 0b001 {
            track_panic!(
                Failed,
                "Dynamic table size update MUST occur at the beginning of the first header block"
            );
        } else {
            track!(LiteralHeaderField::decode(reader, octet).map(
                HeaderField::Literal,
            ))
        }
    }
}
impl<'a> From<StaticEntry> for HeaderField<'a> {
    fn from(f: StaticEntry) -> Self {
        HeaderField::Indexed(IndexedHeaderField(Index::from(f)))
    }
}
impl<'a> From<Index> for HeaderField<'a> {
    fn from(f: Index) -> Self {
        HeaderField::Indexed(IndexedHeaderField(f))
    }
}
impl<'a> From<LiteralHeaderField<'a>> for HeaderField<'a> {
    fn from(f: LiteralHeaderField<'a>) -> Self {
        HeaderField::Literal(f)
    }
}

#[derive(Debug)]
pub struct IndexedHeaderField(Index);
impl IndexedHeaderField {
    pub fn index(&self) -> Index {
        self.0
    }
    pub fn encode<W: Write>(&self, writer: W) -> Result<()> {
        track!(literal::encode_u16(writer, 1, 7, self.0.as_u16()))
    }
    pub fn decode(reader: &mut SliceReader) -> Result<Self> {
        let index = Index::new(track!(literal::decode_u16(reader, 7))?.1).expect("TODO");
        Ok(IndexedHeaderField(index))
    }
}

#[derive(Debug)]
pub struct LiteralHeaderField<'a> {
    pub name: FieldName<'a>,
    pub value: HpackString<Cow<'a, [u8]>>,
    pub form: LiteralFieldForm,
}
impl<'a> LiteralHeaderField<'a> {
    pub fn new(name: &'a [u8], value: &'a [u8]) -> Self {
        LiteralHeaderField {
            name: FieldName::Name(HpackString::new_raw(Cow::Borrowed(name))),
            value: HpackString::new_raw(Cow::Borrowed(value)),
            form: LiteralFieldForm::WithoutIndexing,
        }
    }
    pub fn with_indexed_name<N>(name: N, value: &'a [u8]) -> Self
    where
        N: Into<Index>,
    {
        LiteralHeaderField {
            name: FieldName::Index(name.into()),
            value: HpackString::new_raw(Cow::Borrowed(value)),
            form: LiteralFieldForm::WithoutIndexing,
        }
    }
    pub fn with_indexing(mut self) -> Self {
        self.form = LiteralFieldForm::WithIndexing;
        self
    }
    pub fn huffman_encoded_name(mut self) -> Self {
        if let FieldName::Name(name) = self.name {
            if let Encoding::Raw = name.encoding() {
                self.name = FieldName::Name(HpackString::new_huffman(name.octets()).into_cow());
            } else {
                self.name = FieldName::Name(name);
            }
            self
        } else {
            self
        }
    }
    pub fn huffman_encoded_value(mut self) -> Self {
        if let Encoding::Raw = self.value.encoding() {
            self.value = HpackString::new_huffman(self.value.octets()).into_cow();
        }
        self
    }

    pub fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        track!(self.encode_name(&mut writer))?;
        track!(self.value.encode(writer))
    }

    pub fn decode(reader: &mut SliceReader<'a>, first_octet: u8) -> Result<Self> {
        let (name, form) = track!(Self::decode_name_and_form(reader, first_octet))?;
        let value = track!(HpackString::decode(reader))?;
        Ok(LiteralHeaderField { name, value, form })
    }

    fn decode_name_and_form(
        mut reader: &mut SliceReader<'a>,
        first_octet: u8,
    ) -> Result<(FieldName<'a>, LiteralFieldForm)> {
        if first_octet >> 6 == 0b01 {
            let name = if first_octet & 0b11_1111 == 0 {
                reader.consume(1);
                let name = track!(HpackString::decode(reader))?;
                FieldName::Name(name)
            } else {
                let index = track!(literal::decode_u16(&mut reader, 6))?.1;
                FieldName::Index(Index::new(index).expect("TODO"))
            };
            Ok((name, LiteralFieldForm::WithIndexing))
        } else if first_octet == 0b0001_0000 {
            let name = if first_octet & 0b1111 == 0 {
                reader.consume(1);
                let name = track!(HpackString::decode(reader))?;
                FieldName::Name(name)
            } else {
                let index = track!(literal::decode_u16(&mut reader, 4))?.1;
                FieldName::Index(Index::new(index).expect("TODO"))
            };
            Ok((name, LiteralFieldForm::NeverIndexed))
        } else {
            let name = if first_octet & 0b1111 == 0 {
                reader.consume(1);
                let name = track!(HpackString::decode(reader))?;
                FieldName::Name(name)
            } else {
                let index = track!(literal::decode_u16(&mut reader, 4))?.1;
                FieldName::Index(Index::new(index).expect("TODO"))
            };
            Ok((name, LiteralFieldForm::WithoutIndexing))
        }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiteralFieldForm {
    WithIndexing,
    WithoutIndexing,
    NeverIndexed,
}

#[derive(Debug)]
pub enum FieldName<'a> {
    Index(Index),
    Name(HpackString<Cow<'a, [u8]>>),
}
