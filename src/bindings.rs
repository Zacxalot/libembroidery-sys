// Hand-written FFI bindings for the subset of libembroidery used by embroidermaker.
// Covers emb-color.h, emb-thread.h, emb-stitch.h, and emb-pattern.h.

use std::os::raw::{c_char, c_int, c_double};

// --- emb-stitch.h: stitch flag constants ---
pub const NORMAL: c_int = 0;
pub const JUMP: c_int = 1;
pub const TRIM: c_int = 2;
pub const STOP: c_int = 4;
pub const SEQUIN: c_int = 8;
pub const END: c_int = 16;

// --- emb-stitch.h: list node (mirrors EmbStitch_ / EmbStitchList_) ---
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct EmbStitch {
    pub flags: c_int,
    pub xx: c_double,
    pub yy: c_double,
    pub color: c_int,
}

#[repr(C)]
pub struct EmbStitchList {
    pub stitch: EmbStitch,
    pub next: *mut EmbStitchList,
}

// --- emb-color.h ---
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct EmbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

// --- emb-thread.h ---
#[repr(C)]
#[derive(Debug, Copy, Clone)]
#[allow(non_snake_case)] // field names mirror the C struct in emb-thread.h
pub struct EmbThread {
    pub color: EmbColor,
    pub description: *const c_char,
    pub catalogNumber: *const c_char,
}

// --- emb-thread.h: list node (mirrors EmbThreadList_) ---
#[repr(C)]
pub struct EmbThreadList {
    pub thread: EmbThread,
    pub next: *mut EmbThreadList,
}

// --- emb-pattern.h: EmbPattern is opaque, accessed only through pointers ---
#[repr(C)]
pub struct EmbPattern {
    _private: [u8; 0],
}

extern "C" {
    pub fn embPattern_create() -> *mut EmbPattern;
    pub fn embPattern_free(p: *mut EmbPattern);
    pub fn embPattern_addThread(p: *mut EmbPattern, thread: EmbThread) -> c_int;
    pub fn embPattern_addStitchAbs(
        p: *mut EmbPattern,
        x: c_double,
        y: c_double,
        flags: c_int,
        isAutoColorIndex: c_int,
    );
    pub fn embPattern_write(p: *mut EmbPattern, fileName: *const c_char) -> c_int;
    pub fn embPattern_read(p: *mut EmbPattern, fileName: *const c_char) -> c_int;

    /// Split any stitch/jump whose per-axis delta exceeds the given lengths (mm)
    /// into a run of colinear sub-stitches, preserving flags and colour. Several
    /// format writers (EXP, XXX, …) don't clamp long deltas themselves, so calling
    /// this before `embPattern_write` keeps every record inside the format's
    /// encodable range instead of overflowing and drifting.
    pub fn embPattern_correctForMaxStitchLength(
        p: *mut EmbPattern,
        maxStitchLength: c_double,
        maxJumpLength: c_double,
    );

    // Read-side shims (csrc/shim.c): hand back the linked-list heads.
    pub fn emb_pattern_stitch_list(p: *mut EmbPattern) -> *mut EmbStitchList;
    pub fn emb_pattern_thread_list(p: *mut EmbPattern) -> *mut EmbThreadList;
}
