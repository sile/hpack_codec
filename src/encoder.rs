use std::io::Write;

use {Result, Context};
use field::{self, Index};

#[derive(Debug)]
pub struct Encoder {
    context: Context,
    dynamic_table_size_updates: Vec<u16>,
}
impl Encoder {
    pub fn new(context: Context) -> Self {
        Encoder {
            context,
            dynamic_table_size_updates: Vec::new(),
        }
    }
    pub fn context(&self) -> &Context {
        &self.context
    }
    pub fn set_dynamic_table_size_hard_limit(&mut self, max_size: u16) {
        let old = self.context.dynamic_table.max_table_size;
        self.context.dynamic_table.set_size_limit(max_size);
        if old != self.context.dynamic_table.max_table_size {
            self.dynamic_table_size_updates.push(old);
        }
    }
    pub fn set_dynamic_table_size_soft_limit(&mut self, max_size: u16) -> Result<()> {
        let old = self.context.dynamic_table.max_table_size;
        track!(self.context.dynamic_table.set_max_size(max_size))?;
        if old != self.context.dynamic_table.max_table_size {
            self.dynamic_table_size_updates.push(old);
        }
        Ok(())
    }
    pub fn enter_header_block<W: Write>(&mut self, mut writer: W) -> Result<HeaderBlockEncoder<W>> {
        for max_size in self.dynamic_table_size_updates.drain(..) {
            let entry = field::HeaderField::Update::<Vec<u8>, Vec<u8>>(
                field::DynamicTableSizeUpdate { max_size },
            );
            track!(entry.encode(&mut writer))?;
        }
        Ok(HeaderBlockEncoder {
            context: &mut self.context,
            writer,
        })
    }
}

#[derive(Debug)]
pub struct HeaderBlockEncoder<'a, W> {
    context: &'a mut Context,
    writer: W,
}
impl<'a, W: Write> HeaderBlockEncoder<'a, W> {
    pub fn context(&self) -> &Context {
        &self.context
    }

    pub fn encode_indexed_header_field(&mut self, index: u16) -> Result<()> {
        track!(self.context.validate_entry_index(index))?;
        let field = field::HeaderField::Indexed::<Vec<u8>, Vec<u8>>(
            field::IndexedHeaderField { index: Index(index) },
        );
        track!(field.encode(&mut self.writer))?;
        Ok(())
    }

    pub fn encode_literal_header_field<N, V>(
        &mut self,
        field: field::LiteralHeaderField<N, V>,
    ) -> Result<()>
    where
        N: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        if let field::FieldName::Index(index) = field.name {
            track!(self.context.validate_entry_index(index.as_u16()))?;
        };
        if let field::LiteralFieldForm::WithIndexing = field.form {
            let name = match field.name {
                field::FieldName::Index(index) => {
                    let entry = track!(self.context.find_entry(index))?;
                    entry.name.to_owned()
                }
                field::FieldName::Name(ref name) => track!(name.to_vec())?,
            };
            let value = track!(field.value.to_vec())?;
            self.context.dynamic_table.push_entry(name, value);
        }
        track!(field.encode(&mut self.writer))?;
        Ok(())
    }
    pub fn finish(self) -> W {
        self.writer
    }
}
