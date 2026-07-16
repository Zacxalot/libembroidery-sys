// Regression test: the PEC label field is a fixed 16-char field. Export file
// names longer than 16 chars (before the extension) must not overrun it, or the
// colour count / palette bytes that follow are shifted and readers report a
// wildly wrong thread list. See format-pec.c writePecStitches().

use libembroidery_sys::*;
use std::convert::TryInto;
use std::ffi::CString;

fn write_three_color_pes(path: &str) {
    unsafe {
        let p = embPattern_create();
        assert!(!p.is_null());

        let colors = [
            EmbColor { r: 200, g: 0, b: 0 },
            EmbColor { r: 0, g: 200, b: 0 },
            EmbColor { r: 0, g: 0, b: 200 },
        ];
        let empty = CString::new("").unwrap();
        for c in &colors {
            embPattern_addThread(
                p,
                EmbThread {
                    color: *c,
                    description: empty.as_ptr(),
                    catalogNumber: empty.as_ptr(),
                },
            );
        }

        // Three colour blocks separated by STOP (auto colour index) commands.
        let mut y = 0.0;
        for block in 0..3 {
            if block > 0 {
                embPattern_addStitchAbs(p, 0.0, y, STOP, 1);
            }
            embPattern_addStitchAbs(p, 0.0, y, JUMP, 1);
            for _ in 0..5 {
                embPattern_addStitchAbs(p, 10.0, y, NORMAL, 1);
                embPattern_addStitchAbs(p, 0.0, y + 1.0, NORMAL, 1);
                y += 1.0;
            }
        }
        embPattern_addStitchAbs(p, 0.0, y, END, 1);

        let c_path = CString::new(path).unwrap();
        let rc = embPattern_write(p, c_path.as_ptr());
        assert_eq!(rc, 1, "embPattern_write failed");
        embPattern_free(p);
    }
}

/// Read the PEC colour-count byte the way a conformant reader does: label is a
/// FIXED 16-byte field, not variable length. Returns thread count.
fn pec_thread_count(bytes: &[u8]) -> usize {
    let pec_ptr = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    let mut off = pec_ptr;
    assert_eq!(&bytes[off..off + 3], b"LA:");
    off += 3;
    off += 16; // fixed 16-char label
    assert_eq!(bytes[off], 0x0D, "label field is not exactly 16 bytes");
    off += 1;
    off += 12; // spaces
    off += 4; // 0xFF 0x00 0x06 0x26
    off += 12; // spaces
    bytes[off] as usize + 1
}

#[test]
fn long_export_name_keeps_pec_label_fixed_width() {
    let dir = std::env::temp_dir();

    let short = dir.join("marge.pes");
    let long = dir.join("big-marge-compensate.pes"); // 20-char stem, > 16

    write_three_color_pes(short.to_str().unwrap());
    write_three_color_pes(long.to_str().unwrap());

    let short_bytes = std::fs::read(&short).unwrap();
    let long_bytes = std::fs::read(&long).unwrap();

    assert_eq!(pec_thread_count(&short_bytes), 3, "short name baseline");
    assert_eq!(
        pec_thread_count(&long_bytes),
        3,
        "long export name corrupted the PEC colour table"
    );

    let _ = std::fs::remove_file(&short);
    let _ = std::fs::remove_file(&long);
}
