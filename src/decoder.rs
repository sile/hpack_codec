use crate::field::{
    FieldName, HeaderField, IndexedHeaderField, LiteralFieldForm, LiteralHeaderField,
    RawHeaderField,
};
use crate::io::SliceReader;
use crate::signal::DynamicTableSizeUpdate;
use crate::table::Table;
use crate::Result;
use trackable::error::Failed;

/// HPACK Decoder.
#[derive(Debug)]
pub struct Decoder {
    table: Table,
}
impl Decoder {
    /// Makes a new `Decoder` instance.
    pub fn new(max_dynamic_table_size: u16) -> Self {
        Decoder {
            table: Table::new(max_dynamic_table_size),
        }
    }

    /// Returns the indexing table of this decoder.
    pub fn table(&self) -> &Table {
        &self.table
    }

    /// Sets the hard limit of the dynamic table size of this decoder.
    ///
    /// # Errors
    ///
    /// If the value of `max_size` is greater than the soft limit of this decoder
    /// (i.e., the value of `self.table().dynamic().size_soft_limit()`),
    /// an error will be returned.
    pub fn set_dynamic_table_size_hard_limit(&mut self, max_size: u16) -> Result<()> {
        track_assert!(
            self.table.dynamic().size_soft_limit() <= max_size,
            Failed,
            "soft_limit={}, hard_limit={{old={}, new={}}}",
            self.table.dynamic().size_soft_limit(),
            self.table.dynamic().size_hard_limit(),
            max_size
        );
        self.table.dynamic_mut().set_size_hard_limit(max_size);
        Ok(())
    }

    /// Returns a `HeaderBlockDecoder` instance for decoding the header block `block`.
    pub fn enter_header_block<'a, 'b>(
        &'a mut self,
        block: &'b [u8],
    ) -> Result<HeaderBlockDecoder<'a, 'b>> {
        let mut reader = SliceReader::new(block);
        while (track!(reader.peek_u8())? & 0b0010_0000) != 0 {
            let update = track!(DynamicTableSizeUpdate::decode(&mut reader))?;
            track!(self
                .table
                .dynamic_mut()
                .set_size_soft_limit(update.max_size,))?;
        }
        Ok(HeaderBlockDecoder {
            table: &mut self.table,
            reader,
        })
    }
}

/// Header Block Decoder.
#[derive(Debug)]
pub struct HeaderBlockDecoder<'a, 'b> {
    table: &'a mut Table,
    reader: SliceReader<'b>,
}
impl<'a, 'b: 'a> HeaderBlockDecoder<'a, 'b> {
    /// Decodes a header field.
    ///
    /// If it reached the end of this block, `Ok(None)` will be returned.
    pub fn decode_field(&mut self) -> Result<Option<HeaderField>> {
        if let Some(field) = track!(self.decode_raw_field())? {
            let result = match field {
                RawHeaderField::Indexed(f) => track!(Self::handle_indexed_field(self.table, f)),
                RawHeaderField::Literal(f) => track!(Self::handle_literal_field(self.table, f)),
            };
            result.map(Some)
        } else {
            Ok(None)
        }
    }

    /// Decodes a header field and returns the raw representation of it.
    ///
    /// This method may be useful for intermediaries
    /// (see: [6.2.3.  Literal Header Field Never Indexed]
    ///  (https://tools.ietf.org/html/rfc7541#section-6.2.3)).
    pub fn decode_raw_field(&mut self) -> Result<Option<RawHeaderField<'b>>> {
        if self.reader.eos() {
            Ok(None)
        } else {
            track!(RawHeaderField::decode(&mut self.reader)).map(Some)
        }
    }

    /// Returns the indexing table of this decoder.
    pub fn table(&self) -> &Table {
        &self.table
    }

    fn handle_indexed_field(table: &mut Table, field: IndexedHeaderField) -> Result<HeaderField> {
        track!(table.get(field.index()))
    }
    fn handle_literal_field<'c>(
        table: &'c mut Table,
        field: LiteralHeaderField<'b>,
    ) -> Result<HeaderField<'c>>
    where
        'b: 'c,
    {
        let (name, value, form) = field.unwrap();
        if let LiteralFieldForm::WithIndexing = form {
            let name = match name {
                FieldName::Index(index) => track!(table.get(index))?.name().to_owned(),
                FieldName::Name(name) => track!(name.into_plain_bytes())?.into_owned(),
            };
            let value = track!(value.into_plain_bytes())?.into_owned();

            if let Some(evicted) = table.dynamic_mut().push(name, value) {
                Ok(evicted)
            } else {
                let field = table.dynamic().entries()[0].as_borrowed();
                Ok(field)
            }
        } else {
            let name = match name {
                FieldName::Index(index) => track!(table.get(index))?.into_cow_name(),
                FieldName::Name(name) => track!(name.into_plain_bytes())?,
            };
            let value = track!(value.into_plain_bytes())?;
            Ok(HeaderField::from_cow(name, value))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_decode {
        ($decoder:expr, $key:expr, $value:expr) => {{
            let field = track_try_unwrap!($decoder.decode_field()).unwrap();
            assert_eq!(field.name(), $key);
            assert_eq!(field.value(), $value);
        }};
    }
    macro_rules! assert_eob {
        ($decoder:expr) => {
            let field = track_try_unwrap!($decoder.decode_field());
            assert!(field.is_none());
        };
    }

    #[test]
    /// https://tools.ietf.org/html/rfc7541#appendix-C.2.1
    fn literal_header_field_with_indexing() {
        let mut decoder = Decoder::new(4096);
        {
            let data;
            #[cfg_attr(rustfmt, rustfmt_skip)]
            {
                data = [
                    0x40, 0x0a, 0x63, 0x75, 0x73, 0x74, 0x6f, 0x6d, 0x2d,
                    0x6b, 0x65, 0x79, 0x0d, 0x63, 0x75, 0x73, 0x74, 0x6f,
                    0x6d, 0x2d, 0x68, 0x65, 0x61, 0x64, 0x65, 0x72,
                ];
            }
            let mut block = track_try_unwrap!(decoder.enter_header_block(&data[..]));
            assert_decode!(block, b"custom-key", b"custom-header");
            assert_eob!(block);
        }
        assert_eq!(decoder.table.dynamic().entries().len(), 1);
        assert_eq!(decoder.table.dynamic().size(), 55);
        assert_eq!(decoder.table.dynamic().entries()[0].name(), b"custom-key");
        assert_eq!(
            decoder.table.dynamic().entries()[0].value(),
            b"custom-header"
        );
    }

    #[test]
    /// https://tools.ietf.org/html/rfc7541#appendix-C.2.2
    fn literal_header_field_without_indexing() {
        let mut decoder = Decoder::new(4096);
        {
            let data;
            #[cfg_attr(rustfmt, rustfmt_skip)]
            {
                data = [
                    0x04, 0x0c, 0x2f, 0x73, 0x61, 0x6d, 0x70,
                    0x6c, 0x65, 0x2f, 0x70, 0x61, 0x74, 0x68
                ];
            }
            let mut block = track_try_unwrap!(decoder.enter_header_block(&data[..]));
            assert_decode!(block, b":path", b"/sample/path");
            assert_eob!(block);
        }
        assert_eq!(decoder.table.dynamic().entries().len(), 0);
    }

    #[test]
    /// https://tools.ietf.org/html/rfc7541#appendix-C.2.3
    fn literal_header_field_never_indexed() {
        let mut decoder = Decoder::new(4096);
        {
            let data;
            #[cfg_attr(rustfmt, rustfmt_skip)]
            {
                data = [
                    0x10, 0x08, 0x70, 0x61, 0x73, 0x73, 0x77, 0x6f, 0x72,
                    0x64, 0x06, 0x73, 0x65, 0x63, 0x72, 0x65, 0x74
                ];
            }
            let mut block = track_try_unwrap!(decoder.enter_header_block(&data[..]));
            assert_decode!(block, b"password", b"secret");
            assert_eob!(block);
        }
        assert_eq!(decoder.table.dynamic().entries().len(), 0);
    }

    #[test]
    /// https://tools.ietf.org/html/rfc7541#appendix-C.2.4
    fn indexed_header_field() {
        let mut decoder = Decoder::new(4096);
        {
            let data = [0x82];
            let mut block = track_try_unwrap!(decoder.enter_header_block(&data[..]));
            assert_decode!(block, b":method", b"GET");
            assert_eob!(block);
        }
        assert!(decoder.table.dynamic().entries().is_empty());
    }
}
