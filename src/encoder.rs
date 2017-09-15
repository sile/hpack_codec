use std::borrow::Cow;
use std::io::Write;

use Result;
use field::{RawHeaderField, FieldName, LiteralFieldForm};
use signal::DynamicTableSizeUpdate;
use table::Table;

#[derive(Debug)]
pub struct Encoder {
    table: Table,
    dynamic_table_size_updates: Vec<u16>,
}
impl Encoder {
    pub fn new(max_dynamic_table_size: u16) -> Self {
        Encoder {
            table: Table::new(max_dynamic_table_size),
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
            let update = DynamicTableSizeUpdate { max_size };
            track!(update.encode(&mut writer))?;
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
    pub fn encode_field<'b, F>(&'b mut self, field: F) -> Result<()>
    where
        F: Into<RawHeaderField<'b>>,
    {
        let field = field.into();
        match field {
            RawHeaderField::Indexed(ref field) => {
                track!(self.table.validate_index(field.index()))?;
            }
            RawHeaderField::Literal(ref field) => {
                if let FieldName::Index(index) = *field.name() {
                    track!(self.table.validate_index(index))?;
                };
                if let LiteralFieldForm::WithIndexing = field.form() {
                    let name = match *field.name() {
                        FieldName::Index(index) => {
                            let entry = track!(self.table.get(index))?;
                            entry.name().to_owned()
                        }
                        FieldName::Name(ref name) => track!(name.to_vec())?,
                    };
                    let value = track!(field.value().to_vec())?;
                    self.table.dynamic_mut().push(name, value);
                }
            }
        }
        track!(field.encode(&mut self.writer))?;
        Ok(())
    }
    pub fn finish(self) -> W {
        self.writer
    }
}
