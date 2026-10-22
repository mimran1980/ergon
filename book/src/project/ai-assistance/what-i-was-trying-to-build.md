# What I was trying to build

I have used SBE for more than a decade, including Java and Rust systems. The
project idea predates this AI experiment.

Over those years I repeatedly saw the same classes of problem:

- A new SBE user reads or writes positional data in the wrong order.
- Repeating groups and variable-length data make buffer sizing easy to get
  wrong.
- Java-style flyweights and parent hopping become awkward under Rust's borrow
  checker.
- A low-latency wire representation is valuable on a hot path, but it is
  unnecessarily unpleasant when application code simply wants a Boolean,
  decimal, owned DTO, or database record.
- A partially completed encoder can be mistaken for a complete message unless
  the API makes that misuse impossible.

My desired API followed from those experiences:

- wire order enforced at compile time;
- no final byte slice or encoded length until all required tail fields have
  been written;
- closure-based nested groups that work naturally with Rust borrowing;
- exact buffer sizing for messages containing groups and variable data;
- a fast flyweight path for latency-sensitive code;
- optional conversions and domain DTOs for code that does not need to operate
  directly on the wire representation; and
- official SBE wire compatibility as the non-negotiable contract.

Without coding agents, I estimated that the generator would take me roughly
four to six months. I had previously written code generation by hand in another
open-source project and knew how fiddly a large `syn`/`quote` implementation
could become. My original plan was to wait until a long garden-leave period and
build it manually.
