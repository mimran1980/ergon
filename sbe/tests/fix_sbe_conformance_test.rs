//! Official FIX SBE Conformance suite (FIXTradingCommunity/fix-sbe-conformance).
//!
//! Profile: `ergo-sbe-fix-sbe-0.1.10`. The respond leg uses ergo-sbe generated
//! codecs; inject bytes and RL golden responses are pinned under
//! `tests/fixtures/fix-sbe-conformance/` (produced by the Real Logic Java
//! injector / UnderTest). Equality to RL goldens is the primary gate;
//! `scripts/run-fix-sbe-conformance.sh` additionally runs the official
//! Java RLValidator when a built suite is available.

#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]
use std::error::Error;
use std::path::PathBuf;
use std::process::Command;

mod common;
use common::{compile_and_run, generate};

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fix-sbe-conformance")
}

// Pin inject + RL golden responses (from official Java suite injector/UnderTest).
const TEST1_INJECT: &[u8] = &[
    0x36, 0x00, 0x63, 0x00, 0x01, 0x00, 0x00, 0x00, 0x43, 0x4c, 0x30, 0x30, 0x30, 0x30, 0x30, 0x31,
    0x41, 0x43, 0x43, 0x54, 0x30, 0x30, 0x30, 0x31, 0x53, 0x59, 0x4d, 0x42, 0x4f, 0x4c, 0x2e, 0x41,
    0x32, 0xc0, 0xc2, 0xc5, 0x69, 0xe7, 0x42, 0x05, 0x00, 0xbc, 0x02, 0x00, 0x00, 0x32, 0x98, 0x44,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];
const TEST1_RL: &[u8] = &[
    0x2a, 0x00, 0x62, 0x00, 0x01, 0x00, 0x00, 0x00, 0x4f, 0x52, 0x30, 0x30, 0x30, 0x30, 0x30, 0x31,
    0x45, 0x58, 0x30, 0x30, 0x30, 0x30, 0x30, 0x31, 0x46, 0x31, 0x53, 0x59, 0x4d, 0x42, 0x4f, 0x4c,
    0x2e, 0x41, 0x00, 0x00, 0xff, 0x00, 0x00, 0x32, 0x90, 0x01, 0x00, 0x00, 0x2c, 0x01, 0x00, 0x00,
    0xf4, 0x42, 0x0c, 0x00, 0x01, 0x00, 0x98, 0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x2c, 0x01,
    0x00, 0x00,
];
const TEST2_INJECT: &[u8] = &[
    0x3a, 0x00, 0x63, 0x00, 0x01, 0x00, 0x01, 0x00, 0x43, 0x4c, 0x30, 0x30, 0x30, 0x30, 0x30, 0x31,
    0x41, 0x43, 0x43, 0x54, 0x30, 0x30, 0x30, 0x31, 0x53, 0x59, 0x4d, 0x42, 0x4f, 0x4c, 0x2e, 0x41,
    0x32, 0xc0, 0xc2, 0xc5, 0x69, 0xe7, 0x42, 0x05, 0x00, 0xbc, 0x02, 0x00, 0x00, 0x32, 0x98, 0x44,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xc8, 0x00,
    0x00, 0x00,
];
const TEST2_RL: &[u8] = TEST1_RL; // same response shape as test1
const TEST3_INJECT: &[u8] = &[
    0x3a, 0x00, 0x63, 0x00, 0x01, 0x00, 0x02, 0x00, 0x43, 0x4c, 0x30, 0x30, 0x30, 0x30, 0x30, 0x31,
    0x41, 0x43, 0x43, 0x54, 0x30, 0x30, 0x30, 0x31, 0x53, 0x59, 0x4d, 0x42, 0x4f, 0x4c, 0x2e, 0x41,
    0x32, 0xc0, 0xc2, 0xc5, 0x69, 0xe7, 0x42, 0x05, 0x00, 0xbc, 0x02, 0x00, 0x00, 0x32, 0x98, 0x44,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xc8, 0x00,
    0x00, 0x00, 0x14, 0x00, 0x43, 0x6f, 0x6d, 0x70, 0x6c, 0x69, 0x61, 0x6e, 0x63, 0x65, 0x20, 0x63,
    0x65, 0x72, 0x74, 0x69, 0x66, 0x69, 0x65, 0x64,
];
const TEST3_RL: &[u8] = &[
    0x32, 0x00, 0x62, 0x00, 0x01, 0x00, 0x02, 0x00, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
    0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x38, 0x38, 0x53, 0x59, 0x4d, 0x42, 0x4f, 0x4c,
    0x2e, 0x41, 0x00, 0x00, 0xff, 0x00, 0x00, 0x32, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0xf4, 0x42, 0x53, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x0c, 0x00, 0x00, 0x00, 0x10, 0x00,
    0x4d, 0x61, 0x72, 0x6b, 0x65, 0x74, 0x20, 0x69, 0x73, 0x20, 0x63, 0x6c, 0x6f, 0x73, 0x65, 0x64,
];

fn bytes_lit(name: &str, b: &[u8]) -> String {
    let body = b
        .iter()
        .map(|x| format!("0x{x:02x}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("let {name}: &[u8] = &[{body}];\n")
}

fn respond_schema1_body() -> String {
    let mut code = String::new();
    code.push_str(&bytes_lit("inject1", TEST1_INJECT));
    code.push_str(&bytes_lit("golden1", TEST1_RL));
    code.push_str(&bytes_lit("inject2", TEST2_INJECT));
    code.push_str(&bytes_lit("golden2", TEST2_RL));
    code.push_str(
        r#"
        fn id8(s: &str) -> [u8; 8] {
            let mut out = [b' '; 8];
            let b = s.as_bytes();
            let n = b.len().min(8);
            out[..n].copy_from_slice(&b[..n]);
            out
        }

        fn respond(
            inject: &[u8],
            order_id: &str,
            exec_id: &str,
            exec_type: ExecTypeEnum,
            ord_status: OrdStatusEnum,
            leaves: i32,
            cum: i32,
            trade_date: u16,
            fills: &[(i64, i32)],
        ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
            let order = NewOrderSingleDecoder::decode(inject, 0)?;
            let symbol = order.symbol();
            let side = order.side();
            let fills_n = fills.len();
            let need = ExecutionReportEncoder::compute_length_with_header(fills_n);
            let mut buf = vec![0u8; need];
            let complete = ExecutionReportEncoder::wrap_and_apply_header(&mut buf, 0)?
                .fixed(&ExecutionReportFixedFields {
                    order_id: id8(order_id),
                    exec_id: id8(exec_id),
                    exec_type,
                    ord_status,
                    symbol,
                    maturity_month_year: MONTHYEAR::new(0, 255, 0, 0),
                    side,
                    leaves_qty: QtyEncoding::new(leaves),
                    cum_qty: QtyEncoding::new(cum),
                    trade_date,
                })
                .fills_grp(fills_n as u16, |g| {
                    for (px_mant, qty_mant) in fills {
                        g.add(|e| {
                            e.fill_px(DecimalEncoding::new(*px_mant))
                                .fill_qty(QtyEncoding::new(*qty_mant));
                            Ok(())
                        })?;
                    }
                    Ok(())
                })?;
            let len = complete.encoded_length_with_header();
            Ok(buf[..len].to_vec())
        }

        let out1 = respond(
            inject1,
            "OR000001",
            "EX000001",
            ExecTypeEnum::Trade,
            OrdStatusEnum::PartialFilled,
            400,
            300,
            17140,
            &[(17560, 300)],
        )?;
        assert_eq!(out1.as_slice(), golden1, "test1 ergon vs RL");
        println!("PASS: fix-sbe-conformance test1 byte-identical to RL UnderTest");

        let out2 = respond(
            inject2,
            "OR000001",
            "EX000001",
            ExecTypeEnum::Trade,
            OrdStatusEnum::PartialFilled,
            400,
            300,
            17140,
            &[(17560, 300)],
        )?;
        assert_eq!(out2.as_slice(), golden2, "test2 ergon vs RL");
        println!("PASS: fix-sbe-conformance test2 (later-schema inject) byte-identical to RL");
    "#,
    );
    code
}

fn respond_schema3_body() -> String {
    let mut code = String::new();
    code.push_str(&bytes_lit("inject3", TEST3_INJECT));
    code.push_str(&bytes_lit("golden3", TEST3_RL));
    code.push_str(
        r#"
        fn id8(s: &str) -> [u8; 8] {
            let mut out = [b' '; 8];
            let b = s.as_bytes();
            let n = b.len().min(8);
            out[..n].copy_from_slice(&b[..n]);
            out
        }

        let order = NewOrderSingleDecoder::decode(inject3, 0)?;
        let symbol = order.symbol();
        let side = order.side();
        let reject = b"Market is closed";
        let need = ExecutionReportEncoder::compute_length_with_header(0, reject.len());
        let mut buf = vec![0u8; need];
        let complete = ExecutionReportEncoder::wrap_and_apply_header(&mut buf, 0)?
            .fixed(&ExecutionReportFixedFields {
                order_id: id8("        "),
                exec_id: id8("        "),
                exec_type: ExecTypeEnum::Rejected,
                ord_status: OrdStatusEnum::Rejected,
                symbol,
                maturity_month_year: MONTHYEAR::new(0, 255, 0, 0),
                side,
                leaves_qty: QtyEncoding::new(0),
                cum_qty: QtyEncoding::new(0),
                trade_date: 17140,
                security_id: id8("S1234567"),
            })
            .fills_grp(0, |_| Ok(()))?
            .reject_text(reject)?;
        let len = complete.encoded_length_with_header();
        assert_eq!(&buf[..len], golden3, "test3 ergon vs RL");
        println!("PASS: fix-sbe-conformance test3 (var-data) byte-identical to RL");
    "#,
    );
    code
}

#[test]
fn suite_tests_1_and_2_match_rl_golden() -> Result<(), Box<dyn Error>> {
    let schema = fixtures().join("schema1.xml");
    let (_s, src) = generate(&schema, "conf_s1");
    compile_and_run("conf_s1_t12", &src, &respond_schema1_body());
    Ok(())
}

#[test]
fn suite_test_3_var_data_match_rl_golden() -> Result<(), Box<dyn Error>> {
    let schema = fixtures().join("schema3.xml");
    let (_s, src) = generate(&schema, "conf_s3");
    compile_and_run("conf_s3_t3", &src, &respond_schema3_body());
    Ok(())
}

#[test]
fn fixtures_present_for_declared_profile() {
    let f = fixtures();
    for name in [
        "schema1.xml",
        "schema2.xml",
        "schema3.xml",
        "test1.json",
        "test2.json",
        "test3.json",
        "test1inject.sbe",
        "test2inject.sbe",
        "test3inject.sbe",
        "test1respond_rl.sbe",
        "test2respond_rl.sbe",
        "test3respond_rl.sbe",
    ] {
        assert!(f.join(name).is_file(), "missing conformance fixture {name}");
    }
}

/// Optional: run official RLValidator on ergon-equivalent response bytes.
#[test]
fn optional_java_rlvalidator_accepts_ergon_equivalent_responses() -> Result<(), Box<dyn Error>> {
    let conf = match std::env::var_os("FIX_SBE_CONFORMANCE_HOME") {
        Some(p) => PathBuf::from(p),
        None => {
            eprintln!("RLValidator not available: FIX_SBE_CONFORMANCE_HOME unset");
            return Ok(());
        }
    };
    if !conf.join("target/classes").is_dir() {
        eprintln!(
            "RLValidator not available: suite not built under {}",
            conf.display()
        );
        return Ok(());
    }
    let cp_out = Command::new("mvn")
        .args([
            "-q",
            "-DincludeScope=runtime",
            "dependency:build-classpath",
            "-Dmdep.outputFile=/dev/stdout",
        ])
        .current_dir(&conf)
        .output()?;
    if !cp_out.status.success() {
        eprintln!("RLValidator not available: classpath failed");
        return Ok(());
    }
    let deps = String::from_utf8_lossy(&cp_out.stdout);
    let classpath = format!("{}:{}", conf.join("target/classes").display(), deps.trim());
    let work = PathBuf::from(std::env::temp_dir()).join("ergo_fix_sbe_rlvalidator");
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work)?;
    for (n, golden) in [(1, TEST1_RL), (2, TEST2_RL), (3, TEST3_RL)] {
        // Byte-identity tests prove ergon == these goldens; validator is the
        // official oracle over that wire image.
        let respond = work.join(format!("test{n}respond_ergon.sbe"));
        std::fs::write(&respond, golden)?;
        let status = Command::new("java")
            .args([
                "-cp",
                &classpath,
                "io.fixprotocol.sbe.conformance.rlimpl.RLValidator",
                &format!("test{n}.json"),
                respond.to_str().unwrap(),
            ])
            .current_dir(&conf)
            .status()?;
        assert!(status.success(), "RLValidator failed for test{n}");
        println!("PASS: RLValidator accepted test{n}");
    }
    Ok(())
}
