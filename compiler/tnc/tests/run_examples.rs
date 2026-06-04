//! Integration tests: run every example through the real `tnc` binary and
//! assert its output (or that it is correctly rejected). This is the regression
//! safety net — no more catching bugs by hand.

use std::path::PathBuf;
use std::process::Command;

fn tnc() -> &'static str {
    env!("CARGO_BIN_EXE_tnc")
}

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
}

/// Run `tnc run <example>`, assert success, return normalized stdout.
fn run_ok(name: &str) -> String {
    let out = Command::new(tnc())
        .arg("run")
        .arg(example(name))
        .output()
        .expect("failed to spawn tnc");
    assert!(
        out.status.success(),
        "expected `{}` to run successfully.\nstderr: {}",
        name,
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .expect("stdout was not UTF-8")
        .replace("\r\n", "\n")
}

/// Run `tnc run <example>`, assert it FAILS, return normalized stderr.
fn run_fail(name: &str) -> String {
    let out = Command::new(tnc())
        .arg("run")
        .arg(example(name))
        .output()
        .expect("failed to spawn tnc");
    assert!(
        !out.status.success(),
        "expected `{}` to be rejected, but it succeeded",
        name
    );
    String::from_utf8_lossy(&out.stderr).replace("\r\n", "\n")
}

// ---- good programs: exact output (English + Arabic) ----

#[test]
fn adult_en() {
    assert_eq!(
        run_ok("adult_en.tn"),
        "Adult\nsum 1..5 = 15\nfactorial(5) = 120\n"
    );
}

#[test]
fn adult_ar() {
    assert_eq!(
        run_ok("adult_ar.tn"),
        "بالغ\nالمجموع ١..٥ = 15\nالمضروب(٥) = 120\n"
    );
}

#[test]
fn shapes_en() {
    assert_eq!(
        run_ok("shapes_en.tn"),
        "point: 3 4\narea circle: 75\narea rect: 12\narea dot: 0\nclassify 0: zero\nclassify 7: many\n"
    );
}

#[test]
fn shapes_ar() {
    assert_eq!(
        run_ok("shapes_ar.tn"),
        "النقطة: 3 4\nمساحة الدائرة: 75\nمساحة المستطيل: 12\nمساحة المفرد: 0\nصنف ٠: صفر\nصنف ٧: كثير\n"
    );
}

#[test]
fn points_en() {
    assert_eq!(run_ok("points_en.tn"), "p.sum: 7\nq.x: 13 q.sum: 17\n");
}

#[test]
fn points_ar() {
    assert_eq!(run_ok("points_ar.tn"), "ن.مجموع: 7\nم.س: 13 م.مجموع: 17\n");
}

#[test]
fn mixed() {
    assert_eq!(run_ok("mixed.tn"), "mixed works\n");
}

#[test]
fn interp_en() {
    assert_eq!(
        run_ok("interp_en.tn"),
        "Hello, Sara — you are 20\nnext year you will be 21\na literal brace: { and }\n"
    );
}

#[test]
fn interp_ar() {
    assert_eq!(
        run_ok("interp_ar.tn"),
        "مرحبا، سارة — عمرك 20\nفي السنة القادمة ستكون 21\n"
    );
}

#[test]
fn polyglot() {
    assert_eq!(
        run_ok("polyglot.tn"),
        "balance: 150\nstatus: active\nlabel: rich\n"
    );
}

// ---- bad programs: must be rejected by the type checker ----

#[test]
fn bad_type_is_rejected() {
    assert!(run_fail("bad_type.tn").contains("expected `bool`"));
}

#[test]
fn bad_exhaustive_is_rejected() {
    assert!(run_fail("bad_exhaustive.tn").contains("non-exhaustive"));
}

// ---- security regression: Trojan Source / bidi control characters ----

#[test]
fn trojan_source_is_rejected() {
    let path = std::env::temp_dir().join("tn619_trojan_regression.tn");
    // A U+202E (Right-to-Left Override) hidden inside a comment.
    std::fs::write(&path, "fn main() {\n    // h\u{202E}idden\n    print(1)\n}").unwrap();
    let out = Command::new(tnc())
        .arg("run")
        .arg(&path)
        .output()
        .expect("spawn");
    let _ = std::fs::remove_file(&path);
    assert!(
        !out.status.success(),
        "Trojan-Source file should be rejected"
    );
    assert!(String::from_utf8_lossy(&out.stderr).contains("bidirectional"));
}
