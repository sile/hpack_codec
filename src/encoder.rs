use crate::field::{FieldName, LiteralFieldForm, RawHeaderField};
use crate::signal::DynamicTableSizeUpdate;
use crate::table::Table;
use crate::Result;
use std::io::Write;

/// HPACK Encoder.
#[derive(Debug)]
pub struct Encoder {
    table: Table,
    dynamic_table_size_updates: Vec<u16>,
}
impl Encoder {
    /// Makes a new `Encoder` instance.
    pub fn new(max_dynamic_table_size: u16) -> Self {
        Encoder {
            table: Table::new(max_dynamic_table_size),
            dynamic_table_size_updates: Vec::new(),
        }
    }

    /// Returns the indexing table of this decoder.
    pub fn table(&self) -> &Table {
        &self.table
    }

    /// Sets the hard limit of the dynamic table size of this encoder.
    pub fn set_dynamic_table_size_hard_limit(&mut self, max_size: u16) {
        let old = self.table.dynamic().size_soft_limit();
        self.table.dynamic_mut().set_size_hard_limit(max_size);
        if old != self.table.dynamic().size_soft_limit() {
            self.dynamic_table_size_updates.push(old);
        }
    }

    /// Sets the soft limit of the dynamic table size of this encoder.
    ///
    /// # Errors
    ///
    /// If `max_size` exceeds the hard limit of this, an error will be returned.
    pub fn set_dynamic_table_size_soft_limit(&mut self, max_size: u16) -> Result<()> {
        let old = self.table.dynamic().size_soft_limit();
        track!(self.table.dynamic_mut().set_size_soft_limit(max_size))?;
        if old != self.table.dynamic().size_soft_limit() {
            self.dynamic_table_size_updates.push(old);
        }
        Ok(())
    }

    /// Returns a `HeaderBlockEncoder` instance for encoding header fields to the `block`.
    pub fn enter_header_block<W: Write>(&mut self, mut block: W) -> Result<HeaderBlockEncoder<W>> {
        for max_size in self.dynamic_table_size_updates.drain(..) {
            let update = DynamicTableSizeUpdate { max_size };
            track!(update.encode(&mut block))?;
        }
        Ok(HeaderBlockEncoder {
            table: &mut self.table,
            block,
        })
    }
}

/// Header Block Encoder.
#[derive(Debug)]
pub struct HeaderBlockEncoder<'a, W> {
    table: &'a mut Table,
    block: W,
}
impl<'a, W: Write> HeaderBlockEncoder<'a, W> {
    /// Encodes a header field.
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
                        FieldName::Name(ref name) => track!(name.to_plain_bytes())?.into_owned(),
                    };
                    let value = track!(field.value().to_plain_bytes())?.into_owned();
                    self.table.dynamic_mut().push(name, value);
                }
            }
        }
        track!(field.encode(&mut self.block))?;
        Ok(())
    }

    /// Finishes the encoding for the header block.
    pub fn finish(self) -> W {
        self.block
    }

    /// Returns the indexing table of this decoder.
    pub fn table(&self) -> &Table {
        &self.table
    }
}
