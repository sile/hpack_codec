hpack_codec
===========

[![hpack_codec](https://img.shields.io/crates/v/hpack_codec.svg)](https://crates.io/crates/hpack_codec)
[![Documentation](https://docs.rs/hpack_codec/badge.svg)](https://docs.rs/hpack_codec)
[![Build Status](https://travis-ci.org/sile/hpack_codec.svg?branch=master)](https://travis-ci.org/sile/hpack_codec)
[![Code Coverage](https://codecov.io/gh/sile/hpack_codec/branch/master/graph/badge.svg)](https://codecov.io/gh/sile/hpack_codec/branch/master)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Encoder and Decoder for [HPACK (RFC 7541)][HPACK].

[HPACk]: https://tools.ietf.org/html/rfc7541

[Documentation](https://docs.rs/hpack_codec)

Examples
--------

```rust
use hpack_codec::{Encoder, Decoder};
use hpack_codec::field::{HeaderField, LiteralHeaderField as Field};
use hpack_codec::table::{StaticEntry, Index};

// Encoding
let mut encoder = Encoder::new(4096);
let mut header = encoder.enter_header_block(Vec::new()).unwrap();
header.encode_field(StaticEntry::MethodGet).unwrap();
header.encode_field(Field::with_indexed_name(StaticEntry::Path, b"/hello")).unwrap();
header.encode_field(Field::new(b"foo", b"bar").with_indexing()).unwrap();
header.encode_field(Index::dynamic_table_offset() + 0).unwrap();
let encoded_data = header.finish();

// Decoding
let mut decoder = Decoder::new(4096);
let mut header = decoder.enter_header_block(&encoded_data[..]).unwrap();
assert_eq!(header.decode_field().unwrap(), HeaderField::new(b":method", b"GET").ok());
assert_eq!(header.decode_field().unwrap(), HeaderField::new(b":path", b"/hello").ok());
assert_eq!(header.decode_field().unwrap(), HeaderField::new(b"foo", b"bar").ok());
assert_eq!(header.decode_field().unwrap(), HeaderField::new(b"foo", b"bar").ok());
```
