//! Header Field.
use std;
use std::borrow::Cow;
use std::io::Write;
use byteorder::WriteBytesExt;
use trackable::error::Failed;

use Result;
use huffman;
use io::SliceReader;
use literal::{self, HpackString};
use table::{Index, StaticEntry};

/// Header Field.
///
/// This is a name-value pair. Both the name and value are
/// treated as opaque sequences of octets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderField<'a> {
    name: Cow<'a, [u8]>,
    value: Cow<'a, [u8]>,
}
impl<'a> HeaderField<'a> {
    /// Makes a new `HeaderField` instance.
    ///
    /// # Errors
    ///
    /// If the size of resulting header is too large, the function will returns an `Error`.
    ///
    /// The maximum size of a header (i.e., the sum of it's name and value) is `std::u16::MAX - 32`.
    pub fn new(name: &'a [u8], value: &'a [u8]) -> Result<Self> {
        let entry_size = name.len() + value.len() + 32;
        track_assert!(
            entry_size <= std::u16::MAX as usize,
            Failed,
            "Too large header field: {}",
            entry_size
        );
        Ok(HeaderField {
            name: Cow::Borrowed(name),
            value: Cow::Borrowed(value),
        })
    }

    /// Returns the name of this header field.
    pub fn name(&self) -> &[u8] {
        self.name.as_ref()
    }

    /// Returns the value of this header field.
    pub fn value(&self) -> &[u8] {
        self.value.as_ref()
    }

    /// Returns the entry size of this header field.
    ///
    /// See: [4.1.  Calculating Table Size](https://tools.ietf.org/html/rfc7541#section-4.1)
    pub fn entry_size(&self) -> u16 {
        (self.name.len() + self.value.len() + 32) as u16
    }

    pub(crate) fn from_cow(name: Cow<'a, [u8]>, value: Cow<'a, [u8]>) -> Self {
        let entry_size = name.len() + value.len() + 32;
        debug_assert!(entry_size <= std::u16::MAX as usize);
        HeaderField { name, value }
    }
    pub(crate) fn into_cow_name(self) -> Cow<'a, [u8]> {
        self.name
    }
    pub(crate) fn as_borrowed(&self) -> HeaderField {
        HeaderField {
            name: Cow::Borrowed(self.name.as_ref()),
            value: Cow::Borrowed(self.value.as_ref()),
        }
    }
}

/// Raw representation of a header field.
///
/// See: [2.4.  Header Field Representation](https://tools.ietf.org/html/rfc7541#section-2.4)
#[derive(Debug)]
#[allow(missing_docs)]
pub enum RawHeaderField<'a> {
    Indexed(IndexedHeaderField),
    Literal(LiteralHeaderField<'a>),
}
impl<'a> RawHeaderField<'a> {
    pub(crate) fn encode<W: Write>(&self, writer: W) -> Result<()> {
        match *self {
            RawHeaderField::Indexed(ref field) => track!(field.encode(writer)),
            RawHeaderField::Literal(ref field) => track!(field.encode(writer)),
        }
    }
    pub(crate) fn decode(reader: &mut SliceReader<'a>) -> Result<Self> {
        let octet = track_io!(reader.peek_u8())?;
        if octet >> 7 == 0b1 {
            track!(IndexedHeaderField::decode(reader)).map(RawHeaderField::Indexed)
        } else if octet >> 5 == 0b001 {
            track_panic!(
                Failed,
                "Dynamic table size update MUST occur at the beginning of the first header block"
            );
        } else {
            track!(LiteralHeaderField::decode(reader, octet).map(
                RawHeaderField::Literal,
            ))
        }
    }
}
impl<'a> From<StaticEntry> for RawHeaderField<'a> {
    fn from(f: StaticEntry) -> Self {
        RawHeaderField::Indexed(IndexedHeaderField(Index::from(f)))
    }
}
impl<'a> From<Index> for RawHeaderField<'a> {
    fn from(f: Index) -> Self {
        RawHeaderField::Indexed(IndexedHeaderField(f))
    }
}
impl<'a> From<LiteralHeaderField<'a>> for RawHeaderField<'a> {
    fn from(f: LiteralHeaderField<'a>) -> Self {
        RawHeaderField::Literal(f)
    }
}

/// Indexed representation of a header field.
///
/// See: [6.1.  Indexed Header Field Representation](https://tools.ietf.org/html/rfc7541#section-6.1)
#[derive(Debug)]
pub struct IndexedHeaderField(Index);
impl IndexedHeaderField {
    /// Makes a new `IndexedHeaderField` instance.
    pub fn new(index: Index) -> Self {
        IndexedHeaderField(index)
    }

    /// Returns the index of this header field.
    pub fn index(&self) -> Index {
        self.0
    }

    fn encode<W: Write>(&self, writer: W) -> Result<()> {
        track!(literal::encode_u16(writer, 1, 7, self.0.as_u16()))
    }
    fn decode(reader: &mut SliceReader) -> Result<Self> {
        let index = track!(literal::decode_u16(reader, 7))?.1;
        let index = track!(Index::new(index))?;
        Ok(IndexedHeaderField(index))
    }
}

/// Literal representation of a header field.
///
/// See: [6.2.  Literal Header Field Representation](https://tools.ietf.org/html/rfc7541#section-6.2)
#[derive(Debug)]
pub struct LiteralHeaderField<'a> {
    name: FieldName<'a>,
    value: HpackString<'a>,
    form: LiteralFieldForm,
}
impl<'a> LiteralHeaderField<'a> {
    /// Makes a new `LiteralHeaderField` instance.
    ///
    /// # Examples
    ///
    /// ```
    /// use hpack_codec::field::{LiteralHeaderField, LiteralFieldForm};;
    ///
    /// let field = LiteralHeaderField::new(b"foo", b"bar");
    /// assert_eq!(field.form(), LiteralFieldForm::WithoutIndexing);
    /// ```
    pub fn new(name: &'a [u8], value: &'a [u8]) -> Self {
        LiteralHeaderField {
            name: FieldName::Name(HpackString::Plain(Cow::Borrowed(name))),
            value: HpackString::Plain(Cow::Borrowed(value)),
            form: LiteralFieldForm::WithoutIndexing,
        }
    }

    /// Makes a new `LiteralHeaderField` instance with the specified indexed name.
    ///
    /// # Examples
    ///
    /// ```
    /// use hpack_codec::field::{LiteralHeaderField, LiteralFieldForm};;
    /// use hpack_codec::table::{Index, StaticEntry};
    ///
    /// // Uses an index of the static table.
    /// let field = LiteralHeaderField::with_indexed_name(StaticEntry::Method, b"foo");
    ///
    /// // Uses an index of a dynamic table;
    /// let field = LiteralHeaderField::with_indexed_name(Index::dynamic_table_offset() + 2, b"bar");
    /// ```
    pub fn with_indexed_name<N>(name: N, value: &'a [u8]) -> Self
    where
        N: Into<Index>,
    {
        LiteralHeaderField {
            name: FieldName::Index(name.into()),
            value: HpackString::Plain(Cow::Borrowed(value)),
            form: LiteralFieldForm::WithoutIndexing,
        }
    }

    /// Specifies to index this header field into a dynamic table.
    pub fn with_indexing(mut self) -> Self {
        self.form = LiteralFieldForm::WithIndexing;
        self
    }

    /// Specifies that this header field will be never indexed into a dynamic table.
    pub fn never_indexed(mut self) -> Self {
        self.form = LiteralFieldForm::NeverIndexed;
        self
    }

    /// Encodes the name of this header field by huffman coding.
    pub fn with_huffman_encoded_name(mut self) -> Self {
        if let FieldName::Name(HpackString::Plain(name)) = self.name {
            self.name = FieldName::Name(HpackString::Huffman(Cow::Owned(huffman::encode(&name))));
            self
        } else {
            self
        }
    }

    /// Encodes the value of this header field by huffman coding.
    pub fn with_huffman_encoded_value(mut self) -> Self {
        if let HpackString::Plain(value) = self.value {
            self.value = HpackString::Huffman(Cow::Owned(huffman::encode(&value)));
        }
        self
    }

    /// Returns the name of this header field.
    pub fn name(&self) -> &FieldName {
        &self.name
    }

    /// Returns the value of this header field.
    pub fn value(&self) -> &HpackString {
        &self.value
    }

    /// Returns the form of this header field.
    pub fn form(&self) -> LiteralFieldForm {
        self.form
    }

    pub(crate) fn unwrap(self) -> (FieldName<'a>, HpackString<'a>, LiteralFieldForm) {
        (self.name, self.value, self.form)
    }
    fn encode<W: Write>(&self, mut writer: W) -> Result<()> {
        track!(self.encode_name(&mut writer))?;
        track!(self.value.encode(writer))
    }

    fn decode(reader: &mut SliceReader<'a>, first_octet: u8) -> Result<Self> {
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
                let index = track!(Index::new(index))?;
                FieldName::Index(index)
            };
            Ok((name, LiteralFieldForm::WithIndexing))
        } else if first_octet == 0b0001_0000 {
            let name = if first_octet & 0b1111 == 0 {
                reader.consume(1);
                let name = track!(HpackString::decode(reader))?;
                FieldName::Name(name)
            } else {
                let index = track!(literal::decode_u16(&mut reader, 4))?.1;
                let index = track!(Index::new(index))?;
                FieldName::Index(index)
            };
            Ok((name, LiteralFieldForm::NeverIndexed))
        } else {
            let name = if first_octet & 0b1111 == 0 {
                reader.consume(1);
                let name = track!(HpackString::decode(reader))?;
                FieldName::Name(name)
            } else {
                let index = track!(literal::decode_u16(&mut reader, 4))?.1;
                let index = track!(Index::new(index))?;
                FieldName::Index(index)
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

/// Available forms of literal header fields.
///
/// See: [6.2.  Literal Header Field Representation](https://tools.ietf.org/html/rfc7541#section-6.2)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiteralFieldForm {
    /// See: [6.2.1.  Literal Header Field with Incremental Indexing](https://tools.ietf.org/html/rfc7541#section-6.2.1)
    WithIndexing,

    /// See: [6.2.2.  Literal Header Field without Indexing](https://tools.ietf.org/html/rfc7541#section-6.2.2)
    WithoutIndexing,

    /// See: [6.2.3.  Literal Header Field Never Indexed](https://tools.ietf.org/html/rfc7541#section-6.2.3)
    NeverIndexed,
}

/// The name of a header field.
#[derive(Debug)]
#[allow(missing_docs)]
pub enum FieldName<'a> {
    Index(Index),
    Name(HpackString<'a>),
}
