use std::borrow::Cow;

use Context;

use Result;
use field::{self, Reader};

#[derive(Debug)]
pub struct HeaderField<'a> {
    pub name: Cow<'a, [u8]>,
    pub value: Cow<'a, [u8]>,
}

#[derive(Debug)]
pub struct Decoder {
    context: Context,
}
impl Decoder {
    pub fn new(max_dynamic_table_size: u16) -> Self {
        Decoder { context: Context::new(max_dynamic_table_size) }
    }
    pub fn decode<'a: 'b, 'b>(&'a mut self, reader: &mut Reader<'b>) -> Result<HeaderField<'b>> {
        loop {
            let field = track!(field::HeaderField::decode(reader))?;
            match field {
                field::HeaderField::Indexed(f) => return track!(self.handle_indexed_field(f)),
                field::HeaderField::Literal(f) => return track!(self.handle_literal_field(f)),
                field::HeaderField::Update(f) => {
                    track!(self.context.dynamic_table.set_max_size(f.max_size))?;
                }
            }
        }
    }

    fn handle_indexed_field(&mut self, field: field::IndexedHeaderField) -> Result<HeaderField> {
        let entry = track!(self.context.find_entry(field.index))?;
        Ok(HeaderField {
            name: Cow::Borrowed(entry.name),
            value: Cow::Borrowed(entry.value),
        })
    }

    fn handle_literal_field<'a: 'b, 'b>(
        &'a mut self,
        field: field::LiteralHeaderField<&'b [u8], &'b [u8]>,
    ) -> Result<HeaderField<'b>> {
        if let field::LiteralFieldForm::WithIndexing = field.form {
            let name = match field.name {
                field::FieldName::Index(index) => {
                    track!(self.context.find_entry(index))?.name.to_owned()
                }
                field::FieldName::Name(ref name) => track!(name.to_cow_str())?.into_owned(),
            };
            let value = track!(field.value.to_cow_str())?.into_owned();

            if let Some(entry) = self.context.dynamic_table.push_entry(name, value) {
                Ok(HeaderField {
                    name: Cow::Owned(entry.name),
                    value: Cow::Owned(entry.value),
                })
            } else {
                let entry = self.context.dynamic_table.last_entry().expect(
                    "Never fails",
                );
                Ok(HeaderField {
                    name: Cow::Borrowed(entry.name),
                    value: Cow::Borrowed(entry.value),
                })
            }
        } else {
            let name = match field.name {
                field::FieldName::Index(index) => {
                    Cow::Borrowed(track!(self.context.find_entry(index))?.name)
                }
                field::FieldName::Name(ref name) => track!(name.to_cow_str())?,
            };
            let value = track!(field.value.to_cow_str())?;
            Ok(HeaderField { name, value })
        }
    }
}

#[cfg(test)]
mod test {
    use field::Reader;
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
            let mut reader = Reader::new(&data[..]);
            let field = track_try_unwrap!(decoder.decode(&mut reader));
            assert!(reader.eos());
            assert_eq!(field.name.as_ref(), b"custom-key");
            assert_eq!(field.value.as_ref(), b"custom-header");
        }
        assert_eq!(decoder.context.dynamic_table.entries.len(), 1);
        assert_eq!(decoder.context.dynamic_table.size(), 55);
        assert_eq!(decoder.context.dynamic_table.entries[0].name, b"custom-key");
        assert_eq!(
            decoder.context.dynamic_table.entries[0].value,
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
            let mut reader = Reader::new(&data[..]);
            let field = track_try_unwrap!(decoder.decode(&mut reader));
            assert!(reader.eos());
            assert_eq!(field.name.as_ref(), b":path");
            assert_eq!(field.value.as_ref(), b"/sample/path");
        }
        assert_eq!(decoder.context.dynamic_table.entries.len(), 0);
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
            let mut reader = Reader::new(&data[..]);
            let field = track_try_unwrap!(decoder.decode(&mut reader));
            assert!(reader.eos());
            assert_eq!(field.name.as_ref(), b"password");
            assert_eq!(field.value.as_ref(), b"secret");
        }
        assert_eq!(decoder.context.dynamic_table.entries.len(), 0);
    }

    #[test]
    /// https://tools.ietf.org/html/rfc7541#appendix-C.2.4
    fn indexed_header_field() {
        let mut decoder = Decoder::new(4096);
        {
            let data = [0x82];
            let mut reader = Reader::new(&data[..]);
            let field = track_try_unwrap!(decoder.decode(&mut reader));
            assert!(reader.eos());
            assert_eq!(field.name.as_ref(), b":method");
            assert_eq!(field.value.as_ref(), b"GET");
        }
        assert!(decoder.context.dynamic_table.entries.is_empty());
    }
}
