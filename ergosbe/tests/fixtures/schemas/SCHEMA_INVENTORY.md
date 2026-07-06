# Real-World SBE Schema Inventory

This document catalogs all real-world SBE schemas available for testing ErgoSBE.
It covers schemas from the `simple-binary-encoding/` submodule, exchange FTP sites,
and open-source GitHub repositories.

**Last updated:** 2026-07-06

**Author:** Research for todo 19

---

## 1. Schemas from `simple-binary-encoding/` submodule

The submodule (aeron-io/simple-binary-encoding) ships ~90+ XML files in its
test/benchmark/sample resources. The most significant production-grade schemas
are listed below. All are Apache 2.0 licensed.

### 1a. CME iLink Binary (`ilinkbinary.xml`)

**Source:** CME iLink3 binary order-entry protocol

Stats:
- **Size:** 286 KB, 1584 lines
- **Schema:** id=8, version=5, namespace `http://www.fixprotocol.org/ns/simple/1.0`
- **48 messages** -- session layer (Negotiate, Establish, Sequence, Terminate)
  and business messages (NewOrderSingle, ExecutionReports, MassQuote, etc.)
- **1045 fields, 33 groups** (all flat, single-level), 3 var-data fields
- **85 types, 50 enums, 27 composites, 1 set**
- **27** elements with `sinceVersion` (>0)
- **28** `presence="optional"` types, **31** `presence="constant"` types

Interesting patterns:
- Messages with 50+ fields (ExecutionReportTradeSpread526: 56 fields, 3 groups)
- Both session-level messages and business messages in one schema
- Version-gated fields (`sinceVersion` from 2 to 5) for incremental schema evolution
- Constant types with specific CME values (ClientFlowType, CrossType, HMACVersion)
- Optional types with CME-specific null sentinels (Int32NULL=2147483647,
  LocalMktDate=65535)
- Complex composites: messageHeader, groupSize, DecimalQty, Money, Permissions
- Groups shared across multiple messages (NoFills, NoOrderEvents, NoLegs)
- `deprecated` annotations on some messages and fields

### 1b. CME MDP / FIX Binary (`FixBinary.xml`)

**Source:** CME Market Data Platform (MDP 3.0) market-data messages

Stats:
- **Size:** 127 KB, 960 lines
- **Schema:** id=1, version=9 (dated 20180308),
  namespace `http://fixprotocol.io/2016/sbe`
- **29 messages** -- market data: InstrumentDefinition, IncrementalRefresh, Snapshot
- **547 fields, 53 groups** (all flat, single-level), 0 var-data
- **65 types, 15 enums, 0 composites, 3 sets**
- **46** `sinceVersion` fields, **14** optional, **16** constant

Interesting patterns:
- Namespace: root uses `ns2:` prefix; children (group, field, type, enum, set)
  are **unqualified** (no prefix). This is an important XML parsing edge case.
- Market-data semantics: binary entries, MBO/MBP book snapshots, incremental refresh
- Bitmap sets (MatchEventIndicator -- 8 Boolean indicators packed into uint8)
- Messages with up to 6 groups (MDInstrumentDefinitionOption41)
- Groups with `sinceVersion` on individual fields (e.g., MDUpdateAction sinceVersion=2
  inside NoMDEntries group)
- MDP 3.0 specific: `MDEntryType*` constant types encode market-data entry types
- `blockLength` encoded per message declaration

### 1c. FIX Benchmark Samples (`fix-message-samples.xml`)

**Size:** 21 KB, 327 lines

Stats:
- **1 message** (NewOrderSingle), **4 groups**, 0 var-data
- **23 types, 19 enums, 2 sets**
- 9 optional, 3 constant
- Nested groups (group-within-group pattern)

### 1d. SBE IR Meta-Schema (`sbe-ir.xml`)

**Size:** 7 KB, 101 lines

Self-describing IR schema (an SBE schema that describes the SBE intermediate
representation format). Useful for validating IR round-tripping.

Stats:
- 15 var-data fields, 4 enums, 1 optional field
- No groups or messages (by design -- it describes the IR structure as composites)

### 1e. Other Notable Test Schemas

| File | Size | Key Features |
|------|------|-------------|
| `code-generation-schema.xml` | 11 KB | 5 groups, 9 var-data, 5 enums, 1 set, nested groups |
| `code-generation-schema-with-version.xml` | 5 KB | Same + 22 sinceVersion fields, versioned types |
| `dto-test-schema.xml` | 12 KB | 5 groups, 10 var-data, 30 types, optional fields |
| `field-order-check-schema.xml` | 12 KB | 33 groups, 18 var-data, 16 sinceVersion, 9 optional |
| `example-schema.xml` | 5 KB | Classic Car example (3 groups, 4 var-data, nested groups) |
| `example-extension-schema.xml` | 6 KB | Car with extension, sinceVersion, XInclude |
| `since-version-filter-schema.xml` | 5 KB | 3 groups, 3 var-data, 25 sinceVersion, 17 types |
| `nested-group-schema.xml` | 1.5 KB | 3 groups with nesting (group within group) |
| `basic-variable-length-schema.xml` | 1.9 KB | Var-data fields |
| `versioned-message-v1/v2.xml` | 1.2/1.4 KB | Two schema versions for compatibility testing |
| `optional_enum_nullify.xml` | ~2 KB | Optional enum fields with null values |
| `constant-enum-fields.xml` | 2.5 KB | Constant values in enum/set context |
| `embedded-length-and-count-schema.xml` | 1.9 KB | Embedded dimension types |
| `composite-offsets-schema.xml` | ~2 KB | Composite field offset testing |
| `composite-elements-schema-rc4.xml` | 3 KB | Composite elements RC4 format |
| `basic-group-schema.xml` | 1.4 KB | Minimal group usage |
| `basic-schema-constant-header-field.xml` | 1 KB | Constant header field |
| `encoding-types-schema.xml` | 2.8 KB | 2 enums, 4 sets (encoding type testing) |
| `bigendian-test-schema.xml` | 4.8 KB | Big-endian byte order |
| `fix-message-samples.xml` | 21 KB | Benchmark FIX samples with groups/enums |
| `issue835.xml` | 22 KB | 65 types, 15 enums, 14 optional fields |

### 1f. In-Submodule Paths

These schemas are available (Apache 2.0) in the submodule:
```
simple-binary-encoding/sbe-tool/src/test/resources/
simple-binary-encoding/sbe-benchmarks/src/main/resources/
simple-binary-encoding/sbe-samples/src/main/resources/
simple-binary-encoding/gocode/resources/
```

---

## 2. Externally Sourced Schemas

### 2a. CME MDP 3.0 Latest (`cme_templates_FixBinary.xml`)

**Source:** CME Group public FTP
`ftp://ftp.cmegroup.com/SBEFix/Production/Templates/templates_FixBinary.xml`

Stats:
- **Size:** 147 KB, 1081 lines
- **Schema:** id=1, **version=13** (20230411) -- notably newer than submodule's version 9 (20180308)
- **31 messages** (vs 29 in v9 -- adds MDInstrumentDefinitionFixedIncome57,
  MDInstrumentDefinitionRepo58, SnapshotRefreshTopOrders59)
- **609 fields, 56 groups** (vs 547 fields, 53 groups)
- **72 types, 22 enums, 3 sets** (vs 65 types, 15 enums)
- **97** `sinceVersion` elements (all non-zero, vs 46)
- **14** optional, **14** constant

**Legal:** CME's SBE XML template files are publicly available via anonymous FTP
without login. Use for development/testing is standard practice across the
financial SBE ecosystem. No redistribution restrictions beyond CME's standard
market-data terms.

Key differences from submodule v9:
- +2 messages, +60 fields, +3 groups
- Significantly more version-gated features (97 vs 46)
- Many new types and enums added since 2018

### 2b. Binance Spot API (`binance_spot_3_5.xml`)

**Source:** https://github.com/binance/binance-spot-api-docs/blob/master/sbe/schemas/
`spot_3_5.xml` (latest production, schema id=3, version=5)

Stats:
- **Size:** 141 KB
- **92 messages** -- REST API responses (NewOrderResponse, ExchangeInfoResponse,
  DepthResponse, etc.) and WebSocket events (ExecutionReportEvent,
  BalanceUpdateEvent, etc.)
- **284** `presence="optional"` fields -- heavily uses optional fields with null values
- **81** elements with `sinceVersion`
- Uses XInclude for type definitions
- No groups or var-data fields
- Schema evolution via both `sinceVersion` on individual fields and
  schema-level `version` attribute

Interesting patterns:
- Reframes REST/WebSocket API as SBE message codes -- interesting architectural choice
- Ceiling-focused market-data types (PriceFilter, LotSizeFilter, MaxPositionFilter)
- `sinceVersion` used on entire messages and individual fields
- Heavy optional usage (284 occurrences) for REST fields that may be absent
- Messages at ids 1-21 are filter/rule definitions, 50-54 are websocket session,
  100-105 are REST metadata, 200-219 are market data, 300-317 are trading,
  400-405 are account, 500-505 are user data stream, 600-610 are events
- Per-field `deprecated` annotations
- XInclude-based multi-file composition

Available versions: `spot_3_5.xml` (latest), `spot_3_4.xml`, ... `spot_1_0.xml`,
plus FIX API variant `spot_fix_prod_latest.xml`.

### 2c. FIX Trading Community v2.0 RC3 Examples

**Source:** https://github.com/FIXTradingCommunity/fix-simple-binary-encoding/
Files: `v2-0-RC3/resources/xml/examples.xml` + `types-include.xml` +
`messages-include.xml`

Stats:
- **Size:** 6.3 KB + 540 B + 688 B (7.5 KB total)
- Uses **XInclude** to compose a single schema from multiple files:
  - `types-include.xml` defines composite type MONTH_YEAR
  - `messages-include.xml` defines BusinessMessageReject message
- **Namespace:** `http://fixprotocol.io/2017/sbe` (v2.0 RC3 style)
- Small but important for testing XInclude resolution

Available versions:
- `v1-0-STANDARD/resources/Examples.xml`
- `v2-0-RC1/resources/Examples.xml`
- `v2-0-RC2/resources/xml/examples.xml` + `*-include.xml`
- `v2-0-RC3/resources/xml/examples.xml` + `*-include.xml`

### 2d. EPAM CME MDP3 Handler (copy of CME templates)

**Source:** https://github.com/epam/java-cme-mdp3-handler

Contains copies of `templates_FixBinary.xml` (127 KB) at:
- `mbp-only/src/test/resources/templates_FixBinary.xml`
- `mbp-with-mbo/src/test/resources/templates_FixBinary.xml`

These are the same as the submodule's `FixBinary.xml` (CME v9, 2018).
The newer version (v13, 2023) is only on CME FTP.

### 2e. sambacha/CME-iLink3 Repo

**Source:** https://github.com/sambacha/CME-iLink3

Documentation and example schemas for CME iLink 3 order entry. Licensed MIT.
Reference only -- actual CME schemas are in the submodule's `ilinkbinary.xml`.

---

## 3. Exchanges Known to Use SBE (No Public XML Schema)

| Exchange | Usage | Schema Availability |
|----------|-------|-------------------|
| **Eurex (Deutsche Borse)** | T7 trading platform market data | Schemas distributed to members via Eurex FTP; public download of XML not available |
| **Euronext** | Optiq platform MDG | SBE template docs available but XML schemas in member-only portal |
| **ICE** | ICE Trading Architecture | Uses SBE for market data; schemas via member portal |
| **MOEX (Moscow Exchange)** | ASTS market data | Documents SBE; schemas via member FTP |
| **Bitget** | UTA API | Documents SBE API for spot/futures; schema XML not in public GitHub |
| **Bybit** | V5 API | Describes SBE support; no public XML schema repository |

These would require exchange membership agreements to access the actual XML schemas.

---

## 4. Recommended Test Fixtures for ErgoSBE

### Primary fixtures (highest priority)

1. **`car.xml`** (4 KB) -- Baseline schema. Small, well-understood.
   - Features: 3 groups, var-data, nested groups, enums, sets, constant fields

2. **`ilinkbinary.xml`** (286 KB) -- Most complex production schema available.
   - Features: 48 messages, 33 groups, 27 composites, 50 enums, versioning,
     optional fields, constant fields, 3 var-data

3. **`FixBinary.xml`** (127 KB) -- CME MDP 3.0 production market data.
   - Features: 29 messages, 53 groups, namespace prefix edge case, sinceVersion,
     optional fields, bitmap sets

4. **`binance_spot_3_5.xml`** (141 KB) -- REST API modeled as SBE.
   - Features: 92 messages, heavy optional usage (284), 81 sinceVersion,
     no groups (exercises flat message throughput)

5. **`cme_templates_FixBinary.xml`** (147 KB) -- Newer CME MDP (v13 vs v9).
   - Features: 97 sinceVersion fields, 56 groups, newer schema patterns

6. **`fix_examples_v2rc3.xml`** + includes (7.5 KB) -- XInclude multi-file schema.
   - Features: XInclude resolution, v2.0 RC3 namespace

7. **`fix-message-samples.xml`** (21 KB) -- Benchmark schema with groups and enums.

### Secondary fixtures (edge cases)

8. **`nested-group-schema.xml`** (1.5 KB) -- Group-within-group nesting
9. **`code-generation-schema-with-version.xml`** (5 KB) -- Versioned codegen
10. **`sbe-ir.xml`** (7 KB) -- Self-describing IR
11. **`versioned-message-v1.xml`** / `v2.xml` (1.2/1.4 KB) -- Version compatibility
12. **`new-order-single-schema.xml`** (3.3 KB) -- Minimal FIX message

---

## 5. Pattern Summary by Schema Feature

| Feature | ilinkbinary | FixBinary (v9) | CME FTP (v13) | Binance | FIX v2-RC3 |
|---------|-------------|----------------|---------------|---------|------------|
| Messages | 48 | 29 | 31 | 92 | 1 |
| Fields | 1045 | 547 | 609 | many | few |
| Groups | 33 (flat) | 53 (flat) | 56 (flat) | 0 | 0 |
| Var-data | 3 | 0 | 0 | 0 | 0 |
| Composites | 27 | 0 | 0 | ~5 | 1 |
| Enums | 50 | 15 | 22 | ~5 | 0 |
| Sets | 1 | 3 | 3 | 0 | 0 |
| sinceVersion | 27 | 46 | 97 | 81 | 0 |
| Optional | 28 | 14 | 14 | 284 | 0 |
| Constant | 31 | 16 | 14 | 0 | 0 |
| XInclude | No | No | No | Yes | Yes |
| Nested groups | No | No | No | No | No |
| Deprecated | Yes | No | No | Yes | No |
| Big-endian | No | No | No | No | No |

---

## 6. File Locations

### In this repository (copied to fixtures)

All schemas are available at `ergosbe/tests/fixtures/schemas/`:

```
ergosbe/tests/fixtures/schemas/
  car.xml                              (4.0 KB)   -- Classic Car example
  FixBinary.xml                        (127 KB)   -- CME MDP v9 (submodule)
  fix-message-samples.xml              (21 KB)    -- FIX benchmark samples
  ilinkbinary.xml                      (286 KB)   -- CME iLink3
  nested-group-schema.xml              (1.5 KB)   -- Group nesting test
  new-order-single-schema.xml          (3.3 KB)   -- FIX NewOrderSingle
  sbe-ir.xml                           (7.2 KB)   -- SBE IR meta-schema
  binance_spot_3_5.xml                 (141 KB)   -- Binance Spot API (external)
  cme_templates_FixBinary.xml          (147 KB)   -- CME MDP v13 FTP (external)
  fix_examples_v2rc3.xml               (6.3 KB)   -- FIX v2.0 RC3 (external)
  fix_types_include.xml                (540 B)    -- FIX v2.0 RC3 types include
  fix_messages_include.xml             (688 B)    -- FIX v2.0 RC3 messages include
```

### Via submodule (Apache 2.0)

The `simple-binary-encoding/` submodule (on `first_cut` branch) contains all
test schemas under `sbe-tool/src/test/resources/`.

### Download URLs for external schemas

- CME FTP (latest templates): `ftp://ftp.cmegroup.com/SBEFix/Production/Templates/templates_FixBinary.xml`
- Binance latest: `https://raw.githubusercontent.com/binance/binance-spot-api-docs/master/sbe/schemas/spot_3_5.xml`
- FIX Trading Community: `https://github.com/FIXTradingCommunity/fix-simple-binary-encoding/`

---

## 7. Schema Feature Checklist for ErgoSBE

This section maps schema features found across all discovered schemas to
ErgoSBE implementation status. Use this to identify gaps.

| Feature | Found In | ErgoSBE Status |
|---------|----------|----------------|
| Basic scalar fields | All schemas | Implemented |
| Enums | ilinkbinary, FixBinary, Binance | Works |
| Sets/bitmaps | FixBinary, ilinkbinary, car | Works |
| Composites | ilinkbinary, car | Works |
| Groups (flat) | ilinkbinary, FixBinary, car | Works |
| Nested groups | car, nested-group-schema, field-order-check | Works |
| Var-data (`<data>`) | car, ilinkbinary, various test schemas | Works |
| `sinceVersion` on fields | ilinkbinary, FixBinary, Binance | Works |
| `sinceVersion` on messages | ilinkbinary, Binance | Works |
| `sinceVersion` on groups | ilinkbinary | Works |
| `presence="optional"` | ilinkbinary, FixBinary, Binance | Works |
| `presence="constant"` | ilinkbinary, FixBinary, car | Works |
| Constant enums | All schemas with enums | Works |
| XInclude | Binance, FIX examples, example-schema | Implemented |
| Namespace prefixes | FixBinary (ns2:message), ilinkbinary | TBD |
| Big-endian byte order | example-bigendian-test-schema | TBD |
| Custom header type | custom-header-type.xml | TBD |
| Custom dimension type | ilinkbinary, various | TBD |
| Deprecated fields | ilinkbinary, Binance, since-deprecated-test | TBD |
| Extension schemas | example-extension-schema, extension-schema | TBD |
| Zero-field messages | Binance (many filter messages) | TBD |
| Composites with sinceVersion | ilinkbinary | TBD |
| Individual field with offset | All messages | Works |
| Embedded dimension type | embedded-length-and-count-schema | TBD |
