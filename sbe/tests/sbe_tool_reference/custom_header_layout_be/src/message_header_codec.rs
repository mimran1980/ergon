use crate::*;

pub use encoder::MessageHeaderEncoder;
pub use decoder::MessageHeaderDecoder;

pub const ENCODED_LENGTH: usize = 17;

pub mod encoder {
    use super::*;

    #[derive(Debug, Default)]
    pub struct MessageHeaderEncoder<P> {
        parent: Option<P>,
        offset: usize,
    }

    impl<'a, P> Writer<'a> for MessageHeaderEncoder<P> where P: Writer<'a> + Default {
        #[inline]
        fn get_buf_mut(&mut self) -> &mut WriteBuf<'a> {
            if let Some(parent) = self.parent.as_mut() {
                parent.get_buf_mut()
            } else {
                panic!("parent was None")
            }
        }
    }

    impl<'a, P> MessageHeaderEncoder<P> where P: Writer<'a> + Default {
        pub fn wrap(mut self, parent: P, offset: usize) -> Self {
            self.parent = Some(parent);
            self.offset = offset;
            self
        }

        /// parent fns
        #[inline]
        pub fn parent(&mut self) -> SbeResult<P> {
            self.parent.take().ok_or(SbeErr::ParentNotSet)
        }

        /// primitive field 'blockLength'
        /// - min value: 0
        /// - max value: 4294967294
        /// - null value: 0xffffffff_u32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 1
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn block_length(&mut self, value: u32) -> &mut Self {
            let offset = self.offset + 1;
            self.get_buf_mut().put_u32_at(offset, value);
            self
        }

        /// primitive field 'templateId'
        /// - min value: 0
        /// - max value: 65534
        /// - null value: 0xffff_u16
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 6
        /// - encodedLength: 2
        /// - version: 0
        #[inline]
        pub fn template_id(&mut self, value: u16) -> &mut Self {
            let offset = self.offset + 6;
            self.get_buf_mut().put_u16_at(offset, value);
            self
        }

        /// primitive field 'schemaId'
        /// - min value: 0
        /// - max value: 65534
        /// - null value: 0xffff_u16
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 8
        /// - encodedLength: 2
        /// - version: 0
        #[inline]
        pub fn schema_id(&mut self, value: u16) -> &mut Self {
            let offset = self.offset + 8;
            self.get_buf_mut().put_u16_at(offset, value);
            self
        }

        /// primitive field 'version'
        /// - min value: 0
        /// - max value: 65534
        /// - null value: 0xffff_u16
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 10
        /// - encodedLength: 2
        /// - version: 0
        #[inline]
        pub fn version(&mut self, value: u16) -> &mut Self {
            let offset = self.offset + 10;
            self.get_buf_mut().put_u16_at(offset, value);
            self
        }

        /// primitive field 'numGroups'
        /// - min value: 0
        /// - max value: 65534
        /// - null value: 0xffff_u16
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 13
        /// - encodedLength: 2
        /// - version: 0
        #[inline]
        pub fn num_groups(&mut self, value: u16) -> &mut Self {
            let offset = self.offset + 13;
            self.get_buf_mut().put_u16_at(offset, value);
            self
        }

        /// primitive field 'numVarDataFields'
        /// - min value: 0
        /// - max value: 65534
        /// - null value: 0xffff_u16
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 15
        /// - encodedLength: 2
        /// - version: 0
        #[inline]
        pub fn num_var_data_fields(&mut self, value: u16) -> &mut Self {
            let offset = self.offset + 15;
            self.get_buf_mut().put_u16_at(offset, value);
            self
        }

        /// Set all optional fields to their null values.
        #[inline]
        pub fn nullify_optional_fields(&mut self) -> &mut Self {
            self
        }

    }
} // end encoder mod

pub mod decoder {
    use super::*;

    #[derive(Debug, Default)]
    pub struct MessageHeaderDecoder<P> {
        parent: Option<P>,
        offset: usize,
    }

    impl<'a, P> ActingVersion for MessageHeaderDecoder<P> where P: Reader<'a> + ActingVersion + Default {
        #[inline]
        fn acting_version(&self) -> u16 {
            self.parent.as_ref().unwrap().acting_version()
        }
    }

    impl<'a, P> Reader<'a> for MessageHeaderDecoder<P> where P: Reader<'a> + Default {
        #[inline]
        fn get_buf(&self) -> &ReadBuf<'a> {
            self.parent.as_ref().expect("parent missing").get_buf()
        }
    }

    impl<'a, P> MessageHeaderDecoder<P> where P: Reader<'a> + Default {
        pub fn wrap(mut self, parent: P, offset: usize) -> Self {
            self.parent = Some(parent);
            self.offset = offset;
            self
        }

        #[inline]
        pub fn parent(&mut self) -> SbeResult<P> {
            self.parent.take().ok_or(SbeErr::ParentNotSet)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn block_length(&self) -> u32 {
            self.get_buf().get_u32_at(self.offset + 1)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn template_id(&self) -> u16 {
            self.get_buf().get_u16_at(self.offset + 6)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn schema_id(&self) -> u16 {
            self.get_buf().get_u16_at(self.offset + 8)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn version(&self) -> u16 {
            self.get_buf().get_u16_at(self.offset + 10)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn num_groups(&self) -> u16 {
            self.get_buf().get_u16_at(self.offset + 13)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn num_var_data_fields(&self) -> u16 {
            self.get_buf().get_u16_at(self.offset + 15)
        }

    }
} // end decoder mod
