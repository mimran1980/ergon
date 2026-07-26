//! Header included before every SBE message

use crate::*;

pub use encoder::MessageHeaderEncoder;
pub use decoder::MessageHeaderDecoder;

pub const ENCODED_LENGTH: usize = 7;

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

        /// primitive field 'schemaId'
        /// - description: ID for the schema
        /// - min value: 0
        /// - max value: 65534
        /// - null value: 0xffff_u16
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 0
        /// - encodedLength: 2
        /// - version: 0
        #[inline]
        pub fn schema_id(&mut self, value: u16) -> &mut Self {
            let offset = self.offset;
            self.get_buf_mut().put_u16_at(offset, value);
            self
        }

        /// primitive field 'version'
        /// - description: Version number of the schema
        /// - min value: 0
        /// - max value: 254
        /// - null value: 0xff_u8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 2
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn version(&mut self, value: u8) -> &mut Self {
            let offset = self.offset + 2;
            self.get_buf_mut().put_u8_at(offset, value);
            self
        }

        /// primitive field 'templateId'
        /// - description: ID of the msg type.
        /// - min value: 0
        /// - max value: 254
        /// - null value: 0xff_u8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 3
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn template_id(&mut self, value: u8) -> &mut Self {
            let offset = self.offset + 3;
            self.get_buf_mut().put_u8_at(offset, value);
            self
        }

        /// primitive field 'blockLength'
        /// - description: Length of fixed-size root-block, excludes this header, repeating-groups and var-data
        /// - min value: 0
        /// - max value: 254
        /// - null value: 0xff_u8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 4
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn block_length(&mut self, value: u8) -> &mut Self {
            let offset = self.offset + 4;
            self.get_buf_mut().put_u8_at(offset, value);
            self
        }

        /// primitive field 'numGroups'
        /// - description: Number of repeating-groups
        /// - min value: 0
        /// - max value: 254
        /// - null value: 0xff_u8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 5
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn num_groups(&mut self, value: u8) -> &mut Self {
            let offset = self.offset + 5;
            self.get_buf_mut().put_u8_at(offset, value);
            self
        }

        /// primitive field 'numVarDataFields'
        /// - description: Number of variable-length fields
        /// - min value: 0
        /// - max value: 254
        /// - null value: 0xff_u8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 6
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn num_var_data_fields(&mut self, value: u8) -> &mut Self {
            let offset = self.offset + 6;
            self.get_buf_mut().put_u8_at(offset, value);
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
        fn acting_version(&self) -> u8 {
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
        /// - description: ID for the schema
        #[inline]
        pub fn schema_id(&self) -> u16 {
            self.get_buf().get_u16_at(self.offset)
        }

        /// primitive field - 'REQUIRED'
        /// - description: Version number of the schema
        #[inline]
        pub fn version(&self) -> u8 {
            self.get_buf().get_u8_at(self.offset + 2)
        }

        /// primitive field - 'REQUIRED'
        /// - description: ID of the msg type.
        #[inline]
        pub fn template_id(&self) -> u8 {
            self.get_buf().get_u8_at(self.offset + 3)
        }

        /// primitive field - 'REQUIRED'
        /// - description: Length of fixed-size root-block, excludes this header, repeating-groups and var-data
        #[inline]
        pub fn block_length(&self) -> u8 {
            self.get_buf().get_u8_at(self.offset + 4)
        }

        /// primitive field - 'REQUIRED'
        /// - description: Number of repeating-groups
        #[inline]
        pub fn num_groups(&self) -> u8 {
            self.get_buf().get_u8_at(self.offset + 5)
        }

        /// primitive field - 'REQUIRED'
        /// - description: Number of variable-length fields
        #[inline]
        pub fn num_var_data_fields(&self) -> u8 {
            self.get_buf().get_u8_at(self.offset + 6)
        }

    }
} // end decoder mod
