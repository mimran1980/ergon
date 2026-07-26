use crate::*;

pub use encoder::EngineEncoder;
pub use decoder::EngineDecoder;

pub const ENCODED_LENGTH: usize = 8;

pub mod encoder {
    use super::*;

    #[derive(Debug, Default)]
    pub struct EngineEncoder<P> {
        parent: Option<P>,
        offset: usize,
    }

    impl<'a, P> Writer<'a> for EngineEncoder<P> where P: Writer<'a> + Default {
        #[inline]
        fn get_buf_mut(&mut self) -> &mut WriteBuf<'a> {
            if let Some(parent) = self.parent.as_mut() {
                parent.get_buf_mut()
            } else {
                panic!("parent was None")
            }
        }
    }

    impl<'a, P> EngineEncoder<P> where P: Writer<'a> + Default {
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

        /// primitive field 'capacity'
        /// - min value: 0
        /// - max value: 65534
        /// - null value: 0xffff_u16
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 0
        /// - encodedLength: 2
        /// - version: 0
        #[inline]
        pub fn capacity(&mut self, value: u16) -> &mut Self {
            let offset = self.offset;
            self.get_buf_mut().put_u16_at(offset, value);
            self
        }

        /// primitive field 'numCylinders'
        /// - min value: 0
        /// - max value: 254
        /// - null value: 0xff_u8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 2
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn num_cylinders(&mut self, value: u8) -> &mut Self {
            let offset = self.offset + 2;
            self.get_buf_mut().put_u8_at(offset, value);
            self
        }

        // skipping CONSTANT maxRpm

        #[inline]
        pub fn manufacturer_code_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 3;
            let buf = self.get_buf_mut();
            buf.put_u8_at(offset + index, value);
            self
        }

        /// primitive array field 'manufacturerCode'
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: null
        /// - encodedOffset: 3
        /// - encodedLength: 3
        /// - version: 0
        #[inline]
        pub fn manufacturer_code(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(3, value.len());
            let offset = self.offset + 3;
            let buf = self.get_buf_mut();
            buf.put_slice_at(offset, value);
            self
        }

        /// primitive array field 'manufacturerCode' from an Iterator
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: null
        /// - encodedOffset: 3
        /// - encodedLength: 3
        /// - version: 0
        #[inline]
        pub fn manufacturer_code_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 3;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'manufacturerCode' with zero padding
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: null
        /// - encodedOffset: 3
        /// - encodedLength: 3
        /// - version: 0
        #[inline]
        pub fn manufacturer_code_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(3);
            self.manufacturer_code_from_iter(iter);
            self
        }

        // skipping CONSTANT fuel

        /// COMPOSITE ENCODER
        #[inline]
        pub fn booster_encoder(self) -> booster_tc_odec::BoosterTEncoder<Self> {
            let offset = self.offset + 6;
            booster_tc_odec::BoosterTEncoder::default().wrap(self, offset)
        }

        /// Set all optional fields to their null values.
        #[inline]
        pub fn nullify_optional_fields(&mut self) -> &mut Self {
            {
                let mut composite_encoder = core::mem::take(self).booster_encoder();
                composite_encoder.nullify_optional_fields();
                *self = composite_encoder.parent().expect("parent missing");
            }
            self
        }

    }
} // end encoder mod 

pub mod decoder {
    use super::*;

    #[derive(Debug, Default)]
    pub struct EngineDecoder<P> {
        parent: Option<P>,
        offset: usize,
    }

    impl<'a, P> ActingVersion for EngineDecoder<P> where P: Reader<'a> + ActingVersion + Default {
        #[inline]
        fn acting_version(&self) -> u16 {
            self.parent.as_ref().unwrap().acting_version()
        }
    }

    impl<'a, P> Reader<'a> for EngineDecoder<P> where P: Reader<'a> + Default {
        #[inline]
        fn get_buf(&self) -> &ReadBuf<'a> {
            self.parent.as_ref().expect("parent missing").get_buf()
        }
    }

    impl<'a, P> EngineDecoder<P> where P: Reader<'a> + Default {
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
        pub fn capacity(&self) -> u16 {
            self.get_buf().get_u16_at(self.offset)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn num_cylinders(&self) -> u8 {
            self.get_buf().get_u8_at(self.offset + 2)
        }

        /// CONSTANT 
        #[inline]
        pub fn max_rpm(&self) -> u16 {
            9000
        }

        #[inline]
        pub fn manufacturer_code(&self) -> [u8; 3] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 3)
        }

        /// CONSTANT 
        /// characterEncoding: 'US-ASCII'
        #[inline]
        pub fn fuel(&self) -> &'static [u8] {
            b"Petrol"
        }

        /// COMPOSITE DECODER
        #[inline]
        pub fn booster_decoder(self) -> booster_tc_odec::BoosterTDecoder<Self> {
            let offset = self.offset + 6;
            booster_tc_odec::BoosterTDecoder::default().wrap(self, offset)
        }

    }
} // end decoder mod 
