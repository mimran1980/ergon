//! The versioned nested L3 book that the memoized random-access decoder is
//! evaluated against, plus one encoder per acting version.
//!
//! Shared by `tests/versioned_l3_differential_test.rs` and
//! `benches/versioned_l3_bench.rs` so the benchmark and the differential test
//! can never drift onto different shapes.

#![allow(clippy::all, clippy::pedantic, clippy::restriction)]

use crate::versioned_l3::*;
use crate::{versioned_l3_v0, versioned_l3_v1, versioned_l3_v2};

type Res<T> = Result<T, Box<dyn std::error::Error>>;

pub struct LegSpec {
    pub qty: i64,
    pub reference: &'static [u8],
}
pub struct AllocSpec {
    pub qty: i64,
    pub legs: &'static [LegSpec],
}
pub struct OrderSpec {
    pub qty: i64,
    pub allocations: &'static [AllocSpec],
    pub id: &'static [u8],
    pub trader: &'static [u8],
}
pub struct StatSpec {
    pub fills: u64,
    pub qty: i64,
}
pub struct LevelSpec {
    pub price: i64,
    pub qty: i64,
    pub participant: u64,
    pub orders: &'static [OrderSpec],
    pub stats: &'static [StatSpec],
    pub venue: &'static [u8],
}
pub struct AuditSpec {
    pub ts: u64,
    pub code: u16,
}
pub struct BookSpec {
    pub label: &'static str,
    pub timestamp: u64,
    pub sequence: u64,
    pub epoch: u32,
    pub flags: u32,
    pub bids: &'static [LevelSpec],
    pub asks: &'static [LevelSpec],
    pub audit: &'static [AuditSpec],
    pub symbol: &'static [u8],
    pub source: &'static [u8],
    pub checksum: &'static [u8],
    pub note: &'static [u8],
}

/// Dense: every group non-empty somewhere, every group empty somewhere,
/// var-data both empty and populated at every depth.
pub static DENSE: BookSpec = BookSpec {
    label: "dense",
    timestamp: 0x0102_0304_0506_0708,
    sequence: 42,
    epoch: 7,
    flags: 0xDEAD_BEEF,
    bids: &[
        LevelSpec {
            price: 100,
            qty: 10,
            participant: 1,
            orders: &[
                OrderSpec {
                    qty: 5,
                    allocations: &[
                        AllocSpec {
                            qty: 9,
                            legs: &[
                                LegSpec {
                                    qty: 3,
                                    reference: b"leg-alpha",
                                },
                                LegSpec {
                                    qty: 4,
                                    reference: b"",
                                },
                            ],
                        },
                        AllocSpec { qty: 7, legs: &[] },
                    ],
                    id: b"ord-0",
                    trader: b"t0",
                },
                OrderSpec {
                    qty: 6,
                    allocations: &[],
                    id: b"",
                    trader: b"trader-one",
                },
            ],
            stats: &[StatSpec { fills: 1, qty: 2 }, StatSpec { fills: 3, qty: 4 }],
            venue: b"XNAS",
        },
        LevelSpec {
            price: 101,
            qty: 0,
            participant: 2,
            orders: &[],
            stats: &[],
            venue: b"",
        },
        LevelSpec {
            price: 102,
            qty: 20,
            participant: 3,
            orders: &[OrderSpec {
                qty: 7,
                allocations: &[AllocSpec {
                    qty: 1,
                    legs: &[LegSpec {
                        qty: 2,
                        reference: b"z",
                    }],
                }],
                id: b"ord-2",
                trader: b"",
            }],
            stats: &[StatSpec { fills: 5, qty: 6 }],
            venue: b"XLON",
        },
    ],
    asks: &[
        LevelSpec {
            price: 200,
            qty: 30,
            participant: 4,
            orders: &[OrderSpec {
                qty: 8,
                // The ask side carries its own nesting: `askAllocations` and
                // `askLegs` are separate generated code from the bid side.
                allocations: &[AllocSpec {
                    qty: 4,
                    legs: &[LegSpec {
                        qty: 4,
                        reference: b"ask-leg",
                    }],
                }],
                id: b"ask-0",
                trader: b"ta",
            }],
            stats: &[],
            venue: b"BATS",
        },
        LevelSpec {
            price: 201,
            qty: 31,
            participant: 5,
            orders: &[],
            stats: &[StatSpec { fills: 7, qty: 8 }],
            venue: b"",
        },
    ],
    audit: &[AuditSpec { ts: 11, code: 2 }, AuditSpec { ts: 13, code: 4 }],
    symbol: b"IBM",
    source: b"feed-a",
    checksum: b"",
    note: b"note-text",
};

/// Sparse: one level, one order, nothing nested below it.
pub static SPARSE: BookSpec = BookSpec {
    label: "sparse",
    timestamp: 1,
    sequence: 2,
    epoch: 3,
    flags: 4,
    bids: &[LevelSpec {
        price: 5,
        qty: 6,
        participant: 7,
        orders: &[OrderSpec {
            qty: 8,
            allocations: &[],
            id: b"o",
            trader: b"t",
        }],
        stats: &[],
        venue: b"V",
    }],
    asks: &[],
    audit: &[],
    symbol: b"S",
    source: b"",
    checksum: b"",
    note: b"",
};

/// Empty: every group empty, every var-data zero-length.
pub static EMPTY: BookSpec = BookSpec {
    label: "empty",
    timestamp: 0,
    sequence: 0,
    epoch: 0,
    flags: 0,
    bids: &[],
    asks: &[],
    audit: &[],
    symbol: b"",
    source: b"",
    checksum: b"",
    note: b"",
};

pub static SPECS: &[&BookSpec] = &[&EMPTY, &SPARSE, &DENSE];

pub fn encode_v3(spec: &BookSpec) -> Res<Vec<u8>> {
    fn levels_len(
        b: &mut L3BookBidsRaggedBuilder<'_>,
        levels: &[LevelSpec],
    ) -> Result<(), sbe_rt::EncodeError> {
        for lvl in levels {
            b.add()?
                .orders(|o| {
                    for ord in lvl.orders {
                        o.add()?
                            .allocations(|a| {
                                for al in ord.allocations {
                                    a.add()?.legs(|l| {
                                        for leg in al.legs {
                                            l.add()?.leg_ref(leg.reference.len())?;
                                        }
                                        Ok(())
                                    })?;
                                }
                                Ok(())
                            })?
                            .order_id(ord.id.len())?
                            .trader_id(ord.trader.len())?;
                    }
                    Ok(())
                })?
                .stats(|s| {
                    s.uniform(lvl.stats.len())?;
                    Ok(())
                })?
                .venue(lvl.venue.len())?;
        }
        Ok(())
    }
    fn ask_levels_len(
        b: &mut L3BookAsksRaggedBuilder<'_>,
        levels: &[LevelSpec],
    ) -> Result<(), sbe_rt::EncodeError> {
        for lvl in levels {
            b.add()?
                .ask_orders(|o| {
                    for ord in lvl.orders {
                        o.add()?
                            .ask_allocations(|a| {
                                for al in ord.allocations {
                                    a.add()?.ask_legs(|l| {
                                        for leg in al.legs {
                                            l.add()?.leg_ref(leg.reference.len())?;
                                        }
                                        Ok(())
                                    })?;
                                }
                                Ok(())
                            })?
                            .order_id(ord.id.len())?
                            .trader_id(ord.trader.len())?;
                    }
                    Ok(())
                })?
                .ask_stats(|s| {
                    s.uniform(lvl.stats.len())?;
                    Ok(())
                })?
                .venue(lvl.venue.len())?;
        }
        Ok(())
    }

    let len = L3BookEncodedLength::new()
        .bids_ragged(spec.bids.len() as u16, |b| levels_len(b, spec.bids))?
        .asks_ragged(spec.asks.len() as u16, |b| ask_levels_len(b, spec.asks))?
        .audit(spec.audit.len() as u16)?
        .symbol(spec.symbol.len())?
        .source(spec.source.len())?
        .checksum(spec.checksum.len())?
        .note(spec.note.len())?
        .encoded_length_with_header();

    let mut buf = vec![0u8; len];
    let written = L3BookEncoder::try_wrap_and_apply_header(&mut buf, 0)?
        .fixed(&L3BookFixedFields {
            timestamp: spec.timestamp,
            sequence: spec.sequence,
            epoch: spec.epoch,
            flags: spec.flags,
        })
        .bids(spec.bids.len() as u16, |g| {
            for lvl in spec.bids {
                g.add(|mut entry| {
                    entry
                        .price(lvl.price)
                        .qty(lvl.qty)
                        .participant(lvl.participant);
                    entry
                        .orders(lvl.orders.len() as u16, |o| {
                            for ord in lvl.orders {
                                o.add(|mut e| {
                                    e.order_qty(ord.qty);
                                    e.allocations(ord.allocations.len() as u16, |a| {
                                        for al in ord.allocations {
                                            a.add(|mut ae| {
                                                ae.alloc_qty(al.qty);
                                                ae.legs(al.legs.len() as u16, |l| {
                                                    for leg in al.legs {
                                                        l.add(|mut le| {
                                                            le.leg_qty(leg.qty);
                                                            le.leg_ref(leg.reference)
                                                        })?;
                                                    }
                                                    Ok(())
                                                })
                                            })?;
                                        }
                                        Ok(())
                                    })?
                                    .order_id(ord.id)?
                                    .trader_id(ord.trader)
                                })?;
                            }
                            Ok(())
                        })?
                        .stats(lvl.stats.len() as u16, |s| {
                            for st in lvl.stats {
                                s.add(|e| {
                                    e.fill_count(st.fills).fill_qty(st.qty);
                                    Ok(())
                                })?;
                            }
                            Ok(())
                        })?
                        .venue(lvl.venue)
                })?;
            }
            Ok(())
        })?
        .asks(spec.asks.len() as u16, |g| {
            for lvl in spec.asks {
                g.add(|mut entry| {
                    entry
                        .price(lvl.price)
                        .qty(lvl.qty)
                        .participant(lvl.participant);
                    entry
                        .ask_orders(lvl.orders.len() as u16, |o| {
                            for ord in lvl.orders {
                                o.add(|mut e| {
                                    e.order_qty(ord.qty);
                                    e.ask_allocations(ord.allocations.len() as u16, |a| {
                                        for al in ord.allocations {
                                            a.add(|mut ae| {
                                                ae.alloc_qty(al.qty);
                                                ae.ask_legs(al.legs.len() as u16, |l| {
                                                    for leg in al.legs {
                                                        l.add(|mut le| {
                                                            le.leg_qty(leg.qty);
                                                            le.leg_ref(leg.reference)
                                                        })?;
                                                    }
                                                    Ok(())
                                                })
                                            })?;
                                        }
                                        Ok(())
                                    })?
                                    .order_id(ord.id)?
                                    .trader_id(ord.trader)
                                })?;
                            }
                            Ok(())
                        })?
                        .ask_stats(lvl.stats.len() as u16, |s| {
                            for st in lvl.stats {
                                s.add(|e| {
                                    e.fill_count(st.fills).fill_qty(st.qty);
                                    Ok(())
                                })?;
                            }
                            Ok(())
                        })?
                        .venue(lvl.venue)
                })?;
            }
            Ok(())
        })?
        .audit(spec.audit.len() as u16, |g| {
            for row in spec.audit {
                g.add(|e| {
                    e.ts(row.ts).code(row.code);
                    Ok(())
                })?;
            }
            Ok(())
        })?
        .symbol(spec.symbol)?
        .source(spec.source)?
        .checksum(spec.checksum)?
        .note(spec.note)?
        .encoded_length_with_header();
    assert_eq!(
        len, written,
        "{}: EncodedLength disagreed with encode",
        spec.label
    );
    Ok(buf)
}

pub fn encode_v2(spec: &BookSpec) -> Res<Vec<u8>> {
    use versioned_l3_v2 as v;
    let len = v::L3BookEncodedLength::new()
        .bids_ragged(spec.bids.len() as u16, |b| {
            for lvl in spec.bids {
                b.add()?
                    .orders(|o| {
                        for ord in lvl.orders {
                            o.add()?
                                .allocations(|a| {
                                    a.uniform(ord.allocations.len())?;
                                    Ok(())
                                })?
                                .order_id(ord.id.len())?
                                .trader_id(ord.trader.len())?;
                        }
                        Ok(())
                    })?
                    .stats(|s| {
                        s.uniform(lvl.stats.len())?;
                        Ok(())
                    })?
                    .venue(lvl.venue.len())?;
            }
            Ok(())
        })?
        .asks_ragged(spec.asks.len() as u16, |b| {
            for lvl in spec.asks {
                b.add()?
                    .ask_orders(|o| {
                        for ord in lvl.orders {
                            o.add()?
                                .ask_allocations(|a| {
                                    a.uniform(ord.allocations.len())?;
                                    Ok(())
                                })?
                                .order_id(ord.id.len())?
                                .trader_id(ord.trader.len())?;
                        }
                        Ok(())
                    })?
                    .ask_stats(|s| {
                        s.uniform(lvl.stats.len())?;
                        Ok(())
                    })?
                    .venue(lvl.venue.len())?;
            }
            Ok(())
        })?
        .audit(spec.audit.len() as u16)?
        .symbol(spec.symbol.len())?
        .source(spec.source.len())?
        .checksum(spec.checksum.len())?
        .encoded_length_with_header();

    let mut buf = vec![0u8; len];
    let written = v::L3BookEncoder::try_wrap_and_apply_header(&mut buf, 0)?
        .fixed(&v::L3BookFixedFields {
            timestamp: spec.timestamp,
            sequence: spec.sequence,
            epoch: spec.epoch,
        })
        .bids(spec.bids.len() as u16, |g| {
            for lvl in spec.bids {
                g.add(|mut entry| {
                    entry
                        .price(lvl.price)
                        .qty(lvl.qty)
                        .participant(lvl.participant);
                    entry
                        .orders(lvl.orders.len() as u16, |o| {
                            for ord in lvl.orders {
                                o.add(|mut e| {
                                    e.order_qty(ord.qty);
                                    e.allocations(ord.allocations.len() as u16, |a| {
                                        for al in ord.allocations {
                                            a.add(|ae| {
                                                ae.alloc_qty(al.qty);
                                                Ok(())
                                            })?;
                                        }
                                        Ok(())
                                    })?
                                    .order_id(ord.id)?
                                    .trader_id(ord.trader)
                                })?;
                            }
                            Ok(())
                        })?
                        .stats(lvl.stats.len() as u16, |s| {
                            for st in lvl.stats {
                                s.add(|e| {
                                    e.fill_count(st.fills).fill_qty(st.qty);
                                    Ok(())
                                })?;
                            }
                            Ok(())
                        })?
                        .venue(lvl.venue)
                })?;
            }
            Ok(())
        })?
        .asks(spec.asks.len() as u16, |g| {
            for lvl in spec.asks {
                g.add(|mut entry| {
                    entry
                        .price(lvl.price)
                        .qty(lvl.qty)
                        .participant(lvl.participant);
                    entry
                        .ask_orders(lvl.orders.len() as u16, |o| {
                            for ord in lvl.orders {
                                o.add(|mut e| {
                                    e.order_qty(ord.qty);
                                    e.ask_allocations(ord.allocations.len() as u16, |a| {
                                        for al in ord.allocations {
                                            a.add(|ae| {
                                                ae.alloc_qty(al.qty);
                                                Ok(())
                                            })?;
                                        }
                                        Ok(())
                                    })?
                                    .order_id(ord.id)?
                                    .trader_id(ord.trader)
                                })?;
                            }
                            Ok(())
                        })?
                        .ask_stats(lvl.stats.len() as u16, |s| {
                            for st in lvl.stats {
                                s.add(|e| {
                                    e.fill_count(st.fills).fill_qty(st.qty);
                                    Ok(())
                                })?;
                            }
                            Ok(())
                        })?
                        .venue(lvl.venue)
                })?;
            }
            Ok(())
        })?
        .audit(spec.audit.len() as u16, |g| {
            for row in spec.audit {
                g.add(|e| {
                    e.ts(row.ts).code(row.code);
                    Ok(())
                })?;
            }
            Ok(())
        })?
        .symbol(spec.symbol)?
        .source(spec.source)?
        .checksum(spec.checksum)?
        .encoded_length_with_header();
    assert_eq!(len, written, "{}: v2 EncodedLength disagreed", spec.label);
    Ok(buf)
}

pub fn encode_v1(spec: &BookSpec) -> Res<Vec<u8>> {
    use versioned_l3_v1 as v;
    let len = v::L3BookEncodedLength::new()
        .bids_ragged(spec.bids.len() as u16, |b| {
            for lvl in spec.bids {
                b.add()?
                    .orders(|o| {
                        for ord in lvl.orders {
                            o.add()?
                                .order_id(ord.id.len())?
                                .trader_id(ord.trader.len())?;
                        }
                        Ok(())
                    })?
                    .venue(lvl.venue.len())?;
            }
            Ok(())
        })?
        .asks_ragged(spec.asks.len() as u16, |b| {
            for lvl in spec.asks {
                b.add()?
                    .ask_orders(|o| {
                        for ord in lvl.orders {
                            o.add()?
                                .order_id(ord.id.len())?
                                .trader_id(ord.trader.len())?;
                        }
                        Ok(())
                    })?
                    .venue(lvl.venue.len())?;
            }
            Ok(())
        })?
        .symbol(spec.symbol.len())?
        .source(spec.source.len())?
        .encoded_length_with_header();

    let mut buf = vec![0u8; len];
    let written = v::L3BookEncoder::try_wrap_and_apply_header(&mut buf, 0)?
        .fixed(&v::L3BookFixedFields {
            timestamp: spec.timestamp,
            sequence: spec.sequence,
            epoch: spec.epoch,
        })
        .bids(spec.bids.len() as u16, |g| {
            for lvl in spec.bids {
                g.add(|mut entry| {
                    entry
                        .price(lvl.price)
                        .qty(lvl.qty)
                        .participant(lvl.participant);
                    entry
                        .orders(lvl.orders.len() as u16, |o| {
                            for ord in lvl.orders {
                                o.add(|mut e| {
                                    e.order_qty(ord.qty);
                                    e.order_id(ord.id)?.trader_id(ord.trader)
                                })?;
                            }
                            Ok(())
                        })?
                        .venue(lvl.venue)
                })?;
            }
            Ok(())
        })?
        .asks(spec.asks.len() as u16, |g| {
            for lvl in spec.asks {
                g.add(|mut entry| {
                    entry
                        .price(lvl.price)
                        .qty(lvl.qty)
                        .participant(lvl.participant);
                    entry
                        .ask_orders(lvl.orders.len() as u16, |o| {
                            for ord in lvl.orders {
                                o.add(|mut e| {
                                    e.order_qty(ord.qty);
                                    e.order_id(ord.id)?.trader_id(ord.trader)
                                })?;
                            }
                            Ok(())
                        })?
                        .venue(lvl.venue)
                })?;
            }
            Ok(())
        })?
        .symbol(spec.symbol)?
        .source(spec.source)?
        .encoded_length_with_header();
    assert_eq!(len, written, "{}: v1 EncodedLength disagreed", spec.label);
    Ok(buf)
}

pub fn encode_v0(spec: &BookSpec) -> Res<Vec<u8>> {
    use versioned_l3_v0 as v;
    let len = v::L3BookEncodedLength::new()
        .bids_ragged(spec.bids.len() as u16, |b| {
            for lvl in spec.bids {
                b.add()?.orders(|o| {
                    for ord in lvl.orders {
                        o.add()?.order_id(ord.id.len())?;
                    }
                    Ok(())
                })?;
            }
            Ok(())
        })?
        .asks_ragged(spec.asks.len() as u16, |b| {
            for lvl in spec.asks {
                b.add()?.ask_orders(|o| {
                    for ord in lvl.orders {
                        o.add()?.order_id(ord.id.len())?;
                    }
                    Ok(())
                })?;
            }
            Ok(())
        })?
        .symbol(spec.symbol.len())?
        .encoded_length_with_header();

    let mut buf = vec![0u8; len];
    let written = v::L3BookEncoder::try_wrap_and_apply_header(&mut buf, 0)?
        .fixed(&v::L3BookFixedFields {
            timestamp: spec.timestamp,
            sequence: spec.sequence,
        })
        .bids(spec.bids.len() as u16, |g| {
            for lvl in spec.bids {
                g.add(|mut entry| {
                    entry.price(lvl.price).qty(lvl.qty);
                    entry.orders(lvl.orders.len() as u16, |o| {
                        for ord in lvl.orders {
                            o.add(|mut e| {
                                e.order_qty(ord.qty);
                                e.order_id(ord.id)
                            })?;
                        }
                        Ok(())
                    })
                })?;
            }
            Ok(())
        })?
        .asks(spec.asks.len() as u16, |g| {
            for lvl in spec.asks {
                g.add(|mut entry| {
                    entry.price(lvl.price).qty(lvl.qty);
                    entry.ask_orders(lvl.orders.len() as u16, |o| {
                        for ord in lvl.orders {
                            o.add(|mut e| {
                                e.order_qty(ord.qty);
                                e.order_id(ord.id)
                            })?;
                        }
                        Ok(())
                    })
                })?;
            }
            Ok(())
        })?
        .symbol(spec.symbol)?
        .encoded_length_with_header();
    assert_eq!(len, written, "{}: v0 EncodedLength disagreed", spec.label);
    Ok(buf)
}

pub fn wire_for(version: u16, spec: &BookSpec) -> Res<Vec<u8>> {
    match version {
        0 => encode_v0(spec),
        1 => encode_v1(spec),
        2 => encode_v2(spec),
        _ => encode_v3(spec),
    }
}
