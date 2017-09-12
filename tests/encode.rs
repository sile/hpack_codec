extern crate hpack_codec;
#[macro_use]
extern crate trackable;

use hpack_codec::{Context, Encoder, Name};
use hpack_codec::field::LiteralHeaderFieldBuilder;
use hpack_codec::literal::Encoding;

#[test]
/// https://tools.ietf.org/html/rfc7541#appendix-C.3
fn request_examples_without_huffman_coding() {
    let mut encoder = Encoder::new(Context::new(4096));

    // C.3.1. First Request
    {
        let mut block = track_try_unwrap!(encoder.enter_header_block(Vec::new()));
        track_try_unwrap!(block.encode_indexed_header_field(2));
        track_try_unwrap!(block.encode_indexed_header_field(6));
        track_try_unwrap!(block.encode_indexed_header_field(4));
        track_try_unwrap!(block.encode_literal_header_field(
            LiteralHeaderFieldBuilder::with_indexing().finish(
                Name::Authority,
                b"www.example.com",
            ),
        ));

        let expected;
        #[cfg_attr(rustfmt, rustfmt_skip)]
        {
            expected = [
                0x82, 0x86, 0x84, 0x41, 0x0f, 0x77, 0x77, 0x77, 0x2e, 0x65,
                0x78, 0x61, 0x6d, 0x70, 0x6c, 0x65, 0x2e, 0x63, 0x6f, 0x6d
            ];
        }
        assert_eq!(block.finish(), &expected[..]);
    }
    assert_eq!(encoder.context().dynamic_table().size(), 57);

    // C.3.2. Second Request
    {
        let mut block = track_try_unwrap!(encoder.enter_header_block(Vec::new()));
        track_try_unwrap!(block.encode_indexed_header_field(2));
        track_try_unwrap!(block.encode_indexed_header_field(6));
        track_try_unwrap!(block.encode_indexed_header_field(4));
        track_try_unwrap!(block.encode_indexed_header_field(62));
        track_try_unwrap!(block.encode_literal_header_field(
            LiteralHeaderFieldBuilder::with_indexing().finish(
                Name::CacheControl,
                b"no-cache",
            ),
        ));

        let expected;
        #[cfg_attr(rustfmt, rustfmt_skip)]
        {
            expected = [
                0x82, 0x86, 0x84, 0xbe, 0x58, 0x08, 0x6e,
                0x6f, 0x2d, 0x63, 0x61, 0x63, 0x68, 0x65
            ];
        }
        assert_eq!(block.finish(), &expected[..]);
    }
    assert_eq!(encoder.context().dynamic_table().size(), 110);

    // C.3.3. Third Request
    {
        let mut block = track_try_unwrap!(encoder.enter_header_block(Vec::new()));
        track_try_unwrap!(block.encode_indexed_header_field(2));
        track_try_unwrap!(block.encode_indexed_header_field(7));
        track_try_unwrap!(block.encode_indexed_header_field(5));
        track_try_unwrap!(block.encode_indexed_header_field(63));
        track_try_unwrap!(block.encode_literal_header_field(
            LiteralHeaderFieldBuilder::with_indexing().finish(
                Name::Name(b"custom-key"),
                b"custom-value",
            ),
        ));

        let expected;
        #[cfg_attr(rustfmt, rustfmt_skip)]
        {
            expected = [
                0x82, 0x87, 0x85, 0xbf, 0x40, 0x0a, 0x63, 0x75, 0x73, 0x74, 0x6f,
                0x6d, 0x2d, 0x6b, 0x65, 0x79, 0x0c, 0x63, 0x75, 0x73, 0x74, 0x6f,
                0x6d, 0x2d, 0x76, 0x61, 0x6c, 0x75 ,0x65
            ];
        }
        assert_eq!(block.finish(), &expected[..]);
    }
    assert_eq!(encoder.context().dynamic_table().size(), 164);
}

#[test]
/// https://tools.ietf.org/html/rfc7541#appendix-C.4
fn request_examples_with_huffman_coding() {
    let mut encoder = Encoder::new(Context::new(4096));

    // C.4.1. First Request
    {
        let mut block = track_try_unwrap!(encoder.enter_header_block(Vec::new()));
        track_try_unwrap!(block.encode_indexed_header_field(2));
        track_try_unwrap!(block.encode_indexed_header_field(6));
        track_try_unwrap!(block.encode_indexed_header_field(4));
        track_try_unwrap!(
            block.encode_literal_header_field(
                LiteralHeaderFieldBuilder::with_indexing()
                    .value_encoding(Encoding::Huffman)
                    .finish(Name::Authority, b"www.example.com"),
            )
        );

        let expected;
        #[cfg_attr(rustfmt, rustfmt_skip)]
        {
            expected = [
                0x82, 0x86, 0x84, 0x41, 0x8c, 0xf1, 0xe3, 0xc2,
                0xe5, 0xf2, 0x3a, 0x6b, 0xa0, 0xab, 0x90, 0xf4, 0xff             
            ];
        }
        assert_eq!(block.finish(), &expected[..]);
    }
    assert_eq!(encoder.context().dynamic_table().size(), 57);

    // C.4.2. Second Request
    {
        let mut block = track_try_unwrap!(encoder.enter_header_block(Vec::new()));
        track_try_unwrap!(block.encode_indexed_header_field(2));
        track_try_unwrap!(block.encode_indexed_header_field(6));
        track_try_unwrap!(block.encode_indexed_header_field(4));
        track_try_unwrap!(block.encode_indexed_header_field(62));
        track_try_unwrap!(
            block.encode_literal_header_field(
                LiteralHeaderFieldBuilder::with_indexing()
                    .value_encoding(Encoding::Huffman)
                    .finish(Name::CacheControl, b"no-cache"),
            )
        );

        let expected;
        #[cfg_attr(rustfmt, rustfmt_skip)]
        {
            expected = [
                0x82, 0x86, 0x84, 0xbe, 0x58, 0x86, 0xa8, 0xeb, 0x10, 0x64, 0x9c, 0xbf
            ];
        }
        assert_eq!(block.finish(), &expected[..]);
    }
    assert_eq!(encoder.context().dynamic_table().size(), 110);

    // C.4.3. Third Request
    {
        let mut block = track_try_unwrap!(encoder.enter_header_block(Vec::new()));
        track_try_unwrap!(block.encode_indexed_header_field(2));
        track_try_unwrap!(block.encode_indexed_header_field(7));
        track_try_unwrap!(block.encode_indexed_header_field(5));
        track_try_unwrap!(block.encode_indexed_header_field(63));
        track_try_unwrap!(
            block.encode_literal_header_field(
                LiteralHeaderFieldBuilder::with_indexing()
                    .name_encoding(Encoding::Huffman)
                    .value_encoding(Encoding::Huffman)
                    .finish(Name::Name(b"custom-key"), b"custom-value"),
            )
        );

        let expected;
        #[cfg_attr(rustfmt, rustfmt_skip)]
        {
            expected = [
                0x82, 0x87, 0x85, 0xbf, 0x40, 0x88, 0x25, 0xa8, 0x49,
                0xe9, 0x5b, 0xa9, 0x7d, 0x7f, 0x89, 0x25, 0xa8, 0x49,
                0xe9, 0x5b, 0xb8, 0xe8, 0xb4, 0xbf
            ];
        }
        assert_eq!(block.finish(), &expected[..]);
    }
    assert_eq!(encoder.context().dynamic_table().size(), 164);
}

#[test]
/// https://tools.ietf.org/html/rfc7541#appendix-C.5
fn response_examples_without_huffman_coding() {
    let mut encoder = Encoder::new(Context::new(256));

    // C.5.1. First Request
    {
        let mut block = track_try_unwrap!(encoder.enter_header_block(Vec::new()));
        track_try_unwrap!(block.encode_literal_header_field(
            LiteralHeaderFieldBuilder::with_indexing().finish(
                Name::Status,
                b"302",
            ),
        ));
        track_try_unwrap!(block.encode_literal_header_field(
            LiteralHeaderFieldBuilder::with_indexing().finish(
                Name::CacheControl,
                b"private",
            ),
        ));
        track_try_unwrap!(block.encode_literal_header_field(
            LiteralHeaderFieldBuilder::with_indexing().finish(
                Name::Date,
                b"Mon, 21 Oct 2013 20:13:21 GMT",
            ),
        ));
        track_try_unwrap!(block.encode_literal_header_field(
            LiteralHeaderFieldBuilder::with_indexing().finish(
                Name::Location,
                b"https://www.example.com",
            ),
        ));

        let expected;
        #[cfg_attr(rustfmt, rustfmt_skip)]
        {
            expected = [
                0x48, 0x03, 0x33, 0x30, 0x32, 0x58, 0x07, 0x70, 0x72, 0x69,
                0x76, 0x61, 0x74, 0x65, 0x61, 0x1d, 0x4d, 0x6f, 0x6e, 0x2c,
                0x20, 0x32, 0x31, 0x20, 0x4f, 0x63, 0x74, 0x20, 0x32, 0x30,
                0x31, 0x33, 0x20, 0x32, 0x30, 0x3a, 0x31, 0x33, 0x3a, 0x32,
                0x31, 0x20, 0x47, 0x4d, 0x54, 0x6e, 0x17, 0x68, 0x74, 0x74,
                0x70, 0x73, 0x3a, 0x2f, 0x2f, 0x77, 0x77, 0x77, 0x2e, 0x65,
                0x78, 0x61, 0x6d, 0x70, 0x6c, 0x65, 0x2e, 0x63, 0x6f, 0x6d
            ];
        }
        assert_eq!(block.finish(), &expected[..]);
    }
    assert_eq!(encoder.context().dynamic_table().size(), 222);

    // C.5.2. Second Request
    {
        let mut block = track_try_unwrap!(encoder.enter_header_block(Vec::new()));
        track_try_unwrap!(block.encode_literal_header_field(
            LiteralHeaderFieldBuilder::with_indexing().finish(
                Name::Status,
                b"307",
            ),
        ));
        track_try_unwrap!(block.encode_indexed_header_field(65));
        track_try_unwrap!(block.encode_indexed_header_field(64));
        track_try_unwrap!(block.encode_indexed_header_field(63));

        let expected;
        #[cfg_attr(rustfmt, rustfmt_skip)]
        {
            expected = [
                0x48, 0x03, 0x33, 0x30, 0x37, 0xc1, 0xc0, 0xbf
            ];
        }
        assert_eq!(block.finish(), &expected[..]);
    }
    assert_eq!(encoder.context().dynamic_table().size(), 222);

    // C.5.3. Third Request
    {
        let mut block = track_try_unwrap!(encoder.enter_header_block(Vec::new()));
        track_try_unwrap!(block.encode_indexed_header_field(8));
        track_try_unwrap!(block.encode_indexed_header_field(65));
        track_try_unwrap!(block.encode_literal_header_field(
            LiteralHeaderFieldBuilder::with_indexing().finish(
                Name::Date,
                b"Mon, 21 Oct 2013 20:13:22 GMT",
            ),
        ));
        track_try_unwrap!(block.encode_indexed_header_field(64));
        track_try_unwrap!(block.encode_literal_header_field(
            LiteralHeaderFieldBuilder::with_indexing().finish(
                Name::ContentEncoding,
                b"gzip",
            ),
        ));
        track_try_unwrap!(block.encode_literal_header_field(
            LiteralHeaderFieldBuilder::with_indexing().finish(
                Name::SetCookie,
                &b"foo=ASDJKHQKBZXOQWEOPIUAXQWEOIU; max-age=3600; version=1"[..],
            ),
        ));

        let expected;
        #[cfg_attr(rustfmt, rustfmt_skip)]
        {
            expected = [
                0x88, 0xc1, 0x61, 0x1d, 0x4d, 0x6f, 0x6e, 0x2c,
                0x20, 0x32, 0x31, 0x20, 0x4f, 0x63, 0x74, 0x20,
                0x32, 0x30, 0x31, 0x33, 0x20, 0x32, 0x30, 0x3a,
                0x31, 0x33, 0x3a, 0x32, 0x32, 0x20, 0x47, 0x4d,
                0x54, 0xc0, 0x5a, 0x04, 0x67, 0x7a, 0x69, 0x70,
                0x77, 0x38, 0x66, 0x6f, 0x6f, 0x3d, 0x41, 0x53,
                0x44, 0x4a, 0x4b, 0x48, 0x51, 0x4b, 0x42, 0x5a,
                0x58, 0x4f, 0x51, 0x57, 0x45, 0x4f, 0x50, 0x49,
                0x55, 0x41, 0x58, 0x51, 0x57, 0x45, 0x4f, 0x49,
                0x55, 0x3b, 0x20, 0x6d, 0x61, 0x78, 0x2d, 0x61,
                0x67, 0x65, 0x3d, 0x33, 0x36, 0x30, 0x30, 0x3b,
                0x20, 0x76, 0x65, 0x72, 0x73, 0x69, 0x6f, 0x6e,
                0x3d, 0x31
            ];
        }
        assert_eq!(block.finish(), &expected[..]);
    }
    assert_eq!(encoder.context().dynamic_table().size(), 215);
}

#[test]
/// https://tools.ietf.org/html/rfc7541#appendix-C.6
fn response_examples_with_huffman_coding() {
    let mut encoder = Encoder::new(Context::new(256));

    // C.6.1. First Request
    {
        let mut block = track_try_unwrap!(encoder.enter_header_block(Vec::new()));
        track_try_unwrap!(
            block.encode_literal_header_field(
                LiteralHeaderFieldBuilder::with_indexing()
                    .value_encoding(Encoding::Huffman)
                    .finish(Name::Status, b"302"),
            )
        );
        track_try_unwrap!(
            block.encode_literal_header_field(
                LiteralHeaderFieldBuilder::with_indexing()
                    .value_encoding(Encoding::Huffman)
                    .finish(Name::CacheControl, b"private"),
            )
        );
        track_try_unwrap!(
            block.encode_literal_header_field(
                LiteralHeaderFieldBuilder::with_indexing()
                    .value_encoding(Encoding::Huffman)
                    .finish(Name::Date, b"Mon, 21 Oct 2013 20:13:21 GMT"),
            )
        );
        track_try_unwrap!(
            block.encode_literal_header_field(
                LiteralHeaderFieldBuilder::with_indexing()
                    .value_encoding(Encoding::Huffman)
                    .finish(Name::Location, b"https://www.example.com"),
            )
        );

        let expected;
        #[cfg_attr(rustfmt, rustfmt_skip)]
        {
            expected = [
                0x48, 0x82, 0x64, 0x02, 0x58, 0x85, 0xae, 0xc3, 0x77, 0x1a, 0x4b,
                0x61, 0x96, 0xd0, 0x7a, 0xbe, 0x94, 0x10, 0x54, 0xd4, 0x44, 0xa8,
                0x20, 0x05, 0x95, 0x04, 0x0b, 0x81, 0x66, 0xe0, 0x82, 0xa6, 0x2d,
                0x1b, 0xff, 0x6e, 0x91, 0x9d, 0x29, 0xad, 0x17, 0x18, 0x63, 0xc7,
                0x8f, 0x0b, 0x97, 0xc8, 0xe9, 0xae, 0x82, 0xae, 0x43, 0xd3            
            ];
        }
        assert_eq!(block.finish(), &expected[..]);
    }
    assert_eq!(encoder.context().dynamic_table().size(), 222);

    // C.6.2. Second Request
    {
        let mut block = track_try_unwrap!(encoder.enter_header_block(Vec::new()));
        track_try_unwrap!(
            block.encode_literal_header_field(
                LiteralHeaderFieldBuilder::with_indexing()
                    .value_encoding(Encoding::Huffman)
                    .finish(Name::Status, b"307"),
            )
        );
        track_try_unwrap!(block.encode_indexed_header_field(65));
        track_try_unwrap!(block.encode_indexed_header_field(64));
        track_try_unwrap!(block.encode_indexed_header_field(63));

        let expected;
        #[cfg_attr(rustfmt, rustfmt_skip)]
        {
            expected = [
                0x48, 0x83, 0x64, 0x0e, 0xff, 0xc1, 0xc0, 0xbf
            ];
        }
        assert_eq!(block.finish(), &expected[..]);
    }
    assert_eq!(encoder.context().dynamic_table().size(), 222);

    // C.6.3. Third Request
    {
        let mut block = track_try_unwrap!(encoder.enter_header_block(Vec::new()));
        track_try_unwrap!(block.encode_indexed_header_field(8));
        track_try_unwrap!(block.encode_indexed_header_field(65));
        track_try_unwrap!(
            block.encode_literal_header_field(
                LiteralHeaderFieldBuilder::with_indexing()
                    .value_encoding(Encoding::Huffman)
                    .finish(Name::Date, b"Mon, 21 Oct 2013 20:13:22 GMT"),
            )
        );
        track_try_unwrap!(block.encode_indexed_header_field(64));
        track_try_unwrap!(
            block.encode_literal_header_field(
                LiteralHeaderFieldBuilder::with_indexing()
                    .value_encoding(Encoding::Huffman)
                    .finish(Name::ContentEncoding, b"gzip"),
            )
        );
        track_try_unwrap!(
            block.encode_literal_header_field(
                LiteralHeaderFieldBuilder::with_indexing()
                    .value_encoding(Encoding::Huffman)
                    .finish(
                        Name::SetCookie,
                        &b"foo=ASDJKHQKBZXOQWEOPIUAXQWEOIU; max-age=3600; version=1"[..],
                    ),
            )
        );

        let expected;
        #[cfg_attr(rustfmt, rustfmt_skip)]
        {
            expected = [
                0x88, 0xc1, 0x61, 0x96, 0xd0, 0x7a, 0xbe, 0x94, 0x10, 0x54, 0xd4,
                0x44, 0xa8, 0x20, 0x05, 0x95, 0x04, 0x0b, 0x81, 0x66, 0xe0, 0x84,
                0xa6, 0x2d, 0x1b, 0xff, 0xc0, 0x5a, 0x83, 0x9b, 0xd9, 0xab, 0x77,
                0xad, 0x94, 0xe7, 0x82, 0x1d, 0xd7, 0xf2, 0xe6, 0xc7, 0xb3, 0x35,
                0xdf, 0xdf, 0xcd, 0x5b, 0x39, 0x60, 0xd5, 0xaf, 0x27, 0x08, 0x7f,
                0x36, 0x72, 0xc1, 0xab, 0x27, 0x0f, 0xb5, 0x29, 0x1f, 0x95, 0x87,
                0x31, 0x60, 0x65, 0xc0, 0x03, 0xed, 0x4e, 0xe5, 0xb1, 0x06, 0x3d,
                0x50, 0x07
            ];
        }
        assert_eq!(block.finish(), &expected[..]);
    }
    assert_eq!(encoder.context().dynamic_table().size(), 215);
}
