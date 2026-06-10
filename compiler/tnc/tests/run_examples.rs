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

// ---- good programs: exact output (English + Arabic + French) ----

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
fn adult_fr() {
    assert_eq!(
        run_ok("adult_fr.tn"),
        "Adulte\nsomme 1..5 = 15\nfactorielle(5) = 120\n"
    );
}

#[test]
fn points_fr() {
    assert_eq!(run_ok("points_fr.tn"), "p.somme: 7\nq.x: 13 q.somme: 17\n");
}

#[test]
fn shapes_fr() {
    assert_eq!(
        run_ok("shapes_fr.tn"),
        "point: 3 4\naire cercle: 75\naire rect: 12\naire rien: 0\nclasser 0: zéro\nclasser 7: plusieurs\n"
    );
}

#[test]
fn trilingue() {
    assert_eq!(
        run_ok("trilingue.tn"),
        "solde: 150\nالمالك: Sara\nlabel: riche\nverdict: ça va\n"
    );
}

#[test]
fn arrays_en() {
    assert_eq!(
        run_ok("arrays_en.tn"),
        "first: 10\nlen: 3\nsum: 60\nafter push: [1, 2, 3]\npopped: 3 now: [1, 2]\ngrades: [A, B+]\narrays equal: true\n"
    );
}

#[test]
fn arrays_ar() {
    assert_eq!(
        run_ok("arrays_ar.tn"),
        "الأول: 10\nالطول: 3\nالمجموع: 60\nبعد الإضافة: [1, 2, 3]\nالمسحوب: 3 الآن: [1, 2]\n"
    );
}

#[test]
fn arrays_fr() {
    assert_eq!(
        run_ok("arrays_fr.tn"),
        "premier: 10\nlongueur: 3\nsomme: 60\naprès ajout: [1, 2, 3]\nretiré: 3 maintenant: [1, 2]\n"
    );
}

#[test]
fn result_en() {
    assert_eq!(
        run_ok("result_en.tn"),
        "ok: 5\nerror: division by zero\nok: 3\nerror: division by zero\n"
    );
}

#[test]
fn result_fr() {
    assert_eq!(
        run_ok("result_fr.tn"),
        "ok: 5\nerreur: division par zéro\nok: 3\nerreur: division par zéro\n"
    );
}

#[test]
fn result_ar() {
    assert_eq!(
        run_ok("result_ar.tn"),
        "نجاح: 5\nفشل: قسمة على صفر\nنجاح: 3\nفشل: قسمة على صفر\n"
    );
}

#[test]
fn counter_mut() {
    assert_eq!(
        run_ok("counter_mut.tn"),
        "count: 12\nlog: [after twelve]\nset directly: 100\nxs: [9, 2, 3]\n"
    );
}

#[test]
fn ownership() {
    assert_eq!(
        run_ok("ownership.tn"),
        "owned: hello\ncopy still valid: hello\nfirst / second\ncloned twice: hello\n"
    );
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
fn floats_en() {
    assert_eq!(
        run_ok("floats_en.tn"),
        "area: 12.56636\n7/2 as float: 3.5\n3.9 as int: 3\nis 2.5 > 2.0? true\n"
    );
}

#[test]
fn floats_ar() {
    assert_eq!(run_ok("floats_ar.tn"), "المساحة: 12.56636\n٧/٢ عشري: 3.5\n");
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

/// Run an inline source string through `tnc run` via a temp file.
fn run_src(name: &str, src: &str) -> std::process::Output {
    let path = std::env::temp_dir().join(name);
    std::fs::write(&path, src).unwrap();
    let out = Command::new(tnc())
        .arg("run")
        .arg(&path)
        .output()
        .expect("spawn");
    let _ = std::fs::remove_file(&path);
    out
}

#[test]
fn array_type_errors_are_rejected() {
    let het = run_src("tn619_het.tn", "fn main() { let a = [1, \"x\"] }");
    assert!(!het.status.success());
    assert!(String::from_utf8_lossy(&het.stderr).contains("array element"));

    let imm = run_src("tn619_imm.tn", "fn main() { let a = [1] a.push(2) }");
    assert!(!imm.status.success());
    assert!(String::from_utf8_lossy(&imm.stderr).contains("immutable"));

    let empty = run_src("tn619_empty.tn", "fn main() { let a = [] }");
    assert!(!empty.status.success());
    assert!(String::from_utf8_lossy(&empty.stderr).contains("annotate"));
}

#[test]
fn array_bounds_checked_at_runtime() {
    let oob = run_src("tn619_oob.tn", "fn main() { let a = [1] print(a[5]) }");
    assert!(!oob.status.success());
    assert!(String::from_utf8_lossy(&oob.stderr).contains("out of bounds"));
}

#[test]
fn result_misuse_is_rejected() {
    let q = run_src(
        "tn619_q.tn",
        "fn f() -> Result<int, str> { Ok(1) }\nfn main() { let x = f()? }",
    );
    assert!(!q.status.success());
    assert!(String::from_utf8_lossy(&q.stderr).contains("function returning `Result`"));

    let nx = run_src(
        "tn619_nx.tn",
        "fn f() -> Result<int, str> { Ok(1) }\nfn main() { let s = match f() { Ok(v) => 1 } }",
    );
    assert!(!nx.status.success());
    assert!(String::from_utf8_lossy(&nx.stderr).contains("missing variant(s) Err"));
}

#[test]
fn mut_self_misuse_is_rejected() {
    // &mut self method on an immutable binding: compile error.
    let imm = run_src(
        "tn619_mself.tn",
        "struct C { n: int }\nimpl C { fn bump(&mut self) { self.n = self.n + 1 } }\nfn main() { let c = C { n: 0 } c.bump() }",
    );
    assert!(!imm.status.success());
    assert!(String::from_utf8_lossy(&imm.stderr).contains("immutable binding `c`"));

    // Mutating self inside a &self method: compile error.
    let ro = run_src(
        "tn619_roself.tn",
        "struct C { n: int }\nimpl C { fn get(&self) -> int { self.n = 5 self.n } }\nfn main() { }",
    );
    assert!(!ro.status.success());
    assert!(String::from_utf8_lossy(&ro.stderr).contains("&mut self"));
}

// ---- ownership: use-after-move is a compile error ----

#[test]
fn use_after_move_is_rejected() {
    let basic = run_src(
        "tn619_uam.tn",
        "fn main() { let s = \"hi\" let t = s print(s) }",
    );
    assert!(!basic.status.success());
    assert!(String::from_utf8_lossy(&basic.stderr).contains("use of moved value `s`"));

    let in_loop = run_src(
        "tn619_uam_loop.tn",
        "fn eat(s: str) { print(s) }\nfn main() { let s = \"hi\" for i in 0..3 { eat(s) } }",
    );
    assert!(!in_loop.status.success());
    assert!(String::from_utf8_lossy(&in_loop.stderr).contains("moved inside a loop"));

    let in_branch = run_src(
        "tn619_uam_br.tn",
        "fn eat(s: str) { print(s) }\nfn main() { let s = \"hi\" if true { eat(s) } else { print(\"n\") } eat(s) }",
    );
    assert!(!in_branch.status.success());
    assert!(String::from_utf8_lossy(&in_branch.stderr).contains("use of moved value `s`"));

    // .clone() makes the same program legal.
    let cloned = run_src(
        "tn619_clone_ok.tn",
        "fn eat(s: str) { print(s) }\nfn main() { let s = \"hi\" let t = s.clone() eat(t) eat(s) }",
    );
    assert!(
        cloned.status.success(),
        "clone should satisfy the move checker"
    );
}

// ---- native backend (Cranelift JIT) ----

#[test]
fn jit_matches_interpreter() {
    let jit = Command::new(tnc())
        .arg("jit")
        .arg(example("jit_fib.tn"))
        .output()
        .expect("spawn");
    assert!(
        jit.status.success(),
        "jit failed: {}",
        String::from_utf8_lossy(&jit.stderr)
    );
    let interp = Command::new(tnc())
        .arg("run")
        .arg(example("jit_fib.tn"))
        .output()
        .expect("spawn");
    assert!(interp.status.success());
    assert_eq!(
        String::from_utf8_lossy(&jit.stdout).replace("\r\n", "\n"),
        String::from_utf8_lossy(&interp.stdout).replace("\r\n", "\n"),
        "native code and interpreter must agree"
    );
    assert_eq!(
        String::from_utf8_lossy(&jit.stdout).replace("\r\n", "\n"),
        "6765\n5050\n3628800\n0 0\n1 1\n2 4\n"
    );
}

#[test]
fn jit_rejects_unsupported_constructs() {
    let path = std::env::temp_dir().join("tn619_jit_unsup.tn");
    std::fs::write(&path, "fn main() { let s = \"hello\" print(s) }").unwrap();
    let out = Command::new(tnc())
        .arg("jit")
        .arg(&path)
        .output()
        .expect("spawn");
    let _ = std::fs::remove_file(&path);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("tnc run"));
}

// ---- robustness: UTF-8 BOM (Windows editors / PowerShell prepend one) ----

#[test]
fn utf8_bom_is_skipped() {
    let out = run_src("tn619_bom.tn", "\u{FEFF}fn main() { print(\"ok\") }");
    assert!(out.status.success(), "BOM-prefixed source should run");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n"),
        "ok\n"
    );
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
