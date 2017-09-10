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
    pub fn decode<'a, 'b>(&'a mut self, reader: Reader<'b>) -> Decode<'a, 'b> {
        Decode {
            context: &mut self.context,
            reader,
        }
    }
}

#[derive(Debug)]
pub struct Decode<'a, 'b> {
    context: &'a mut Context,
    reader: Reader<'b>,
}
impl<'a: 'b, 'b> Iterator for Decode<'a, 'b> {
    type Item = Result<HeaderField<'b>>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.reader.eos() {
            None
        } else {
            let result = field::HeaderField::decode(&mut self.reader);
            unimplemented!("{:?}", result)
        }
    }
}
