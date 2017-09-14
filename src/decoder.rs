use std::borrow::Cow;

use Result;
use field::LiteralFieldForm;
use io::SliceReader;
use table::Table;

#[derive(Debug)]
pub struct HeaderField<'a> {
    pub name: Cow<'a, [u8]>,
    pub value: Cow<'a, [u8]>,
}

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
    pub fn decode<'a, 'b: 'a>(
        &'a mut self,
        reader: &mut SliceReader<'b>,
    ) -> Result<HeaderField<'a>> {
        loop {
            let field = track!(HeaderField::decode(reader))?;
            match field {
                HeaderField::Indexed(f) => return track!(self.handle_indexed_field(f)),
                HeaderField::Literal(f) => return track!(self.handle_literal_field(f)),
                HeaderField::Update(f) => {
                    track!(self.table.dynamic_mut().set_size_soft_limit(f.max_size))?;
                }
            }
        }
    }

    fn handle_indexed_field(&mut self, field: IndexedHeaderField) -> Result<HeaderField> {
        let entry = track!(self.table.get(field.index))?;
        Ok(HeaderField {
            name: Cow::Borrowed(entry.name),
            value: Cow::Borrowed(entry.value),
        })
    }

    fn handle_literal_field<'a: 'b, 'b>(
        &'a mut self,
        field: LiteralHeaderField<&'b [u8], &'b [u8]>,
    ) -> Result<HeaderField<'b>> {
        if let LiteralFieldForm::WithIndexing = field.form {
            let name = match field.name {
                FieldName::Index(index) => track!(self.table.get(index))?.name.to_owned(),
                FieldName::Name(ref name) => track!(name.to_cow_str())?.into_owned(),
            };
            let value = track!(field.value.to_cow_str())?.into_owned();

            if let Some(entry) = self.table.dynamic_mut().push(name, value) {
                Ok(HeaderField {
                    name: Cow::Owned(entry.name),
                    value: Cow::Owned(entry.value),
                })
            } else {
                let entry = self.table.dynamic().entries()[0].as_ref();
                Ok(HeaderField {
                    name: Cow::Borrowed(entry.name),
                    value: Cow::Borrowed(entry.value),
                })
            }
        } else {
            let name = match field.name {
                FieldName::Index(index) => Cow::Borrowed(track!(self.table.get(index))?.name),
                FieldName::Name(ref name) => track!(name.to_cow_str())?,
            };
            let value = track!(field.value.to_cow_str())?;
            Ok(HeaderField { name, value })
        }
    }
}

#[cfg(test)]
mod test {
    use io::SliceReader;
    use super::*;

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
            let mut reader = SliceReader::new(&data[..]);
            let field = track_try_unwrap!(decoder.decode(&mut reader));
            assert!(reader.eos());
            assert_eq!(field.name.as_ref(), b"custom-key");
            assert_eq!(field.value.as_ref(), b"custom-header");
        }
        assert_eq!(decoder.table.dynamic().entries().len(), 1);
        assert_eq!(decoder.table.dynamic().size(), 55);
        assert_eq!(decoder.table.dynamic().entries()[0].name, b"custom-key");
        assert_eq!(decoder.table.dynamic().entries()[0].value, b"custom-header");
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
            let mut reader = SliceReader::new(&data[..]);
            let field = track_try_unwrap!(decoder.decode(&mut reader));
            assert!(reader.eos());
            assert_eq!(field.name.as_ref(), b":path");
            assert_eq!(field.value.as_ref(), b"/sample/path");
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
            let mut reader = SliceReader::new(&data[..]);
            let field = track_try_unwrap!(decoder.decode(&mut reader));
            assert!(reader.eos());
            assert_eq!(field.name.as_ref(), b"password");
            assert_eq!(field.value.as_ref(), b"secret");
        }
        assert_eq!(decoder.table.dynamic().entries().len(), 0);
    }

    #[test]
    /// https://tools.ietf.org/html/rfc7541#appendix-C.2.4
    fn indexed_header_field() {
        let mut decoder = Decoder::new(4096);
        {
            let data = [0x82];
            let mut reader = SliceReader::new(&data[..]);
            let field = track_try_unwrap!(decoder.decode(&mut reader));
            assert!(reader.eos());
            assert_eq!(field.name.as_ref(), b":method");
            assert_eq!(field.value.as_ref(), b"GET");
        }
        assert!(decoder.table.dynamic().entries().is_empty());
    }
}
