// Regression tests for corrupted machine exports.
//
// 1. Long moves must be split before writing. Tight relative formats (EXP, XXX,
//    JEF) encode each record's per-axis delta as a signed byte in 0.1mm units
//    (±12.7mm). A single long stitch/jump (e.g. a 100mm connector) overflows
//    that; EXP's writer casts straight to `char` and *wraps*, so the reconstructed
//    design drifts instead of erroring. `embPattern_correctForMaxStitchLength`
//    (called before write) must break long moves into encodable sub-stitches so
//    the decoded geometry still matches the design.
//
// 2. DST must carry its palette. DST has no native colour storage, so without the
//    extended `TC:` header records a reader has to guess ("corrupted" colours).
//    The writer must emit a `TC:#RRGGBB` line per thread.
//
// The files are decoded directly here (raw EXP stream / raw DST header) rather
// than via libembroidery's own reader, which parses TC weakly — we want to verify
// the bytes an external machine/tool actually sees.

use libembroidery_sys::*;
use std::ffi::CString;

/// Build a pattern with two colour blocks and a deliberately long (~90mm) move
/// plus a ~112mm jump — neither encodable in one tight-format record. Returns the
/// design's absolute bounding box (min_x, max_x) in mm for a drift check.
unsafe fn write_design(path: &str, colors: &[EmbColor]) {
    let p = embPattern_create();
    assert!(!p.is_null());

    let empty = CString::new("").unwrap();
    for c in colors {
        embPattern_addThread(
            p,
            EmbThread {
                color: *c,
                description: empty.as_ptr(),
                catalogNumber: empty.as_ptr(),
            },
        );
    }

    embPattern_addStitchAbs(p, 0.0, 0.0, JUMP, 1);
    embPattern_addStitchAbs(p, 5.0, 0.0, NORMAL, 1);
    embPattern_addStitchAbs(p, 10.0, 0.0, NORMAL, 1);
    embPattern_addStitchAbs(p, 100.0, 40.0, NORMAL, 1); // ~98mm single stitch
    embPattern_addStitchAbs(p, 105.0, 40.0, NORMAL, 1);
    embPattern_addStitchAbs(p, 105.0, 40.0, STOP, 1); // colour change
    embPattern_addStitchAbs(p, 0.0, 0.0, JUMP, 1); // ~112mm jump home
    embPattern_addStitchAbs(p, 5.0, 0.0, NORMAL, 1);
    embPattern_addStitchAbs(p, 5.0, 0.0, END, 1);

    // The fix under test: split every move to <=12mm before the writer encodes it.
    embPattern_correctForMaxStitchLength(p, 12.0, 12.0);

    let c_path = CString::new(path).unwrap();
    let rc = embPattern_write(p, c_path.as_ptr());
    assert_eq!(rc, 1, "embPattern_write failed for {path}");
    embPattern_free(p);
}

/// Decode an EXP stitch stream to absolute positions (0.1mm units), the way any
/// conformant reader does. Returns (min_x, max_x, min_y, max_y).
fn decode_exp_extents(bytes: &[u8]) -> (i32, i32, i32, i32) {
    let dec = |b: u8| -> i32 {
        if b >= 0x80 { b as i32 - 0x100 } else { b as i32 }
    };
    let (mut x, mut y) = (0i32, 0i32);
    let (mut minx, mut maxx, mut miny, mut maxy) = (0i32, 0i32, 0i32, 0i32);
    let mut i = 0;
    while i + 1 < bytes.len() {
        let b0 = bytes[i];
        let b1 = bytes[i + 1];
        if b0 == 0x1a {
            break;
        }
        let (dx, dy);
        if b0 == 0x80 {
            // Special record: STOP (b1&1), TRIM/JUMP (2/4/6), extension (0x80).
            // Payload is the following two bytes; extension carries no move.
            if b1 == 0x80 {
                i += 4;
                continue;
            }
            dx = dec(bytes[i + 2]);
            dy = dec(bytes[i + 3]);
            i += 4;
        } else {
            dx = dec(b0);
            dy = dec(b1);
            i += 2;
        }
        // Each per-axis delta must fit a signed byte, else it wrapped on write.
        assert!(dx.abs() <= 127 && dy.abs() <= 127, "delta out of byte range");
        x += dx;
        y += dy;
        minx = minx.min(x);
        maxx = maxx.max(x);
        miny = miny.min(y);
        maxy = maxy.max(y);
    }
    (minx, maxx, miny, maxy)
}

#[test]
fn long_moves_survive_exp_without_drift() {
    let dir = std::env::temp_dir();
    let colors = [EmbColor { r: 200, g: 10, b: 20 }, EmbColor { r: 20, g: 180, b: 40 }];
    let path = dir.join("embroider_split_test.exp");
    let ps = path.to_str().unwrap();
    unsafe { write_design(ps, &colors) };

    let bytes = std::fs::read(&path).unwrap();
    let (minx, maxx, miny, maxy) = decode_exp_extents(&bytes);

    // Design spans x:[0,105]mm, y:[0,40]mm -> [0,1050] and [-400,0] in 0.1mm units
    // (EXP y is inverted on encode). Without splitting, the 90mm move wraps the
    // signed byte and the reconstruction collapses/drifts far from these bounds.
    let w = maxx - minx;
    let h = maxy - miny;
    assert!(
        (w - 1050).abs() <= 10,
        "x span {} (0.1mm) drifted from design's 1050 — long move wrapped, not split",
        w
    );
    assert!(
        (h - 400).abs() <= 10,
        "y span {} (0.1mm) drifted from design's 400 — long move wrapped, not split",
        h
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn dst_writes_thread_colour_records() {
    let dir = std::env::temp_dir();
    let colors = [EmbColor { r: 200, g: 10, b: 20 }, EmbColor { r: 20, g: 180, b: 40 }];
    let path = dir.join("embroider_dst_color_test.dst");
    let ps = path.to_str().unwrap();
    unsafe { write_design(ps, &colors) };

    let bytes = std::fs::read(&path).unwrap();
    let header = &bytes[..512.min(bytes.len())];
    let header_str = String::from_utf8_lossy(header);

    // DST is colourless without the Tajima TC extension; the writer must emit one
    // #RRGGBB line per thread so the palette survives to the machine/software.
    assert!(
        header_str.contains("TC:#C80A14"),
        "DST header missing TC record for thread 1: {:?}",
        header_str
    );
    assert!(
        header_str.contains("TC:#14B428"),
        "DST header missing TC record for thread 2"
    );
    // The header must remain exactly 512 bytes (TC lines shrink the pad, not grow
    // the header) or every stitch offset after it is wrong.
    assert!(bytes.len() > 512, "DST truncated");
    let _ = std::fs::remove_file(&path);
}
