use std::io::Write;

use Result;
use field::{self, Index};
use table::Table;

#[derive(Debug)]
pub struct Encoder {
    table: Table,
    dynamic_table_size_updates: Vec<u16>,
}
impl Encoder {
    pub fn new(table: Table) -> Self {
        Encoder {
            table,
            dynamic_table_size_updates: Vec::new(),
        }
    }
    pub fn table(&self) -> &Table {
        &self.table
    }
    pub fn set_dynamic_table_size_hard_limit(&mut self, max_size: u16) {
        let old = self.table.dynamic().size_soft_limit();
        self.table.dynamic_mut().set_size_hard_limit(max_size);
        if old != self.table.dynamic().size_soft_limit() {
            self.dynamic_table_size_updates.push(old);
        }
    }
    pub fn set_dynamic_table_size_soft_limit(&mut self, max_size: u16) -> Result<()> {
        let old = self.table.dynamic().size_soft_limit();
        track!(self.table.dynamic_mut().set_size_soft_limit(max_size))?;
        if old != self.table.dynamic().size_soft_limit() {
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
            table: &mut self.table,
            writer,
        })
    }
}

#[derive(Debug)]
pub struct HeaderBlockEncoder<'a, W> {
    table: &'a mut Table,
    writer: W,
}
impl<'a, W: Write> HeaderBlockEncoder<'a, W> {
    pub fn table(&self) -> &Table {
        &self.table
    }

    pub fn encode_indexed_header_field(&mut self, index: u16) -> Result<()> {
        track!(self.table.validate_index(index))?;
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
            track!(self.table.validate_index(index.as_u16()))?;
        };
        if let field::LiteralFieldForm::WithIndexing = field.form {
            let name = match field.name {
                field::FieldName::Index(index) => {
                    let entry = track!(self.table.get(index))?;
                    entry.name.to_owned()
                }
                field::FieldName::Name(ref name) => track!(name.to_vec())?,
            };
            let value = track!(field.value.to_vec())?;
            self.table.dynamic_mut().push(name, value);
        }
        track!(field.encode(&mut self.writer))?;
        Ok(())
    }
    pub fn finish(self) -> W {
        self.writer
    }
}
