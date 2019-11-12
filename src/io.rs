use crate::Result;
use byteorder::ReadBytesExt;
use std::cmp;
use std::io::{Read, Result as IoResult};
use trackable::error::Failed;

#[derive(Debug)]
pub struct SliceReader<'a> {
    slice: &'a [u8],
    offset: usize,
}
impl<'a> SliceReader<'a> {
    pub fn new(slice: &'a [u8]) -> Self {
        SliceReader { slice, offset: 0 }
    }
    pub fn eos(&self) -> bool {
        debug_assert!(self.offset <= self.slice.len());
        self.offset == self.slice.len()
    }
    pub fn peek_u8(&mut self) -> Result<u8> {
        let value = track_io!(self.read_u8())?;
        self.unread();
        Ok(value)
    }
    pub fn consume(&mut self, size: usize) {
        self.offset = cmp::min(self.offset + size, self.slice.len());
    }
    pub fn read_slice(&mut self, size: usize) -> Result<&'a [u8]> {
        track_assert!(
            self.offset + size <= self.slice.len(),
            Failed,
            "offset={}, size={}, slice.len={}",
            self.offset,
            size,
            self.slice.len()
        );
        let slice = &self.slice[self.offset..self.offset + size];
        self.offset += size;
        Ok(slice)
    }

    fn unread(&mut self) {
        debug_assert!(self.offset > 0);
        self.offset -= 1;
    }
}
impl<'a> Read for SliceReader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        let size = (&self.slice[self.offset..]).read(buf)?;
        self.offset += size;
        Ok(size)
    }
}
