// sbe-tool (official simple-binary-encoding) Rust SBE generated code for the
// orderbook benchmark schema. Base types + shared codecs extracted from the
// Car patched file; BookSnapshot codec from sbe-tool generation.

pub mod sbe_tool {

    use ::core::convert::TryInto;

    pub const SBE_SCHEMA_ID: u16 = 1;
    pub const SBE_SCHEMA_VERSION: u16 = 0;
    pub const SBE_SEMANTIC_VERSION: &str = "5.2";

    pub type SbeResult<T> = core::result::Result<T, SbeErr>;

    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub enum SbeErr {
        ParentNotSet,
    }
    impl core::fmt::Display for SbeErr {
        #[inline]
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(f, "{self:?}")
        }
    }
    impl std::error::Error for SbeErr {}

    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub enum Either<L, R> {
        Left(L),
        Right(R),
    }

    pub trait Writer<'a>: Sized {
        fn get_buf_mut(&mut self) -> &mut WriteBuf<'a>;
    }

    pub trait Encoder<'a>: Writer<'a> {
        fn get_limit(&self) -> usize;
        fn set_limit(&mut self, limit: usize);
        fn nullify_optional_fields(&mut self) -> &mut Self {
            self
        }
    }

    pub trait ActingVersion {
        fn acting_version(&self) -> u16;
    }

    pub trait Reader<'a>: Sized {
        fn get_buf(&self) -> &ReadBuf<'a>;
    }

    pub trait Decoder<'a>: Reader<'a> {
        fn get_limit(&self) -> usize;
        fn set_limit(&mut self, limit: usize);
    }

    #[derive(Clone, Copy, Debug, Default)]
    pub struct ReadBuf<'a> {
        data: &'a [u8],
    }
    impl<'a> Reader<'a> for ReadBuf<'a> {
        #[inline]
        fn get_buf(&self) -> &ReadBuf<'a> {
            self
        }
    }
    #[allow(dead_code)]
    impl<'a> ReadBuf<'a> {
        #[inline]
        pub const fn new(data: &'a [u8]) -> Self {
            Self { data }
        }

        #[inline]
        pub fn get_bytes_at<const N: usize>(slice: &[u8], index: usize) -> [u8; N] {
            slice[index..index + N]
                .try_into()
                .expect("slice with incorrect length")
        }

        #[inline]
        pub const fn get_u8_at(&self, index: usize) -> u8 {
            self.data[index]
        }

        #[inline]
        pub fn get_i8_at(&self, index: usize) -> i8 {
            i8::from_le_bytes(Self::get_bytes_at(self.data, index))
        }

        #[inline]
        pub fn get_i16_at(&self, index: usize) -> i16 {
            i16::from_le_bytes(Self::get_bytes_at(self.data, index))
        }

        #[inline]
        pub fn get_i32_at(&self, index: usize) -> i32 {
            i32::from_le_bytes(Self::get_bytes_at(self.data, index))
        }

        #[inline]
        pub fn get_i64_at(&self, index: usize) -> i64 {
            i64::from_le_bytes(Self::get_bytes_at(self.data, index))
        }

        #[inline]
        pub fn get_u16_at(&self, index: usize) -> u16 {
            u16::from_le_bytes(Self::get_bytes_at(self.data, index))
        }

        #[inline]
        pub fn get_u32_at(&self, index: usize) -> u32 {
            u32::from_le_bytes(Self::get_bytes_at(self.data, index))
        }

        #[inline]
        pub fn get_u64_at(&self, index: usize) -> u64 {
            u64::from_le_bytes(Self::get_bytes_at(self.data, index))
        }

        #[inline]
        pub fn get_f32_at(&self, index: usize) -> f32 {
            f32::from_le_bytes(Self::get_bytes_at(self.data, index))
        }

        #[inline]
        pub fn get_f64_at(&self, index: usize) -> f64 {
            f64::from_le_bytes(Self::get_bytes_at(self.data, index))
        }

        #[inline]
        pub fn get_slice_at(&self, index: usize, len: usize) -> &[u8] {
            &self.data[index..index + len]
        }
    }

    #[derive(Debug, Default)]
    pub struct WriteBuf<'a> {
        data: &'a mut [u8],
    }
    impl<'a> WriteBuf<'a> {
        pub const fn new(data: &'a mut [u8]) -> Self {
            Self { data }
        }

        #[inline]
        pub fn put_bytes_at<const COUNT: usize>(
            &mut self,
            index: usize,
            bytes: &[u8; COUNT],
        ) -> usize {
            self.data[index..index + COUNT].copy_from_slice(bytes);
            COUNT
        }

        #[inline]
        pub const fn put_u8_at(&mut self, index: usize, value: u8) {
            self.data[index] = value;
        }

        #[inline]
        pub fn put_i8_at(&mut self, index: usize, value: i8) {
            self.put_bytes_at(index, &i8::to_le_bytes(value));
        }

        #[inline]
        pub fn put_i16_at(&mut self, index: usize, value: i16) {
            self.put_bytes_at(index, &i16::to_le_bytes(value));
        }

        #[inline]
        pub fn put_i32_at(&mut self, index: usize, value: i32) {
            self.put_bytes_at(index, &i32::to_le_bytes(value));
        }

        #[inline]
        pub fn put_i64_at(&mut self, index: usize, value: i64) {
            self.put_bytes_at(index, &i64::to_le_bytes(value));
        }

        #[inline]
        pub fn put_u16_at(&mut self, index: usize, value: u16) {
            self.put_bytes_at(index, &u16::to_le_bytes(value));
        }

        #[inline]
        pub fn put_u32_at(&mut self, index: usize, value: u32) {
            self.put_bytes_at(index, &u32::to_le_bytes(value));
        }

        #[inline]
        pub fn put_u64_at(&mut self, index: usize, value: u64) {
            self.put_bytes_at(index, &u64::to_le_bytes(value));
        }

        #[inline]
        pub fn put_f32_at(&mut self, index: usize, value: f32) {
            self.put_bytes_at(index, &f32::to_le_bytes(value));
        }

        #[inline]
        pub fn put_f64_at(&mut self, index: usize, value: f64) {
            self.put_bytes_at(index, &f64::to_le_bytes(value));
        }

        #[inline]
        pub fn put_slice_at(&mut self, index: usize, src: &[u8]) -> usize {
            let len = src.len();
            let dest = self.data.split_at_mut(index).1.split_at_mut(len).0;
            dest.clone_from_slice(src);
            len
        }
    }
    impl<'a> From<&'a mut WriteBuf<'a>> for &'a mut [u8] {
        #[inline]
        fn from(buf: &'a mut WriteBuf<'a>) -> &'a mut [u8] {
            buf.data
        }
    }

    pub mod group_size_encoding_codec {
        use super::*;

        pub use decoder::GroupSizeEncodingDecoder;
        pub use encoder::GroupSizeEncodingEncoder;

        pub const ENCODED_LENGTH: usize = 4;

        pub mod encoder {
            use super::*;

            #[derive(Debug, Default)]
            pub struct GroupSizeEncodingEncoder<P> {
                parent: Option<P>,
                offset: usize,
            }

            impl<'a, P> Writer<'a> for GroupSizeEncodingEncoder<P>
            where
                P: Writer<'a> + Default,
            {
                #[inline]
                fn get_buf_mut(&mut self) -> &mut WriteBuf<'a> {
                    if let Some(parent) = self.parent.as_mut() {
                        parent.get_buf_mut()
                    } else {
                        panic!("parent was None")
                    }
                }
            }

            impl<'a, P> GroupSizeEncodingEncoder<P>
            where
                P: Writer<'a> + Default,
            {
                pub fn wrap(mut self, parent: P, offset: usize) -> Self {
                    self.parent = Some(parent);
                    self.offset = offset;
                    self
                }

                #[inline]
                pub fn parent(&mut self) -> SbeResult<P> {
                    self.parent.take().ok_or(SbeErr::ParentNotSet)
                }

                #[inline]
                pub fn block_length(&mut self, value: u16) -> &mut Self {
                    let offset = self.offset;
                    self.get_buf_mut().put_u16_at(offset, value);
                    self
                }

                #[inline]
                pub fn num_in_group(&mut self, value: u16) -> &mut Self {
                    let offset = self.offset + 2;
                    self.get_buf_mut().put_u16_at(offset, value);
                    self
                }

                #[inline]
                pub fn nullify_optional_fields(&mut self) -> &mut Self {
                    self
                }
            }
        }

        pub mod decoder {
            use super::*;

            #[derive(Debug, Default)]
            pub struct GroupSizeEncodingDecoder<P> {
                parent: Option<P>,
                offset: usize,
            }

            impl<'a, P> ActingVersion for GroupSizeEncodingDecoder<P>
            where
                P: Reader<'a> + ActingVersion + Default,
            {
                #[inline]
                fn acting_version(&self) -> u16 {
                    self.parent.as_ref().unwrap().acting_version()
                }
            }

            impl<'a, P> Reader<'a> for GroupSizeEncodingDecoder<P>
            where
                P: Reader<'a> + Default,
            {
                #[inline]
                fn get_buf(&self) -> &ReadBuf<'a> {
                    self.parent.as_ref().expect("parent missing").get_buf()
                }
            }

            impl<'a, P> GroupSizeEncodingDecoder<P>
            where
                P: Reader<'a> + Default,
            {
                pub fn wrap(mut self, parent: P, offset: usize) -> Self {
                    self.parent = Some(parent);
                    self.offset = offset;
                    self
                }

                #[inline]
                pub fn parent(&mut self) -> SbeResult<P> {
                    self.parent.take().ok_or(SbeErr::ParentNotSet)
                }

                #[inline]
                pub fn block_length(&self) -> u16 {
                    self.get_buf().get_u16_at(self.offset)
                }

                #[inline]
                pub fn num_in_group(&self) -> u16 {
                    self.get_buf().get_u16_at(self.offset + 2)
                }
            }
        }
    }

    pub mod message_header_codec {
        use super::*;

        pub use decoder::MessageHeaderDecoder;
        pub use encoder::MessageHeaderEncoder;

        pub const ENCODED_LENGTH: usize = 8;

        pub mod encoder {
            use super::*;

            #[derive(Debug, Default)]
            pub struct MessageHeaderEncoder<P> {
                parent: Option<P>,
                offset: usize,
            }

            impl<'a, P> Writer<'a> for MessageHeaderEncoder<P>
            where
                P: Writer<'a> + Default,
            {
                #[inline]
                fn get_buf_mut(&mut self) -> &mut WriteBuf<'a> {
                    if let Some(parent) = self.parent.as_mut() {
                        parent.get_buf_mut()
                    } else {
                        panic!("parent was None")
                    }
                }
            }

            impl<'a, P> MessageHeaderEncoder<P>
            where
                P: Writer<'a> + Default,
            {
                pub fn wrap(mut self, parent: P, offset: usize) -> Self {
                    self.parent = Some(parent);
                    self.offset = offset;
                    self
                }

                #[inline]
                pub fn parent(&mut self) -> SbeResult<P> {
                    self.parent.take().ok_or(SbeErr::ParentNotSet)
                }

                #[inline]
                pub fn block_length(&mut self, value: u16) -> &mut Self {
                    let offset = self.offset;
                    self.get_buf_mut().put_u16_at(offset, value);
                    self
                }

                #[inline]
                pub fn template_id(&mut self, value: u16) -> &mut Self {
                    let offset = self.offset + 2;
                    self.get_buf_mut().put_u16_at(offset, value);
                    self
                }

                #[inline]
                pub fn schema_id(&mut self, value: u16) -> &mut Self {
                    let offset = self.offset + 4;
                    self.get_buf_mut().put_u16_at(offset, value);
                    self
                }

                #[inline]
                pub fn version(&mut self, value: u16) -> &mut Self {
                    let offset = self.offset + 6;
                    self.get_buf_mut().put_u16_at(offset, value);
                    self
                }

                #[inline]
                pub fn nullify_optional_fields(&mut self) -> &mut Self {
                    self
                }
            }
        }

        pub mod decoder {
            use super::*;

            #[derive(Debug, Default)]
            pub struct MessageHeaderDecoder<P> {
                parent: Option<P>,
                offset: usize,
            }

            impl<'a, P> ActingVersion for MessageHeaderDecoder<P>
            where
                P: Reader<'a> + ActingVersion + Default,
            {
                #[inline]
                fn acting_version(&self) -> u16 {
                    self.parent.as_ref().unwrap().acting_version()
                }
            }

            impl<'a, P> Reader<'a> for MessageHeaderDecoder<P>
            where
                P: Reader<'a> + Default,
            {
                #[inline]
                fn get_buf(&self) -> &ReadBuf<'a> {
                    self.parent.as_ref().expect("parent missing").get_buf()
                }
            }

            impl<'a, P> MessageHeaderDecoder<P>
            where
                P: Reader<'a> + Default,
            {
                pub fn wrap(mut self, parent: P, offset: usize) -> Self {
                    self.parent = Some(parent);
                    self.offset = offset;
                    self
                }

                #[inline]
                pub fn parent(&mut self) -> SbeResult<P> {
                    self.parent.take().ok_or(SbeErr::ParentNotSet)
                }

                #[inline]
                pub fn block_length(&self) -> u16 {
                    self.get_buf().get_u16_at(self.offset)
                }

                #[inline]
                pub fn template_id(&self) -> u16 {
                    self.get_buf().get_u16_at(self.offset + 2)
                }

                #[inline]
                pub fn schema_id(&self) -> u16 {
                    self.get_buf().get_u16_at(self.offset + 4)
                }

                #[inline]
                pub fn version(&self) -> u16 {
                    self.get_buf().get_u16_at(self.offset + 6)
                }
            }
        }
    }

    pub mod book_snapshot_codec {
        use super::*;

        pub use encoder::BookSnapshotEncoder;

        pub const SBE_BLOCK_LENGTH: u16 = 0;
        pub const SBE_TEMPLATE_ID: u16 = 1;

        pub mod encoder {
            use super::*;
            use message_header_codec::*;

            #[derive(Debug, Default)]
            pub struct BookSnapshotEncoder<'a> {
                buf: WriteBuf<'a>,
                initial_offset: usize,
                offset: usize,
                limit: usize,
            }

            impl<'a> Writer<'a> for BookSnapshotEncoder<'a> {
                #[inline]
                fn get_buf_mut(&mut self) -> &mut WriteBuf<'a> {
                    &mut self.buf
                }
            }

            impl<'a> Encoder<'a> for BookSnapshotEncoder<'a> {
                #[inline]
                fn get_limit(&self) -> usize {
                    self.limit
                }

                #[inline]
                fn set_limit(&mut self, limit: usize) {
                    self.limit = limit;
                }
            }

            impl<'a> BookSnapshotEncoder<'a> {
                pub fn wrap(mut self, buf: WriteBuf<'a>, offset: usize) -> Self {
                    let limit = offset + SBE_BLOCK_LENGTH as usize;
                    self.buf = buf;
                    self.initial_offset = offset;
                    self.offset = offset;
                    self.limit = limit;
                    self
                }

                #[inline]
                pub const fn encoded_length(&self) -> usize {
                    self.limit - self.offset
                }

                #[inline]
                pub fn header(self, offset: usize) -> MessageHeaderEncoder<Self> {
                    let mut header = MessageHeaderEncoder::default().wrap(self, offset);
                    header.block_length(SBE_BLOCK_LENGTH);
                    header.template_id(SBE_TEMPLATE_ID);
                    header.schema_id(SBE_SCHEMA_ID);
                    header.version(SBE_SCHEMA_VERSION);
                    header
                }

                /// GROUP ENCODER (id=1)
                #[inline]
                pub fn levels_encoder(self, count: u16, levels_encoder: LevelsEncoder<Self>) -> LevelsEncoder<Self> {
                    levels_encoder.wrap(self, count)
                }
            }

            #[derive(Debug, Default)]
            pub struct LevelsEncoder<P> {
                parent: Option<P>,
                count: u16,
                index: usize,
                offset: usize,
                initial_limit: usize,
            }

            impl<'a, P> Writer<'a> for LevelsEncoder<P> where P: Writer<'a> + Default {
                #[inline]
                fn get_buf_mut(&mut self) -> &mut WriteBuf<'a> {
                    if let Some(parent) = self.parent.as_mut() {
                        parent.get_buf_mut()
                    } else {
                        panic!("parent was None")
                    }
                }
            }

            impl<'a, P> Encoder<'a> for LevelsEncoder<P> where P: Encoder<'a> + Default {
                #[inline]
                fn get_limit(&self) -> usize {
                    self.parent.as_ref().expect("parent missing").get_limit()
                }

                #[inline]
                fn set_limit(&mut self, limit: usize) {
                    self.parent.as_mut().expect("parent missing").set_limit(limit);
                }
            }

            impl<'a, P> LevelsEncoder<P> where P: Encoder<'a> + Default {
                #[inline]
                pub fn wrap(
                    mut self,
                    mut parent: P,
                    count: u16,
                ) -> Self {
                    let initial_limit = parent.get_limit();
                    parent.set_limit(initial_limit + 4);
                    parent.get_buf_mut().put_u16_at(initial_limit, Self::block_length());
                    parent.get_buf_mut().put_u16_at(initial_limit + 2, count);
                    self.parent = Some(parent);
                    self.count = count;
                    self.index = usize::MAX;
                    self.offset = usize::MAX;
                    self.initial_limit = initial_limit;
                    self
                }

                #[inline]
                pub const fn block_length() -> u16 {
                    20
                }

                #[inline]
                pub fn parent(&mut self) -> SbeResult<P> {
                    self.parent.take().ok_or(SbeErr::ParentNotSet)
                }

                /// will return Some(current index) when successful otherwise None
                #[inline]
                pub fn advance(&mut self) -> SbeResult<Option<usize>> {
                    let index = self.index.wrapping_add(1);
                    if index >= self.count as usize {
                        return Ok(None);
                    }
                    if let Some(parent) = self.parent.as_mut() {
                        self.offset = parent.get_limit();
                        parent.set_limit(self.offset + Self::block_length() as usize);
                        self.index = index;
                        Ok(Some(index))
                    } else {
                        Err(SbeErr::ParentNotSet)
                    }
                }

                #[inline]
                pub fn price(&mut self, value: i64) -> &mut Self {
                    let offset = self.offset;
                    self.get_buf_mut().put_i64_at(offset, value);
                    self
                }

                #[inline]
                pub fn qty(&mut self, value: i64) -> &mut Self {
                    let offset = self.offset + 8;
                    self.get_buf_mut().put_i64_at(offset, value);
                    self
                }

                #[inline]
                pub fn num_orders(&mut self, value: u32) -> &mut Self {
                    let offset = self.offset + 16;
                    self.get_buf_mut().put_u32_at(offset, value);
                    self
                }
            }
        }
    }
}
