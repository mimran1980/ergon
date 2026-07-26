use crate::*;

pub use decoder::ExtendedCarDecoder;
pub use encoder::ExtendedCarEncoder;

pub use crate::SBE_SCHEMA_ID;
pub use crate::SBE_SCHEMA_VERSION;
pub use crate::SBE_SEMANTIC_VERSION;

pub const SBE_BLOCK_LENGTH: u16 = 37;
pub const SBE_TEMPLATE_ID: u16 = 2;

pub mod encoder {
    use super::*;
    use message_header_codec::*;

    #[derive(Debug, Default)]
    pub struct ExtendedCarEncoder<'a> {
        buf: WriteBuf<'a>,
        initial_offset: usize,
        offset: usize,
        limit: usize,
    }

    impl<'a> Writer<'a> for ExtendedCarEncoder<'a> {
        #[inline]
        fn get_buf_mut(&mut self) -> &mut WriteBuf<'a> {
            &mut self.buf
        }
    }

    impl<'a> Encoder<'a> for ExtendedCarEncoder<'a> {
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
            self.added_1_opt(None);
            self.added_4_opt(None);
            {
                let mut composite_encoder = core::mem::take(self).engine_encoder();
                composite_encoder.nullify_optional_fields();
                *self = composite_encoder.parent().expect("parent missing");
            }
            {
                let mut composite_encoder = core::mem::take(self).added_6_encoder();
                composite_encoder.nullify_optional_fields();
                *self = composite_encoder.parent().expect("parent missing");
            }
            self
        }
    }

    impl<'a> ExtendedCarEncoder<'a> {
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

        /// primitive field 'serialNumber'
        /// - min value: 0
        /// - max value: -2
        /// - null value: 0xffffffffffffffff_u64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 0
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn serial_number(&mut self, value: u64) -> &mut Self {
            let offset = self.offset;
            self.get_buf_mut().put_u64_at(offset, value);
            self
        }

        /// primitive field 'modelYear'
        /// - min value: 0
        /// - max value: 65534
        /// - null value: 0xffff_u16
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 8
        /// - encodedLength: 2
        /// - version: 0
        #[inline]
        pub fn model_year(&mut self, value: u16) -> &mut Self {
            let offset = self.offset + 8;
            self.get_buf_mut().put_u16_at(offset, value);
            self
        }

        /// REQUIRED enum
        #[inline]
        pub fn available(&mut self, value: boolean_type::BooleanType) -> &mut Self {
            let offset = self.offset + 10;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        /// REQUIRED enum
        #[inline]
        pub fn code(&mut self, value: model::Model) -> &mut Self {
            let offset = self.offset + 11;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        #[inline]
        pub fn vehicle_code_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 12;
            let buf = self.get_buf_mut();
            buf.put_u8_at(offset + index, value);
            self
        }

        /// primitive array field 'vehicleCode'
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: ASCII
        /// - semanticType: null
        /// - encodedOffset: 12
        /// - encodedLength: 6
        /// - version: 0
        #[inline]
        pub fn vehicle_code(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(6, value.len());
            let offset = self.offset + 12;
            let buf = self.get_buf_mut();
            buf.put_slice_at(offset, value);
            self
        }

        /// primitive array field 'vehicleCode' from an Iterator
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: ASCII
        /// - semanticType: null
        /// - encodedOffset: 12
        /// - encodedLength: 6
        /// - version: 0
        #[inline]
        pub fn vehicle_code_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 12;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'vehicleCode' with zero padding
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: ASCII
        /// - semanticType: null
        /// - encodedOffset: 12
        /// - encodedLength: 6
        /// - version: 0
        #[inline]
        pub fn vehicle_code_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(6);
            self.vehicle_code_from_iter(iter);
            self
        }

        #[inline]
        pub fn extras(&mut self, value: optional_extras::OptionalExtras) {
            let offset = self.offset + 18;
            self.get_buf_mut().put_u8_at(offset, value.0)
        }

        // skipping CONSTANT enum 'discountedModel'

        /// COMPOSITE ENCODER
        #[inline]
        pub fn engine_encoder(self) -> engine_codec::EngineEncoder<Self> {
            let offset = self.offset + 19;
            engine_codec::EngineEncoder::default().wrap(self, offset)
        }

        /// primitive field 'added1'
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 29
        /// - encodedLength: 4
        /// - version: 2
        #[inline]
        pub fn added_1(&mut self, value: i32) -> &mut Self {
            let offset = self.offset + 29;
            self.get_buf_mut().put_i32_at(offset, value);
            self
        }

        /// optional primitive field 'added1'
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 29
        /// - encodedLength: 4
        /// - version: 2
        /// Set to `None` to encode the field null value.
        #[inline]
        pub fn added_1_opt(&mut self, value: Option<i32>) -> &mut Self {
            match value {
                Some(value) => self.added_1(value),
                None => self.added_1(-2147483648_i32),
            };
            self
        }

        /// REQUIRED enum
        #[inline]
        pub fn added_4(&mut self, value: boolean_type::BooleanType) -> &mut Self {
            let offset = self.offset + 33;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        /// optional enum field 'added4'
        /// - min value: 0
        /// - max value: 254
        /// - null value: 0xff_u8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 33
        /// - encodedLength: 1
        /// - version: 4
        /// Set to `None` to encode the field null value.
        #[inline]
        pub fn added_4_opt(&mut self, value: Option<boolean_type::BooleanType>) -> &mut Self {
            match value {
                Some(value) => self.added_4(value),
                None => self.added_4(boolean_type::BooleanType::NullVal),
            };
            self
        }

        /// COMPOSITE ENCODER
        #[inline]
        pub fn added_6_encoder(self) -> byte_pair_codec::BytePairEncoder<Self> {
            let offset = self.offset + 34;
            byte_pair_codec::BytePairEncoder::default().wrap(self, offset)
        }

        #[inline]
        pub fn added_7(&mut self, value: optional_extras::OptionalExtras) {
            let offset = self.offset + 36;
            self.get_buf_mut().put_u8_at(offset, value.0)
        }

        /// GROUP ENCODER (id=10)
        #[inline]
        pub fn fuel_figures_encoder(self, count: u16, fuel_figures_encoder: FuelFiguresEncoder<Self>) -> FuelFiguresEncoder<Self> {
            fuel_figures_encoder.wrap(self, count)
        }

        /// GROUP ENCODER (id=13)
        #[inline]
        pub fn performance_figures_encoder(self, count: u16, performance_figures_encoder: PerformanceFiguresEncoder<Self>) -> PerformanceFiguresEncoder<Self> {
            performance_figures_encoder.wrap(self, count)
        }

        /// VAR_DATA ENCODER - character encoding: 'UTF-8'
        #[inline]
        pub fn manufacturer(&mut self, value: &str) -> &mut Self {
            let limit = self.get_limit();
            let data_length = value.len().min((u32::MAX - 1) as usize);
            self.set_limit(limit + 4 + data_length);
            self.get_buf_mut().put_u32_at(limit, data_length as u32);
            self.get_buf_mut().put_slice_at(limit + 4, &value[0..data_length].as_bytes());
            self
        }

        /// VAR_DATA ENCODER - character encoding: 'UTF-8'
        #[inline]
        pub fn model(&mut self, value: &str) -> &mut Self {
            let limit = self.get_limit();
            let data_length = value.len().min((u32::MAX - 1) as usize);
            self.set_limit(limit + 4 + data_length);
            self.get_buf_mut().put_u32_at(limit, data_length as u32);
            self.get_buf_mut().put_slice_at(limit + 4, &value[0..data_length].as_bytes());
            self
        }

        /// VAR_DATA ENCODER - character encoding: 'None'
        #[inline]
        pub fn activation_code(&mut self, value: &[u8]) -> &mut Self {
            let limit = self.get_limit();
            let data_length = value.len().min((u32::MAX - 1) as usize);
            self.set_limit(limit + 4 + data_length);
            self.get_buf_mut().put_u32_at(limit, data_length as u32);
            self.get_buf_mut().put_slice_at(limit + 4, &value[0..data_length]);
            self
        }

        /// VAR_DATA ENCODER - character encoding: 'UTF-8'
        #[inline]
        pub fn added_5(&mut self, value: &str) -> &mut Self {
            let limit = self.get_limit();
            let data_length = value.len().min((u32::MAX - 1) as usize);
            self.set_limit(limit + 4 + data_length);
            self.get_buf_mut().put_u32_at(limit, data_length as u32);
            self.get_buf_mut().put_slice_at(limit + 4, &value[0..data_length].as_bytes());
            self
        }

        /// VAR_DATA ENCODER - character encoding: 'ASCII'
        #[inline]
        pub fn var_char_data(&mut self, value: &[u8]) -> &mut Self {
            let limit = self.get_limit();
            let data_length = value.len().min((u16::MAX - 1) as usize);
            self.set_limit(limit + 2 + data_length);
            self.get_buf_mut().put_u16_at(limit, data_length as u16);
            self.get_buf_mut().put_slice_at(limit + 2, &value[0..data_length]);
            self
        }

    }

    #[derive(Debug, Default)]
    pub struct FuelFiguresEncoder<P> {
        parent: Option<P>,
        count: u16,
        index: usize,
        offset: usize,
        initial_limit: usize,
    }

    impl<'a, P> Writer<'a> for FuelFiguresEncoder<P> where P: Writer<'a> + Default {
        #[inline]
        fn get_buf_mut(&mut self) -> &mut WriteBuf<'a> {
            if let Some(parent) = self.parent.as_mut() {
                parent.get_buf_mut()
            } else {
                panic!("parent was None")
            }
        }
    }

    impl<'a, P> Encoder<'a> for FuelFiguresEncoder<P> where P: Encoder<'a> + Default {
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
            self.added_3_opt(None);
            self
        }
    }

    impl<'a, P> FuelFiguresEncoder<P> where P: Encoder<'a> + Default {
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
            9
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

        /// primitive field 'speed'
        /// - min value: 0
        /// - max value: 65534
        /// - null value: 0xffff_u16
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 0
        /// - encodedLength: 2
        /// - version: 0
        #[inline]
        pub fn speed(&mut self, value: u16) -> &mut Self {
            let offset = self.offset;
            self.get_buf_mut().put_u16_at(offset, value);
            self
        }

        /// primitive field 'mpg'
        /// - min value: -3.4028234663852886E38
        /// - max value: 3.4028234663852886E38
        /// - null value: f32::NAN
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 2
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn mpg(&mut self, value: f32) -> &mut Self {
            let offset = self.offset + 2;
            self.get_buf_mut().put_f32_at(offset, value);
            self
        }

        #[inline]
        pub fn added_2_at(&mut self, index: usize, value: i8) -> &mut Self {
            let offset = self.offset + 6;
            let buf = self.get_buf_mut();
            buf.put_i8_at(offset + index, value);
            self
        }

        /// primitive array field 'added2'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 6
        /// - encodedLength: 2
        /// - version: 2
        #[inline]
        pub fn added_2(&mut self, value: &[i8]) -> &mut Self {
            debug_assert_eq!(2, value.len());
            let offset = self.offset + 6;
            let buf = self.get_buf_mut();
            buf.put_i8_at(offset, value[0]);
            buf.put_i8_at(offset + 1, value[1]);
            self
        }

        /// primitive array field 'added2' from an Iterator
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 6
        /// - encodedLength: 2
        /// - version: 2
        #[inline]
        pub fn added_2_from_iter(&mut self, iter: impl Iterator<Item = i8>) -> &mut Self {
            let offset = self.offset + 6;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_i8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'added2' with zero padding
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 6
        /// - encodedLength: 2
        /// - version: 2
        #[inline]
        pub fn added_2_zero_padded(&mut self, value: &[i8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_i8)).take(2);
            self.added_2_from_iter(iter);
            self
        }

        /// primitive field 'added3'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 8
        /// - encodedLength: 1
        /// - version: 3
        #[inline]
        pub fn added_3(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 8;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// optional primitive field 'added3'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 8
        /// - encodedLength: 1
        /// - version: 3
        /// Set to `None` to encode the field null value.
        #[inline]
        pub fn added_3_opt(&mut self, value: Option<i8>) -> &mut Self {
            match value {
                Some(value) => self.added_3(value),
                None => self.added_3(-128_i8),
            };
            self
        }

        /// VAR_DATA ENCODER - character encoding: 'UTF-8'
        #[inline]
        pub fn usage_description(&mut self, value: &str) -> &mut Self {
            let limit = self.get_limit();
            let data_length = value.len().min((u32::MAX - 1) as usize);
            self.set_limit(limit + 4 + data_length);
            self.get_buf_mut().put_u32_at(limit, data_length as u32);
            self.get_buf_mut().put_slice_at(limit + 4, &value[0..data_length].as_bytes());
            self
        }

    }

    #[derive(Debug, Default)]
    pub struct PerformanceFiguresEncoder<P> {
        parent: Option<P>,
        count: u16,
        index: usize,
        offset: usize,
        initial_limit: usize,
    }

    impl<'a, P> Writer<'a> for PerformanceFiguresEncoder<P> where P: Writer<'a> + Default {
        #[inline]
        fn get_buf_mut(&mut self) -> &mut WriteBuf<'a> {
            if let Some(parent) = self.parent.as_mut() {
                parent.get_buf_mut()
            } else {
                panic!("parent was None")
            }
        }
    }

    impl<'a, P> Encoder<'a> for PerformanceFiguresEncoder<P> where P: Encoder<'a> + Default {
        #[inline]
        fn get_limit(&self) -> usize {
            self.parent.as_ref().expect("parent missing").get_limit()
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.parent.as_mut().expect("parent missing").set_limit(limit);
        }
    }

    impl<'a, P> PerformanceFiguresEncoder<P> where P: Encoder<'a> + Default {
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
            1
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

        /// primitive field 'octaneRating'
        /// - min value: 90
        /// - max value: 110
        /// - null value: 0xff_u8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 0
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn octane_rating(&mut self, value: u8) -> &mut Self {
            let offset = self.offset;
            self.get_buf_mut().put_u8_at(offset, value);
            self
        }

        /// GROUP ENCODER (id=15)
        #[inline]
        pub fn acceleration_encoder(self, count: u16, acceleration_encoder: AccelerationEncoder<Self>) -> AccelerationEncoder<Self> {
            acceleration_encoder.wrap(self, count)
        }

    }

    #[derive(Debug, Default)]
    pub struct AccelerationEncoder<P> {
        parent: Option<P>,
        count: u16,
        index: usize,
        offset: usize,
        initial_limit: usize,
    }

    impl<'a, P> Writer<'a> for AccelerationEncoder<P> where P: Writer<'a> + Default {
        #[inline]
        fn get_buf_mut(&mut self) -> &mut WriteBuf<'a> {
            if let Some(parent) = self.parent.as_mut() {
                parent.get_buf_mut()
            } else {
                panic!("parent was None")
            }
        }
    }

    impl<'a, P> Encoder<'a> for AccelerationEncoder<P> where P: Encoder<'a> + Default {
        #[inline]
        fn get_limit(&self) -> usize {
            self.parent.as_ref().expect("parent missing").get_limit()
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.parent.as_mut().expect("parent missing").set_limit(limit);
        }
    }

    impl<'a, P> AccelerationEncoder<P> where P: Encoder<'a> + Default {
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
            6
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

        /// primitive field 'mph'
        /// - min value: 0
        /// - max value: 65534
        /// - null value: 0xffff_u16
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 0
        /// - encodedLength: 2
        /// - version: 0
        #[inline]
        pub fn mph(&mut self, value: u16) -> &mut Self {
            let offset = self.offset;
            self.get_buf_mut().put_u16_at(offset, value);
            self
        }

        /// primitive field 'seconds'
        /// - min value: -3.4028234663852886E38
        /// - max value: 3.4028234663852886E38
        /// - null value: f32::NAN
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 2
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn seconds(&mut self, value: f32) -> &mut Self {
            let offset = self.offset + 2;
            self.get_buf_mut().put_f32_at(offset, value);
            self
        }

    }

} // end encoder

pub mod decoder {
    use super::*;
    use message_header_codec::*;

    #[derive(Clone, Copy, Debug, Default)]
    pub struct ExtendedCarDecoder<'a> {
        buf: ReadBuf<'a>,
        initial_offset: usize,
        offset: usize,
        limit: usize,
        pub acting_block_length: u16,
        pub acting_version: u16,
    }

    impl ActingVersion for ExtendedCarDecoder<'_> {
        #[inline]
        fn acting_version(&self) -> u16 {
            self.acting_version
        }
    }

    impl<'a> Reader<'a> for ExtendedCarDecoder<'a> {
        #[inline]
        fn get_buf(&self) -> &ReadBuf<'a> {
            &self.buf
        }
    }

    impl<'a> Decoder<'a> for ExtendedCarDecoder<'a> {
        #[inline]
        fn get_limit(&self) -> usize {
            self.limit
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.limit = limit;
        }
    }

    impl<'a> ExtendedCarDecoder<'a> {
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
        pub fn serial_number(&self) -> u64 {
            self.get_buf().get_u64_at(self.offset)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn model_year(&self) -> u16 {
            self.get_buf().get_u16_at(self.offset + 8)
        }

        /// REQUIRED enum
        #[inline]
        pub fn available(&self) -> boolean_type::BooleanType {
            self.get_buf().get_u8_at(self.offset + 10).into()
        }

        /// REQUIRED enum
        #[inline]
        pub fn code(&self) -> model::Model {
            self.get_buf().get_u8_at(self.offset + 11).into()
        }

        #[inline]
        pub fn vehicle_code(&self) -> [u8; 6] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 12)
        }

        /// BIT SET DECODER
        #[inline]
        pub fn extras(&self) -> optional_extras::OptionalExtras {
            optional_extras::OptionalExtras::new(self.get_buf().get_u8_at(self.offset + 18))
        }

        /// CONSTANT enum
        #[inline]
        pub fn discounted_model(&self) -> model::Model {
            model::Model::C
        }

        /// COMPOSITE DECODER
        #[inline]
        pub fn engine_decoder(self) -> engine_codec::EngineDecoder<Self> {
            let offset = self.offset + 19;
            engine_codec::EngineDecoder::default().wrap(self, offset)
        }

        /// primitive field - 'OPTIONAL' { null_value: '-2147483648_i32' }
        #[inline]
        pub fn added_1(&self) -> Option<i32> {
            if self.acting_version() < 2 {
                return None;
            }

            let value = self.get_buf().get_i32_at(self.offset + 29);
            if value == -2147483648_i32 {
                None
            } else {
                Some(value)
            }
        }

        /// REQUIRED enum
        #[inline]
        pub fn added_4(&self) -> boolean_type::BooleanType {
            if self.acting_version() < 4 {
                return boolean_type::BooleanType::default();
            }

            self.get_buf().get_u8_at(self.offset + 33).into()
        }

        /// COMPOSITE DECODER
        #[inline]
        pub fn added_6_decoder(self) -> Either<Self, byte_pair_codec::BytePairDecoder<Self>> {
            if self.acting_version() < 6 {
                return Either::Left(self);
            }

            let offset = self.offset + 34;
            Either::Right(byte_pair_codec::BytePairDecoder::default().wrap(self, offset))
        }

        /// BIT SET DECODER
        #[inline]
        pub fn added_7(&self) -> optional_extras::OptionalExtras {
            if self.acting_version() < 6 {
                return optional_extras::OptionalExtras::default();
            }

            optional_extras::OptionalExtras::new(self.get_buf().get_u8_at(self.offset + 36))
        }

        /// GROUP DECODER (id=10)
        #[inline]
        pub fn fuel_figures_decoder(self) -> FuelFiguresDecoder<Self> {
            FuelFiguresDecoder::default().wrap(self)
        }

        /// GROUP DECODER (id=13)
        #[inline]
        pub fn performance_figures_decoder(self) -> PerformanceFiguresDecoder<Self> {
            PerformanceFiguresDecoder::default().wrap(self)
        }

        /// VAR_DATA DECODER - character encoding: 'UTF-8'
        #[inline]
        pub fn manufacturer_decoder(&mut self) -> (usize, usize) {
            let offset = self.get_limit();
            let data_length = self.get_buf().get_u32_at(offset) as usize;
            self.set_limit(offset + 4 + data_length);
            (offset + 4, data_length)
        }

        #[inline]
        pub fn manufacturer_slice(&'a self, coordinates: (usize, usize)) -> &'a [u8] {
            debug_assert!(self.get_limit() >= coordinates.0 + coordinates.1);
            self.get_buf().get_slice_at(coordinates.0, coordinates.1)
        }

        /// VAR_DATA DECODER - character encoding: 'UTF-8'
        #[inline]
        pub fn model_decoder(&mut self) -> (usize, usize) {
            let offset = self.get_limit();
            let data_length = self.get_buf().get_u32_at(offset) as usize;
            self.set_limit(offset + 4 + data_length);
            (offset + 4, data_length)
        }

        #[inline]
        pub fn model_slice(&'a self, coordinates: (usize, usize)) -> &'a [u8] {
            debug_assert!(self.get_limit() >= coordinates.0 + coordinates.1);
            self.get_buf().get_slice_at(coordinates.0, coordinates.1)
        }

        /// VAR_DATA DECODER - character encoding: 'None'
        #[inline]
        pub fn activation_code_decoder(&mut self) -> (usize, usize) {
            let offset = self.get_limit();
            let data_length = self.get_buf().get_u32_at(offset) as usize;
            self.set_limit(offset + 4 + data_length);
            (offset + 4, data_length)
        }

        #[inline]
        pub fn activation_code_slice(&'a self, coordinates: (usize, usize)) -> &'a [u8] {
            debug_assert!(self.get_limit() >= coordinates.0 + coordinates.1);
            self.get_buf().get_slice_at(coordinates.0, coordinates.1)
        }

        /// VAR_DATA DECODER - character encoding: 'UTF-8'
        #[inline]
        pub fn added_5_decoder(&mut self) -> (usize, usize) {
            if self.acting_version() < 5 {
                return (self.get_limit(), 0);
            }

            let offset = self.get_limit();
            let data_length = self.get_buf().get_u32_at(offset) as usize;
            self.set_limit(offset + 4 + data_length);
            (offset + 4, data_length)
        }

        #[inline]
        pub fn added_5_slice(&'a self, coordinates: (usize, usize)) -> &'a [u8] {
            if self.acting_version() < 5 {
                return &[] as &[u8];
            }

            debug_assert!(self.get_limit() >= coordinates.0 + coordinates.1);
            self.get_buf().get_slice_at(coordinates.0, coordinates.1)
        }

        /// VAR_DATA DECODER - character encoding: 'ASCII'
        #[inline]
        pub fn var_char_data_decoder(&mut self) -> (usize, usize) {
            if self.acting_version() < 6 {
                return (self.get_limit(), 0);
            }

            let offset = self.get_limit();
            let data_length = self.get_buf().get_u16_at(offset) as usize;
            self.set_limit(offset + 2 + data_length);
            (offset + 2, data_length)
        }

        #[inline]
        pub fn var_char_data_slice(&'a self, coordinates: (usize, usize)) -> &'a [u8] {
            if self.acting_version() < 6 {
                return &[] as &[u8];
            }

            debug_assert!(self.get_limit() >= coordinates.0 + coordinates.1);
            self.get_buf().get_slice_at(coordinates.0, coordinates.1)
        }

    }

    #[derive(Debug, Default)]
    pub struct FuelFiguresDecoder<P> {
        parent: Option<P>,
        block_length: u16,
        count: u16,
        index: usize,
        offset: usize,
    }

    impl<'a, P> ActingVersion for FuelFiguresDecoder<P> where P: Reader<'a> + ActingVersion + Default {
        #[inline]
        fn acting_version(&self) -> u16 {
            self.parent.as_ref().unwrap().acting_version()
        }
    }

    impl<'a, P> Reader<'a> for FuelFiguresDecoder<P> where P: Reader<'a> + Default {
        #[inline]
        fn get_buf(&self) -> &ReadBuf<'a> {
            self.parent.as_ref().expect("parent missing").get_buf()
        }
    }

    impl<'a, P> Decoder<'a> for FuelFiguresDecoder<P> where P: Decoder<'a> + ActingVersion + Default {
        #[inline]
        fn get_limit(&self) -> usize {
            self.parent.as_ref().expect("parent missing").get_limit()
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.parent.as_mut().expect("parent missing").set_limit(limit);
        }
    }

    impl<'a, P> FuelFiguresDecoder<P> where P: Decoder<'a> + ActingVersion + Default {
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

        /// group token - Token{signal=BEGIN_GROUP, name='fuelFigures', referencedName='null', description='null', packageName='null', id=10, version=0, deprecated=0, encodedLength=9, offset=37, componentTokenCount=24, encoding=Encoding{presence=REQUIRED, primitiveType=null, byteOrder=LITTLE_ENDIAN, minValue=null, maxValue=null, nullValue=null, constValue=null, characterEncoding='null', epoch='null', timeUnit=null, semanticType='null'}}
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
        pub fn speed(&self) -> u16 {
            self.get_buf().get_u16_at(self.offset)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn mpg(&self) -> f32 {
            self.get_buf().get_f32_at(self.offset + 2)
        }

        #[inline]
        pub fn added_2(&self) -> [i8; 2] {
            if self.acting_version() < 2 {
                return [-128_i8; 2];
            }

            let buf = self.get_buf();
            [
                buf.get_i8_at(self.offset + 6),
                buf.get_i8_at(self.offset + 6 + 1),
            ]
        }

        /// primitive field - 'OPTIONAL' { null_value: '-128_i8' }
        #[inline]
        pub fn added_3(&self) -> Option<i8> {
            if self.acting_version() < 3 {
                return None;
            }

            let value = self.get_buf().get_i8_at(self.offset + 8);
            if value == -128_i8 {
                None
            } else {
                Some(value)
            }
        }

        /// VAR_DATA DECODER - character encoding: 'UTF-8'
        #[inline]
        pub fn usage_description_decoder(&mut self) -> (usize, usize) {
            let offset = self.parent.as_ref().expect("parent missing").get_limit();
            let data_length = self.get_buf().get_u32_at(offset) as usize;
            self.parent.as_mut().unwrap().set_limit(offset + 4 + data_length);
            (offset + 4, data_length)
        }

        #[inline]
        pub fn usage_description_slice(&'a self, coordinates: (usize, usize)) -> &'a [u8] {
            debug_assert!(self.get_limit() >= coordinates.0 + coordinates.1);
            self.get_buf().get_slice_at(coordinates.0, coordinates.1)
        }

    }

    #[derive(Debug, Default)]
    pub struct PerformanceFiguresDecoder<P> {
        parent: Option<P>,
        block_length: u16,
        count: u16,
        index: usize,
        offset: usize,
    }

    impl<'a, P> ActingVersion for PerformanceFiguresDecoder<P> where P: Reader<'a> + ActingVersion + Default {
        #[inline]
        fn acting_version(&self) -> u16 {
            self.parent.as_ref().unwrap().acting_version()
        }
    }

    impl<'a, P> Reader<'a> for PerformanceFiguresDecoder<P> where P: Reader<'a> + Default {
        #[inline]
        fn get_buf(&self) -> &ReadBuf<'a> {
            self.parent.as_ref().expect("parent missing").get_buf()
        }
    }

    impl<'a, P> Decoder<'a> for PerformanceFiguresDecoder<P> where P: Decoder<'a> + ActingVersion + Default {
        #[inline]
        fn get_limit(&self) -> usize {
            self.parent.as_ref().expect("parent missing").get_limit()
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.parent.as_mut().expect("parent missing").set_limit(limit);
        }
    }

    impl<'a, P> PerformanceFiguresDecoder<P> where P: Decoder<'a> + ActingVersion + Default {
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

        /// group token - Token{signal=BEGIN_GROUP, name='performanceFigures', referencedName='null', description='null', packageName='null', id=13, version=0, deprecated=0, encodedLength=1, offset=-1, componentTokenCount=21, encoding=Encoding{presence=REQUIRED, primitiveType=null, byteOrder=LITTLE_ENDIAN, minValue=null, maxValue=null, nullValue=null, constValue=null, characterEncoding='null', epoch='null', timeUnit=null, semanticType='null'}}
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
        pub fn octane_rating(&self) -> u8 {
            self.get_buf().get_u8_at(self.offset)
        }

        /// GROUP DECODER (id=15)
        #[inline]
        pub fn acceleration_decoder(self) -> AccelerationDecoder<Self> {
            AccelerationDecoder::default().wrap(self)
        }

    }

    #[derive(Debug, Default)]
    pub struct AccelerationDecoder<P> {
        parent: Option<P>,
        block_length: u16,
        count: u16,
        index: usize,
        offset: usize,
    }

    impl<'a, P> ActingVersion for AccelerationDecoder<P> where P: Reader<'a> + ActingVersion + Default {
        #[inline]
        fn acting_version(&self) -> u16 {
            self.parent.as_ref().unwrap().acting_version()
        }
    }

    impl<'a, P> Reader<'a> for AccelerationDecoder<P> where P: Reader<'a> + Default {
        #[inline]
        fn get_buf(&self) -> &ReadBuf<'a> {
            self.parent.as_ref().expect("parent missing").get_buf()
        }
    }

    impl<'a, P> Decoder<'a> for AccelerationDecoder<P> where P: Decoder<'a> + ActingVersion + Default {
        #[inline]
        fn get_limit(&self) -> usize {
            self.parent.as_ref().expect("parent missing").get_limit()
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.parent.as_mut().expect("parent missing").set_limit(limit);
        }
    }

    impl<'a, P> AccelerationDecoder<P> where P: Decoder<'a> + ActingVersion + Default {
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

        /// group token - Token{signal=BEGIN_GROUP, name='acceleration', referencedName='null', description='null', packageName='null', id=15, version=0, deprecated=0, encodedLength=6, offset=1, componentTokenCount=12, encoding=Encoding{presence=REQUIRED, primitiveType=null, byteOrder=LITTLE_ENDIAN, minValue=null, maxValue=null, nullValue=null, constValue=null, characterEncoding='null', epoch='null', timeUnit=null, semanticType='null'}}
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
        pub fn mph(&self) -> u16 {
            self.get_buf().get_u16_at(self.offset)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn seconds(&self) -> f32 {
            self.get_buf().get_f32_at(self.offset + 2)
        }

    }

} // end decoder
