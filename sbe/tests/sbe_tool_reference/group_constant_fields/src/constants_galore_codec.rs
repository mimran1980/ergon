use crate::*;

pub use decoder::ConstantsGaloreDecoder;
pub use encoder::ConstantsGaloreEncoder;

pub use crate::SBE_SCHEMA_ID;
pub use crate::SBE_SCHEMA_VERSION;
pub use crate::SBE_SEMANTIC_VERSION;

pub const SBE_BLOCK_LENGTH: u16 = 2;
pub const SBE_TEMPLATE_ID: u16 = 1;

pub mod encoder {
    use super::*;
    use message_header_codec::*;

    #[derive(Debug, Default)]
    pub struct ConstantsGaloreEncoder<'a> {
        buf: WriteBuf<'a>,
        initial_offset: usize,
        offset: usize,
        limit: usize,
    }

    impl<'a> Writer<'a> for ConstantsGaloreEncoder<'a> {
        #[inline]
        fn get_buf_mut(&mut self) -> &mut WriteBuf<'a> {
            &mut self.buf
        }
    }

    impl<'a> Encoder<'a> for ConstantsGaloreEncoder<'a> {
        #[inline]
        fn get_limit(&self) -> usize {
            self.limit
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.limit = limit;
        }

        /// Set all optional fields to their 'null' values.
        #[inline]
        fn nullify_optional_fields(&mut self) -> &mut Self {
            {
                let mut composite_encoder = core::mem::take(self).ee_ncoder();
                composite_encoder.nullify_optional_fields();
                *self = composite_encoder.parent().expect("parent missing");
            }
            self
        }
    }

    impl<'a> ConstantsGaloreEncoder<'a> {
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

        /// primitive field 'a'
        /// - min value: 0
        /// - max value: 254
        /// - null value: 0xff_u8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 0
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn a(&mut self, value: u8) -> &mut Self {
            let offset = self.offset;
            self.get_buf_mut().put_u8_at(offset, value);
            self
        }

        // skipping CONSTANT b

        // skipping CONSTANT enum 'c'

        // skipping CONSTANT d

        /// COMPOSITE ENCODER
        #[inline]
        pub fn ee_ncoder(self) -> composite_with_const_codec::CompositeWithConstEncoder<Self> {
            let offset = self.offset + 1;
            composite_with_const_codec::CompositeWithConstEncoder::default().wrap(self, offset)
        }

        /// GROUP ENCODER (id=7)
        #[inline]
        pub fn fe_ncoder(self, count: u16, fe_ncoder: FEncoder<Self>) -> FEncoder<Self> {
            fe_ncoder.wrap(self, count)
        }

    }

    #[derive(Debug, Default)]
    pub struct FEncoder<P> {
        parent: Option<P>,
        count: u16,
        index: usize,
        offset: usize,
        initial_limit: usize,
    }

    impl<'a, P> Writer<'a> for FEncoder<P> where P: Writer<'a> + Default {
        #[inline]
        fn get_buf_mut(&mut self) -> &mut WriteBuf<'a> {
            if let Some(parent) = self.parent.as_mut() {
                parent.get_buf_mut()
            } else {
                panic!("parent was None")
            }
        }
    }

    impl<'a, P> Encoder<'a> for FEncoder<P> where P: Encoder<'a> + Default {
        #[inline]
        fn get_limit(&self) -> usize {
            self.parent.as_ref().expect("parent missing").get_limit()
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.parent.as_mut().expect("parent missing").set_limit(limit);
        }

        /// Set all optional fields to their 'null' values.
        #[inline]
        fn nullify_optional_fields(&mut self) -> &mut Self {
            {
                let mut composite_encoder = core::mem::take(self).he_ncoder();
                composite_encoder.nullify_optional_fields();
                *self = composite_encoder.parent().expect("parent missing");
            }
            self
        }
    }

    impl<'a, P> FEncoder<P> where P: Encoder<'a> + Default {
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
            2
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

        /// primitive field 'g'
        /// - min value: 0
        /// - max value: 254
        /// - null value: 0xff_u8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 0
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn g(&mut self, value: u8) -> &mut Self {
            let offset = self.offset;
            self.get_buf_mut().put_u8_at(offset, value);
            self
        }

        /// COMPOSITE ENCODER
        #[inline]
        pub fn he_ncoder(self) -> composite_with_const_codec::CompositeWithConstEncoder<Self> {
            let offset = self.offset + 1;
            composite_with_const_codec::CompositeWithConstEncoder::default().wrap(self, offset)
        }

        // skipping CONSTANT i

        // skipping CONSTANT j

        // skipping CONSTANT enum 'k'

        // skipping CONSTANT l

    }

} // end encoder

pub mod decoder {
    use super::*;
    use message_header_codec::*;

    #[derive(Clone, Copy, Debug, Default)]
    pub struct ConstantsGaloreDecoder<'a> {
        buf: ReadBuf<'a>,
        initial_offset: usize,
        offset: usize,
        limit: usize,
        pub acting_block_length: u16,
        pub acting_version: u16,
    }

    impl ActingVersion for ConstantsGaloreDecoder<'_> {
        #[inline]
        fn acting_version(&self) -> u16 {
            self.acting_version
        }
    }

    impl<'a> Reader<'a> for ConstantsGaloreDecoder<'a> {
        #[inline]
        fn get_buf(&self) -> &ReadBuf<'a> {
            &self.buf
        }
    }

    impl<'a> Decoder<'a> for ConstantsGaloreDecoder<'a> {
        #[inline]
        fn get_limit(&self) -> usize {
            self.limit
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.limit = limit;
        }
    }

    impl<'a> ConstantsGaloreDecoder<'a> {
        pub fn wrap(
            mut self,
            buf: ReadBuf<'a>,
            offset: usize,
            acting_block_length: u16,
            acting_version: u16,
        ) -> Self {
            let limit = offset + acting_block_length as usize;
            self.buf = buf;
            self.initial_offset = offset;
            self.offset = offset;
            self.limit = limit;
            self.acting_block_length = acting_block_length;
            self.acting_version = acting_version;
            self
        }

        #[inline]
        pub const fn encoded_length(&self) -> usize {
            self.limit - self.offset
        }

        #[inline]
        pub fn header(self, mut header: MessageHeaderDecoder<ReadBuf<'a>>, offset: usize) -> Self {
            debug_assert_eq!(SBE_TEMPLATE_ID, header.template_id());
            let acting_block_length = header.block_length();
            let acting_version = header.version();

            self.wrap(
                header.parent().unwrap(),
                offset + message_header_codec::ENCODED_LENGTH,
                acting_block_length,
                acting_version,
            )
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn a(&self) -> u8 {
            self.get_buf().get_u8_at(self.offset)
        }

        /// CONSTANT
        #[inline]
        pub fn b(&self) -> u16 {
            9000
        }

        /// CONSTANT enum
        #[inline]
        pub fn c(&self) -> model::Model {
            model::Model::C
        }

        /// CONSTANT
        #[inline]
        pub fn d(&self) -> u16 {
            9000
        }

        /// COMPOSITE DECODER
        #[inline]
        pub fn ed_ecoder(self) -> composite_with_const_codec::CompositeWithConstDecoder<Self> {
            let offset = self.offset + 1;
            composite_with_const_codec::CompositeWithConstDecoder::default().wrap(self, offset)
        }

        /// GROUP DECODER (id=7)
        #[inline]
        pub fn fd_ecoder(self) -> FDecoder<Self> {
            FDecoder::default().wrap(self)
        }

    }

    #[derive(Debug, Default)]
    pub struct FDecoder<P> {
        parent: Option<P>,
        block_length: u16,
        count: u16,
        index: usize,
        offset: usize,
    }

    impl<'a, P> ActingVersion for FDecoder<P> where P: Reader<'a> + ActingVersion + Default {
        #[inline]
        fn acting_version(&self) -> u16 {
            self.parent.as_ref().unwrap().acting_version()
        }
    }

    impl<'a, P> Reader<'a> for FDecoder<P> where P: Reader<'a> + Default {
        #[inline]
        fn get_buf(&self) -> &ReadBuf<'a> {
            self.parent.as_ref().expect("parent missing").get_buf()
        }
    }

    impl<'a, P> Decoder<'a> for FDecoder<P> where P: Decoder<'a> + ActingVersion + Default {
        #[inline]
        fn get_limit(&self) -> usize {
            self.parent.as_ref().expect("parent missing").get_limit()
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.parent.as_mut().expect("parent missing").set_limit(limit);
        }
    }

    impl<'a, P> FDecoder<P> where P: Decoder<'a> + ActingVersion + Default {
        pub fn wrap(
            mut self,
            mut parent: P,
        ) -> Self {
            let initial_offset = parent.get_limit();
            let block_length = parent.get_buf().get_u16_at(initial_offset);
            let count = parent.get_buf().get_u16_at(initial_offset + 2);
            parent.set_limit(initial_offset + 4);
            self.parent = Some(parent);
            self.block_length = block_length;
            self.count = count;
            self.index = usize::MAX;
            self.offset = 0;
            self
        }

        /// group token - Token{signal=BEGIN_GROUP, name='f', referencedName='null', description='null', packageName='null', id=7, version=0, deprecated=0, encodedLength=2, offset=2, componentTokenCount=32, encoding=Encoding{presence=REQUIRED, primitiveType=null, byteOrder=LITTLE_ENDIAN, minValue=null, maxValue=null, nullValue=null, constValue=null, characterEncoding='null', epoch='null', timeUnit=null, semanticType='null'}}
        #[inline]
        pub fn parent(&mut self) -> SbeResult<P> {
            self.parent.take().ok_or(SbeErr::ParentNotSet)
        }

        #[inline]
        pub fn acting_version(&mut self) -> u16 {
            self.parent.as_ref().unwrap().acting_version()
        }

        #[inline]
        pub fn count(&self) -> u16 {
            self.count
        }

        /// will return Some(current index) when successful otherwise None
        pub fn advance(&mut self) -> SbeResult<Option<usize>> {
            let index = self.index.wrapping_add(1);
            if index >= self.count as usize {
                 return Ok(None);
            }
            if let Some(parent) = self.parent.as_mut() {
                self.offset = parent.get_limit();
                parent.set_limit(self.offset + self.block_length as usize);
                self.index = index;
                Ok(Some(index))
            } else {
                Err(SbeErr::ParentNotSet)
            }
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn g(&self) -> u8 {
            self.get_buf().get_u8_at(self.offset)
        }

        /// COMPOSITE DECODER
        #[inline]
        pub fn hd_ecoder(self) -> composite_with_const_codec::CompositeWithConstDecoder<Self> {
            let offset = self.offset + 1;
            composite_with_const_codec::CompositeWithConstDecoder::default().wrap(self, offset)
        }

        /// CONSTANT
        #[inline]
        pub fn i(&self) -> u16 {
            9000
        }

        /// CONSTANT
        #[inline]
        pub fn j(&self) -> u16 {
            9000
        }

        /// CONSTANT enum
        #[inline]
        pub fn k(&self) -> model::Model {
            model::Model::C
        }

        /// CONSTANT
        /// characterEncoding: 'US-ASCII'
        #[inline]
        pub fn l(&self) -> &'static [u8] {
            b"Huzzah"
        }

    }

} // end decoder
