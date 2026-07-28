#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::restriction)]
#![allow(dead_code)]
#![allow(unused)]

pub mod l3_codec {
    include!(concat!(env!("OUT_DIR"), "/l3_codec.rs"));
}

pub mod orderbook_codec {
    include!(concat!(env!("OUT_DIR"), "/orderbook_codec.rs"));
}
