use std::borrow::Cow;
use std::cmp;
use std::io::{self, Read, Write};
use byteorder::{ReadBytesExt, WriteBytesExt};
use trackable::error::Failed;

use {Result, Name};
use literal::{self, HpackString, Encoding};

#[derive(Debug)]
pub struct LiteralHeaderFieldBuilder {
    form: LiteralFieldForm,
    name_encoding: Encoding,
    value_encoding: Encoding,
}
impl LiteralHeaderFieldBuilder {
    pub fn with_indexing() -> Self {
        LiteralHeaderFieldBuilder {
            form: LiteralFieldForm::WithIndexing,
            name_encoding: Encoding::Raw,
            value_encoding: Encoding::Raw,
        }
    }
    pub fn without_indexing() -> Self {
        LiteralHeaderFieldBuilder {
            form: LiteralFieldForm::WithoutIndexing,
            name_encoding: Encoding::Raw,
            value_encoding: Encoding::Raw,
        }
    }
    pub fn never_indexed() -> Self {
        LiteralHeaderFieldBuilder {
            form: LiteralFieldForm::NeverIndexed,
            name_encoding: Encoding::Raw,
            value_encoding: Encoding::Raw,
        }
    }
    pub fn name_encoding(&mut self, encoding: Encoding) -> &mut Self {
        self.name_encoding = encoding;
        self
    }
    pub fn value_encoding(&mut self, encoding: Encoding) -> &mut Self {
        self.value_encoding = encoding;
        self
    }
    pub fn finish<'a, 'b>(
        &self,
        name: Name<'a>,
        value: &'b [u8],
    ) -> LiteralHeaderField<Cow<'a, [u8]>, Cow<'b, [u8]>> {
        let name = name.to_field_name(self.name_encoding);
        let value = HpackString::new(value, self.value_encoding);
        let form = self.form;
        LiteralHeaderField { form, name, value }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Index(pub u16);
impl Index {
    pub fn new(index: u16) -> Result<Self> {
        track_assert_ne!(index, 0, Failed);
        Ok(Index(index))
    }
    pub fn as_u16(&self) -> u16 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    pub fn new(octets: &'a [u8]) -> Self {
        Reader { octets, offset: 0 }
    }
    pub fn eos(&self) -> bool {
        debug_assert!(self.offset <= self.octets.len());
        self.offset == self.octets.len()
    }
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
            Failed,
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
pub enum HeaderField<N, V = N> {
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
    pub max_size: u16,
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
    pub index: Index,
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
    pub name: FieldName<N>,
    pub value: HpackString<V>,
    pub form: LiteralFieldForm,
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
                reader.consume(1);
                let name = track!(HpackString::decode(reader))?;
                FieldName::Name(name)
            } else {
                let index = track!(literal::decode_u16(&mut reader, 6))?.1;
                FieldName::Index(Index(index))
            };
            Ok((name, LiteralFieldForm::WithIndexing))
        } else if first_octet == 0b0001_0000 {
            let name = if first_octet & 0b1111 == 0 {
                reader.consume(1);
                let name = track!(HpackString::decode(reader))?;
                FieldName::Name(name)
            } else {
                let index = track!(literal::decode_u16(&mut reader, 4))?.1;
                FieldName::Index(Index(index))
            };
            Ok((name, LiteralFieldForm::NeverIndexed))
        } else {
            let name = if first_octet & 0b1111 == 0 {
                reader.consume(1);
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

#[cfg(test)]
mod test {
    use literal::HpackString;
    use super::*;

    #[test]
    /// https://tools.ietf.org/html/rfc7541#appendix-C.2.1
    fn literal_header_field_with_indexing() {
        let field = HeaderField::Literal(LiteralHeaderField {
            form: LiteralFieldForm::WithIndexing,
            name: FieldName::Name(HpackString::new_raw(b"custom-key")),
            value: HpackString::new_raw(b"custom-header"),
        });

        // encode
        let mut buf = Vec::new();
        track_try_unwrap!(field.encode(&mut buf));
        #[cfg_attr(rustfmt, rustfmt_skip)]
        {
            assert_eq!(
                buf,
                [
                    0x40, 0x0a, 0x63, 0x75, 0x73, 0x74, 0x6f, 0x6d, 0x2d,
                    0x6b, 0x65, 0x79, 0x0d, 0x63, 0x75, 0x73, 0x74, 0x6f,
                    0x6d, 0x2d, 0x68, 0x65, 0x61, 0x64, 0x65, 0x72,
                ]
            );
        }

        // decode
        let mut reader = Reader::new(&buf[..]);
        let field = track_try_unwrap!(HeaderField::decode(&mut reader));
        if let HeaderField::Literal(field) = field {
            assert_eq!(field.form, LiteralFieldForm::WithIndexing);
            if let FieldName::Name(ref name) = field.name {
                assert_eq!(name.octets(), b"custom-key");
            } else {
                panic!("{:?}", field.name);
            }
            assert_eq!(field.value.octets(), b"custom-header");
        } else {
            panic!("{:?}", field);
        }
    }

    #[test]
    /// https://tools.ietf.org/html/rfc7541#appendix-C.2.2
    fn literal_header_field_without_indexing() {
        let field = HeaderField::Literal::<Vec<u8>, _>(LiteralHeaderField {
            form: LiteralFieldForm::WithoutIndexing,
            name: FieldName::Index(Index(4)),
            value: HpackString::new_raw(b"/sample/path"),
        });

        // encode
        let mut buf = Vec::new();
        track_try_unwrap!(field.encode(&mut buf));
        #[cfg_attr(rustfmt, rustfmt_skip)]
        {
            assert_eq!(
                buf,
                [
                    0x04, 0x0c, 0x2f, 0x73, 0x61, 0x6d, 0x70,
                    0x6c, 0x65, 0x2f, 0x70, 0x61, 0x74, 0x68
                ]
            );
        }

        // decode
        let mut reader = Reader::new(&buf[..]);
        let field = track_try_unwrap!(HeaderField::decode(&mut reader));
        if let HeaderField::Literal(field) = field {
            assert_eq!(field.form, LiteralFieldForm::WithoutIndexing);
            if let FieldName::Index(index) = field.name {
                assert_eq!(index.as_u16(), 4);
            } else {
                panic!("{:?}", field.name);
            }
            assert_eq!(field.value.octets(), b"/sample/path");
        } else {
            panic!("{:?}", field);
        }
    }

    #[test]
    /// https://tools.ietf.org/html/rfc7541#appendix-C.2.3
    fn literal_header_field_never_indexed() {
        let field = HeaderField::Literal(LiteralHeaderField {
            form: LiteralFieldForm::NeverIndexed,
            name: FieldName::Name(HpackString::new_raw(b"password")),
            value: HpackString::new_raw(b"secret"),
        });

        // encode
        let mut buf = Vec::new();
        track_try_unwrap!(field.encode(&mut buf));
        #[cfg_attr(rustfmt, rustfmt_skip)]
        {
            assert_eq!(
                buf,
                [
                    0x10, 0x08, 0x70, 0x61, 0x73, 0x73, 0x77, 0x6f, 0x72,
                    0x64, 0x06, 0x73, 0x65, 0x63, 0x72, 0x65, 0x74
                ]
            );
        }

        // decode
        let mut reader = Reader::new(&buf[..]);
        let field = track_try_unwrap!(HeaderField::decode(&mut reader));
        if let HeaderField::Literal(field) = field {
            assert_eq!(field.form, LiteralFieldForm::NeverIndexed);
            if let FieldName::Name(ref name) = field.name {
                assert_eq!(name.octets(), b"password");
            } else {
                panic!("{:?}", field.name);
            }
            assert_eq!(field.value.octets(), b"secret");
        } else {
            panic!("{:?}", field);
        }
    }

    #[test]
    /// https://tools.ietf.org/html/rfc7541#appendix-C.2.4
    fn indexed_header_field() {
        let field =
            HeaderField::Indexed::<Vec<u8>, Vec<u8>>(IndexedHeaderField { index: Index(2) });

        // encode
        let mut buf = Vec::new();
        track_try_unwrap!(field.encode(&mut buf));
        assert_eq!(buf, [0x82]);

        // decode
        let mut reader = Reader::new(&buf[..]);
        let field = track_try_unwrap!(HeaderField::decode(&mut reader));
        if let HeaderField::Indexed(field) = field {
            assert_eq!(field.index.as_u16(), 2);
        } else {
            panic!("{:?}", field);
        }
    }
}
