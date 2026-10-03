//! Deterministic price/time priority matching for the simulated venue.
use std::collections::BTreeMap;

/// A limit order, using Decimal9 mantissas.
#[derive(Clone, Copy, Debug)]
pub struct Order {
    /// Caller-assigned unique id.
    pub id: u64,
    /// Buy when true, sell otherwise.
    pub buy: bool,
    /// Limit price.
    pub price: i64,
    /// Submitted quantity.
    pub qty: i64,
}

/// A trade at the resting order's price.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fill {
    /// Aggressing order.
    pub taker: u64,
    /// Resting order.
    pub maker: u64,
    /// Executed price.
    pub price: i64,
    /// Executed quantity on each side.
    pub qty: i64,
}

/// Per-order quantity conservation ledger.
#[derive(Clone, Copy, Debug)]
pub struct Account {
    /// Original order.
    pub order: Order,
    /// Quantity already traded.
    pub filled: i64,
    /// Quantity still resting.
    pub resting: i64,
    /// Quantity cancelled.
    pub cancelled: i64,
    seq: u64,
}

/// A simulation book; maps are deterministic and ids are point lookups.
#[derive(Debug, Default)]
pub struct MatchingBook {
    accounts: BTreeMap<u64, Account>,
    seq: u64,
}

impl MatchingBook {
    /// Submit an order, appending its trades to `fills`. Returns false for
    /// invalid quantities/prices or duplicate ids, without changing the book.
    pub fn submit(&mut self, order: Order, fills: &mut Vec<Fill>) -> bool {
        if order.price <= 0 || order.qty <= 0 || self.accounts.contains_key(&order.id) {
            return false;
        }
        let seq = self.seq;
        self.seq += 1;
        let mut remaining = order.qty;
        while remaining > 0 {
            let maker = self
                .accounts
                .values()
                .filter(|a| a.resting > 0 && a.order.buy != order.buy)
                .filter(|a| {
                    if order.buy {
                        a.order.price <= order.price
                    } else {
                        a.order.price >= order.price
                    }
                })
                .min_by_key(|a| {
                    (
                        if order.buy {
                            a.order.price
                        } else {
                            -a.order.price
                        },
                        a.seq,
                    )
                })
                .map(|a| a.order.id);
            let Some(maker) = maker else {
                break;
            };
            let Some(account) = self.accounts.get_mut(&maker) else {
                break;
            };
            let qty = account.resting.min(remaining);
            account.resting -= qty;
            account.filled += qty;
            remaining -= qty;
            fills.push(Fill {
                taker: order.id,
                maker,
                price: account.order.price,
                qty,
            });
        }
        self.accounts.insert(
            order.id,
            Account {
                order,
                filled: order.qty - remaining,
                resting: remaining,
                cancelled: 0,
                seq,
            },
        );
        true
    }

    /// Cancel the remaining quantity. Already terminal/unknown ids return 0.
    pub fn cancel(&mut self, id: u64) -> i64 {
        let Some(account) = self.accounts.get_mut(&id) else {
            return 0;
        };
        let qty = account.resting;
        account.resting = 0;
        account.cancelled += qty;
        qty
    }

    /// An order's current conservation ledger.
    #[must_use]
    pub fn account(&self, id: u64) -> Option<&Account> {
        self.accounts.get(&id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn trades_at_resting_price_and_cancels_remainder() {
        let mut book = MatchingBook::default();
        let mut fills = Vec::new();
        assert!(book.submit(
            Order {
                id: 1,
                buy: false,
                price: 100,
                qty: 3
            },
            &mut fills
        ));
        assert!(book.submit(
            Order {
                id: 2,
                buy: true,
                price: 105,
                qty: 5
            },
            &mut fills
        ));
        assert_eq!(
            fills,
            [Fill {
                maker: 1,
                taker: 2,
                price: 100,
                qty: 3
            }]
        );
        assert_eq!(book.cancel(2), 2);
        assert_eq!(book.cancel(2), 0);
    }

    proptest! {
        #[test]
        fn matching_conserves_quantity_and_never_crosses(orders in prop::collection::vec((any::<bool>(), 1_i64..1000, 1_i64..100, any::<bool>()), 0..200)) {
            let mut book = MatchingBook::default();
            let mut fills = Vec::new();
            for (idx, (buy, price, qty, cancel)) in orders.into_iter().enumerate() {
                let id = idx as u64;
                prop_assert!(book.submit(Order { id, buy, price, qty }, &mut fills), "valid order rejected");
                if cancel { book.cancel(id); }
                for a in book.accounts.values() {
                    prop_assert_eq!(a.filled + a.resting + a.cancelled, a.order.qty);
                }
                let bid = book.accounts.values().filter(|a| a.order.buy && a.resting > 0).map(|a| a.order.price).max();
                let ask = book.accounts.values().filter(|a| !a.order.buy && a.resting > 0).map(|a| a.order.price).min();
                if let (Some(bid), Some(ask)) = (bid, ask) { prop_assert!(bid < ask); }
                for fill in &fills {
                    for id in [fill.maker, fill.taker] {
                        let o = book.accounts[&id].order;
                        prop_assert!(if o.buy { fill.price <= o.price } else { fill.price >= o.price }, "limit violated");
                    }
                }
            }
        }
    }
}
