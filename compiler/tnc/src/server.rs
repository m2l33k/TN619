//! A tiny, dependency-free HTTP server for the TN619 web playground.
//!
//! `tnc serve [port]` starts it; open the printed URL in a browser, type TN619
//! code (English, Arabic, or mixed), and run it. Uses only `std::net` — no web
//! framework, no external crates.

use crate::compile_and_run;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

pub fn serve(port: u16) -> Result<(), String> {
    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr).map_err(|e| format!("cannot bind {}: {}", addr, e))?;
    println!("TN619 playground → http://{}   (Ctrl-C to stop)", addr);
    for stream in listener.incoming().flatten() {
        // Handle connections one at a time; this is a local dev tool.
        let _ = handle(stream);
    }
    Ok(())
}

fn handle(mut stream: TcpStream) -> std::io::Result<()> {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 8192];
    loop {
        if let Some(hp) = find(&buf, b"\r\n\r\n") {
            let head = String::from_utf8_lossy(&buf[..hp]).to_string();
            let clen = content_length(&head);
            if buf.len() >= hp + 4 + clen {
                let body = String::from_utf8_lossy(&buf[hp + 4..hp + 4 + clen]).to_string();
                return respond(&mut stream, &head, &body);
            }
        }
        let n = stream.read(&mut tmp)?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        if buf.len() > 4_000_000 {
            break;
        }
    }
    Ok(())
}

fn respond(stream: &mut TcpStream, head: &str, body: &str) -> std::io::Result<()> {
    let first = head.lines().next().unwrap_or("");
    let is_run = first.starts_with("POST") && first.contains(" /run");

    if is_run {
        let text = match compile_and_run(body) {
            Ok(out) => {
                if out.is_empty() {
                    "(program produced no output)".to_string()
                } else {
                    out
                }
            }
            Err(e) => format!("error: {}", e),
        };
        write_response(
            stream,
            "200 OK",
            "text/plain; charset=utf-8",
            text.as_bytes(),
        )
    } else {
        write_response(
            stream,
            "200 OK",
            "text/html; charset=utf-8",
            PAGE.as_bytes(),
        )
    }
}

fn write_response(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &[u8],
) -> std::io::Result<()> {
    let header = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n",
        status,
        content_type,
        body.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn content_length(head: &str) -> usize {
    for line in head.lines() {
        if let Some((k, v)) = line.split_once(':') {
            if k.trim().eq_ignore_ascii_case("content-length") {
                return v.trim().parse().unwrap_or(0);
            }
        }
    }
    0
}

const PAGE: &str = r####"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>TN619 Playground</title>
<style>
  :root { color-scheme: dark; }
  * { box-sizing: border-box; }
  body { margin: 0; font-family: system-ui, "Segoe UI", sans-serif; background:#0f1419; color:#e6e6e6; }
  header { padding: 14px 20px; border-bottom:1px solid #243; background:#11161d; }
  header h1 { margin:0; font-size:18px; } header span { color:#7fd; font-size:13px; }
  .wrap { display:flex; gap:12px; padding:16px; height: calc(100vh - 120px); }
  .col { flex:1; display:flex; flex-direction:column; }
  label { font-size:12px; color:#8aa; margin-bottom:6px; }
  textarea, pre { flex:1; margin:0; padding:12px; border-radius:8px; border:1px solid #243;
    background:#0b0f14; color:#e6e6e6; font-family: "Cascadia Code", Consolas, monospace; font-size:14px; }
  textarea { resize:none; }
  pre { overflow:auto; white-space:pre-wrap; }
  .bar { padding: 0 16px 14px; display:flex; gap:8px; flex-wrap:wrap; align-items:center; }
  button { background:#1f6feb; color:#fff; border:0; padding:8px 14px; border-radius:6px; cursor:pointer; font-size:14px; }
  button.alt { background:#243; }
  button:hover { filter:brightness(1.1); }
</style>
</head>
<body>
  <header>
    <h1>TN619 Playground <span>— write systems code in English or Arabic</span></h1>
  </header>
  <div class="bar">
    <button onclick="run()">▶ Run (Ctrl+Enter)</button>
    <button class="alt" onclick="load('en')">English sample</button>
    <button class="alt" onclick="load('ar')">Arabic sample</button>
    <button class="alt" onclick="load('poly')">Mixed sample</button>
  </div>
  <div class="wrap">
    <div class="col">
      <label>Source</label>
      <textarea id="src" spellcheck="false" dir="auto"></textarea>
    </div>
    <div class="col">
      <label>Output</label>
      <pre id="out" dir="auto">Press Run…</pre>
    </div>
  </div>
<script>
const samples = {
  en: `fn main() {\n    let age = 20\n    if age > 18 {\n        print("Adult")\n    }\n    print("sum:", sum_to(5))\n}\n\nfn sum_to(n: int) -> int {\n    var s = 0\n    for i in 1..n {\n        s = s + i\n    }\n    s + n\n}`,
  ar: `دالة رئيسي() {\n    دع العمر = ٢٠\n    اذا العمر > ١٨ {\n        اطبع("بالغ")\n    }\n    اطبع("المضروب:", مضروب(٥))\n}\n\nدالة مضروب(ن: عدد) -> عدد {\n    اذا ن <= ١ { ١ } وإلا { ن * مضروب(ن - ١) }\n}`,
  poly: `struct Point { x: int, y: int }\n\nتطبيق Point {\n    دالة sum(&الذات) -> عدد { الذات.x + الذات.y }\n}\n\nدالة رئيسي() {\n    دع p = Point { x: ٣, y: ٤ }\n    اطبع("sum = {p.sum()}")\n}`
};
function load(k){ document.getElementById('src').value = samples[k]; }
async function run(){
  const out = document.getElementById('out');
  out.textContent = "running…";
  try {
    const r = await fetch('/run', { method:'POST', body: document.getElementById('src').value });
    out.textContent = await r.text();
  } catch(e){ out.textContent = "request failed: " + e; }
}
document.addEventListener('keydown', e => { if((e.ctrlKey||e.metaKey) && e.key==='Enter') run(); });
load('poly');
</script>
</body>
</html>"####;
