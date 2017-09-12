extern crate byteorder;
#[macro_use]
extern crate trackable;

use trackable::error::Failed;

pub use encoder::{Encoder, HeaderBlockEncoder};
pub use name::Name;

use field::Index;

macro_rules! track_io {
    ($e:expr) => {
        $e.map_err(|e| {
            use ::trackable::error::{Failed,ErrorKindExt};
            Failed.cause(e)
        })
    }
}

pub mod table;

pub mod decoder; // TODO: private
mod encoder;
pub mod huffman; // TODO: private
pub mod field; // TODO: private
pub mod literal; // TODO: private
mod name;

pub type Error = trackable::error::TrackableError<trackable::error::Failed>;
pub type Result<T> = ::std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Context {
    dynamic_table: table::DynamicTable,
}
impl Context {
    pub fn new(max_table_size: u16) -> Self {
        Context { dynamic_table: table::DynamicTable::new(max_table_size) }
    }
    pub fn dynamic_table(&self) -> &table::DynamicTable {
        &self.dynamic_table
    }
    pub fn validate_entry_index(&self, index: u16) -> Result<()> {
        let max_index = table::STATIC_TABLE.len() + self.dynamic_table.entries().len();
        track_assert!(
            index as usize <= max_index,
            Failed,
            "Too large index: {} (max={})",
            index,
            max_index
        );
        track_assert_ne!(index, 0, Failed);
        Ok(())
    }

    pub fn find_entry(&self, index: Index) -> Result<table::Entry<&[u8]>> {
        debug_assert_ne!(index.as_u16(), 0);
        let index = index.as_u16() as usize - 1;
        if index < 61 {
            Ok(table::STATIC_TABLE[index].clone())
        } else {
            let entry = track_assert_some!(
                self.dynamic_table.entries().get(index - 61),
                Failed,
                "Too large index: {}",
                index + 1
            );
            Ok(entry.as_ref())
        }
    }
}
