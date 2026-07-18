#![forbid(unsafe_code)]
#![allow(clippy::all)]
#![allow(non_camel_case_types)]
#![allow(ambiguous_glob_reexports)]

use ::core::convert::TryInto;

pub mod add_passive_member_codec;
pub mod admin_request_codec;
pub mod admin_request_type;
pub mod admin_response_code;
pub mod admin_response_codec;
pub mod append_position_codec;
pub mod backup_query_codec;
pub mod backup_response_codec;
pub mod boolean_type;
pub mod cancel_timer_codec;
pub mod canvass_position_codec;
pub mod catchup_position_codec;
pub mod challenge_codec;
pub mod challenge_response_codec;
pub mod change_type;
pub mod client_session_codec;
pub mod close_reason;
pub mod close_session_codec;
pub mod cluster_action;
pub mod cluster_action_request_codec;
pub mod cluster_members_change_codec;
pub mod cluster_members_codec;
pub mod cluster_members_extended_response_codec;
pub mod cluster_members_query_codec;
pub mod cluster_members_response_codec;
pub mod cluster_session_codec;
pub mod cluster_time_unit;
pub mod commit_position_codec;
pub mod consensus_module_codec;
pub mod event_code;
pub mod group_size_encoding_codec;
pub mod heartbeat_request_codec;
pub mod heartbeat_response_codec;
pub mod join_cluster_codec;
pub mod join_log_codec;
pub mod membership_change_event_codec;
pub mod message_header_codec;
pub mod new_leader_event_codec;
pub mod new_leadership_term_codec;
pub mod new_leadership_term_event_codec;
pub mod pending_message_tracker_codec;
pub mod remove_member_codec;
pub mod request_service_ack_codec;
pub mod request_vote_codec;
pub mod schedule_timer_codec;
pub mod service_ack_codec;
pub mod service_termination_position_codec;
pub mod session_close_event_codec;
pub mod session_close_request_codec;
pub mod session_connect_request_codec;
pub mod session_event_codec;
pub mod session_keep_alive_codec;
pub mod session_message_header_codec;
pub mod session_open_event_codec;
pub mod snapshot_mark;
pub mod snapshot_marker_codec;
pub mod snapshot_recording_query_codec;
pub mod snapshot_recordings_codec;
pub mod standby_snapshot_codec;
pub mod stop_catchup_codec;
pub mod termination_ack_codec;
pub mod termination_position_codec;
pub mod timer_codec;
pub mod timer_event_codec;
pub mod var_ascii_encoding_codec;
pub mod var_data_encoding_codec;
pub mod vote_codec;

pub const SBE_SCHEMA_ID: u16 = 111;
pub const SBE_SCHEMA_VERSION: u16 = 16;
pub const SBE_SEMANTIC_VERSION: &str = "5.4";

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
    pub(crate) fn get_bytes_at<const N: usize>(slice: &[u8], index: usize) -> [u8; N] {
        slice[index..index + N].try_into().expect("slice with incorrect length")
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
    pub fn put_bytes_at<const COUNT: usize>(&mut self, index: usize, bytes: &[u8; COUNT]) -> usize {
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
