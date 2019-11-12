use crate::io::SliceReader;
use crate::literal;
use crate::Result;
use std::io::Write;

#[derive(Debug)]
pub struct DynamicTableSizeUpdate {
    pub max_size: u16,
}
impl DynamicTableSizeUpdate {
    pub fn encode<W: Write>(&self, writer: W) -> Result<()> {
        track!(literal::encode_u16(writer, 0b001, 5, self.max_size))
    }
    pub fn decode(reader: &mut SliceReader) -> Result<Self> {
        let max_size = track!(literal::decode_u16(reader, 5))?.1;
        Ok(DynamicTableSizeUpdate { max_size })
    }
}
