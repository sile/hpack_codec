use std::borrow::Cow;
use trackable::error::Failed;

use Result;
use field::{HeaderField, PlainHeaderField, IndexedHeaderField, LiteralHeaderField,
            LiteralFieldForm, FieldName};
use io::SliceReader;
use signal::DynamicTableSizeUpdate;
use table::Table;

#[derive(Debug)]
pub struct Decoder {
    table: Table,
}
impl Decoder {
    pub fn table(&self) -> &Table {
        &self.table
    }
    pub fn new(max_dynamic_table_size: u16) -> Self {
        Decoder { table: Table::new(max_dynamic_table_size) }
    }
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
    pub fn enter_header_block<'a, 'b>(
        &'a mut self,
        block: &'b [u8],
    ) -> Result<HeaderBlockDecoder<'a, 'b>> {
        let mut reader = SliceReader::new(block);
        while (track!(reader.peek_u8())? & 0b0010_0000) != 0 {
            let update = track!(DynamicTableSizeUpdate::decode(&mut reader))?;
            track!(self.table.dynamic_mut().set_size_soft_limit(
                update.max_size,
            ))?;
        }
        Ok(HeaderBlockDecoder {
            table: &mut self.table,
            reader,
        })
    }
}

#[derive(Debug)]
pub struct HeaderBlockDecoder<'a, 'b> {
    table: &'a mut Table,
    reader: SliceReader<'b>,
}
impl<'a, 'b: 'a> HeaderBlockDecoder<'a, 'b> {
    pub fn decode_field(&mut self) -> Result<Option<HeaderField<'b>>> {
        if self.reader.eos() {
            Ok(None)
        } else {
            track!(HeaderField::decode(&mut self.reader)).map(Some)
        }
    }
    pub fn decode_plain_field(&mut self) -> Result<Option<PlainHeaderField<'a>>> {
        if let Some(field) = track!(self.decode_field())? {
            let result = match field {
                HeaderField::Indexed(f) => track!(Self::handle_indexed_field(self.table, f)),
                HeaderField::Literal(f) => track!(Self::handle_literal_field(self.table, f)),
            };
            result.map(Some)
        } else {
            Ok(None)
        }
    }

    // TODO
    pub fn eos(&self) -> bool {
        self.reader.eos()
    }
    fn handle_indexed_field(
        table: &'a mut Table,
        field: IndexedHeaderField,
    ) -> Result<PlainHeaderField<'a>> {
        let entry = track!(table.get(field.index()))?;
        Ok(PlainHeaderField {
            name: Cow::Borrowed(entry.name),
            value: Cow::Borrowed(entry.value),
        })
    }
    fn handle_literal_field(
        table: &'a mut Table,
        field: LiteralHeaderField<'b>,
    ) -> Result<PlainHeaderField<'a>> {
        if let LiteralFieldForm::WithIndexing = field.form {
            let name = match field.name {
                FieldName::Index(index) => track!(table.get(index))?.name.to_owned(),
                FieldName::Name(name) => track!(name.into_raw())?.into_owned(),
            };
            let value = track!(field.value.into_raw())?.into_owned();

            if let Some(entry) = table.dynamic_mut().push(name, value) {
                Ok(PlainHeaderField {
                    name: Cow::Owned(entry.name),
                    value: Cow::Owned(entry.value),
                })
            } else {
                let entry = table.dynamic().entries()[0].as_ref();
                Ok(PlainHeaderField {
                    name: Cow::Borrowed(entry.name),
                    value: Cow::Borrowed(entry.value),
                })
            }
        } else {
            let name = match field.name {
                FieldName::Index(index) => Cow::Borrowed(track!(table.get(index))?.name),
                FieldName::Name(name) => track!(name.into_raw())?,
            };
            let value = track!(field.value.into_raw())?;
            Ok(PlainHeaderField { name, value })
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    macro_rules! assert_decode {
        ($decoder:expr, $key:expr, $value:expr) => {
            {
                let field = track_try_unwrap!($decoder.decode_plain_field()).unwrap();
                assert_eq!(field.name.as_ref(), $key);
                assert_eq!(field.value.as_ref(), $value);
            }
        }
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
            assert!(block.eos());
        }
        assert_eq!(decoder.table.dynamic().entries().len(), 1);
        assert_eq!(decoder.table.dynamic().size(), 55);
        assert_eq!(decoder.table.dynamic().entries()[0].name, b"custom-key");
        assert_eq!(decoder.table.dynamic().entries()[0].value, b"custom-header");
    }

    // #[test]
    // /// https://tools.ietf.org/html/rfc7541#appendix-C.2.2
    // fn literal_header_field_without_indexing() {
    //     let mut decoder = Decoder::new(4096);
    //     {
    //         let data;
    //         #[cfg_attr(rustfmt, rustfmt_skip)]
    //         {
    //             data = [
    //                 0x04, 0x0c, 0x2f, 0x73, 0x61, 0x6d, 0x70,
    //                 0x6c, 0x65, 0x2f, 0x70, 0x61, 0x74, 0x68
    //             ];
    //         }
    //         let mut reader = SliceReader::new(&data[..]);
    //         let field = track_try_unwrap!(decoder.decode(&mut reader));
    //         assert!(reader.eos());
    //         assert_eq!(field.name.as_ref(), b":path");
    //         assert_eq!(field.value.as_ref(), b"/sample/path");
    //     }
    //     assert_eq!(decoder.table.dynamic().entries().len(), 0);
    // }

    // #[test]
    // /// https://tools.ietf.org/html/rfc7541#appendix-C.2.3
    // fn literal_header_field_never_indexed() {
    //     let mut decoder = Decoder::new(4096);
    //     {
    //         let data;
    //         #[cfg_attr(rustfmt, rustfmt_skip)]
    //         {
    //             data = [
    //                 0x10, 0x08, 0x70, 0x61, 0x73, 0x73, 0x77, 0x6f, 0x72,
    //                 0x64, 0x06, 0x73, 0x65, 0x63, 0x72, 0x65, 0x74
    //             ];
    //         }
    //         let mut reader = SliceReader::new(&data[..]);
    //         let field = track_try_unwrap!(decoder.decode(&mut reader));
    //         assert!(reader.eos());
    //         assert_eq!(field.name.as_ref(), b"password");
    //         assert_eq!(field.value.as_ref(), b"secret");
    //     }
    //     assert_eq!(decoder.table.dynamic().entries().len(), 0);
    // }

    // #[test]
    // /// https://tools.ietf.org/html/rfc7541#appendix-C.2.4
    // fn indexed_header_field() {
    //     let mut decoder = Decoder::new(4096);
    //     {
    //         let data = [0x82];
    //         let mut reader = SliceReader::new(&data[..]);
    //         let field = track_try_unwrap!(decoder.decode(&mut reader));
    //         assert!(reader.eos());
    //         assert_eq!(field.name.as_ref(), b":method");
    //         assert_eq!(field.value.as_ref(), b"GET");
    //     }
    //     assert!(decoder.table.dynamic().entries().is_empty());
    // }
}
