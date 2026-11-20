//! Puts one view of the menu on stdout as a PPM, for looking at on a desk.
//!
//! ```text
//! cargo run -p asmoday --example render -- 1080 2400 confirm zh > menu.ppm
//! ```
//!
//! Views: `list`, `confirm`, `yes`, `fail` (never started), `crash` (died
//! while running), `stop` (stopped on purpose, for an app not built yet).
//! Languages: `zh` (the default), `en`.

use asmoday::{render, Action, Crash, Lang, Reason, View};
use std::io::Write;

fn main() {
    let mut args = std::env::args().skip(1);
    let w: u32 = args.next().and_then(|v| v.parse().ok()).unwrap_or(1080);
    let h: u32 = args.next().and_then(|v| v.parse().ok()).unwrap_or(2400);
    let view = match args.next().as_deref() {
        Some("confirm") => View::Confirm { action: Action::Reset, yes: false },
        Some("yes") => View::Confirm { action: Action::PowerOff, yes: true },
        Some("fail") => View::Failure { crash: Crash::Startup, code: 139, reason: None },
        Some("crash") => View::Failure { crash: Crash::Runtime, code: 139, reason: None },
        Some("stop") => View::Failure { crash: Crash::Runtime, code: 67, reason: Some(Reason::Music) },
        Some("list") | None => View::List { sel: 0 },
        Some(other) => {
            eprintln!("unknown view {other:?}: list, confirm, yes, fail, crash or stop");
            std::process::exit(2);
        }
    };
    let lang = match args.next().as_deref() {
        Some("en") => Lang::En,
        Some("zh") | None => Lang::Zh,
        Some(other) => {
            eprintln!("unknown language {other:?}: zh or en");
            std::process::exit(2);
        }
    };

    let data = render(w, h, view, lang);
    // P6: binary RGB, one header line per field.
    let mut out = format!("P6\n{w} {h}\n255\n").into_bytes();
    out.reserve((w as usize) * (h as usize) * 3);
    for px in data.as_chunks::<4>().0 {
        out.extend_from_slice(&px[..3]);
    }
    std::io::stdout().write_all(&out).expect("write the image");
}
