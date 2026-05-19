//! HIP-0105 allocator ABI (`__base_alloc` / `__base_free`).
//!
//! The host writes JSON payloads into guest linear memory by:
//!   1. Calling `__base_alloc(len) -> ptr`
//!   2. Writing `len` bytes at `ptr`
//!   3. Calling the user's export `fn(ptr, len) -> i64`
//!   4. Calling `__base_free(ptr, len)` on both the input buffer and
//!      the result buffer the export returned.
//!
//! On the guest side we back this with the global Rust allocator. We
//! lay the bytes out as a `Box<[u8]>` so dropping reconstitutes the
//! correct allocation; `leak_box` is the partner that hands a slice's
//! pointer to the host without freeing.

use alloc::alloc::{alloc, dealloc, Layout};
use alloc::boxed::Box;
use core::ptr;

/// Layout used for every HIP-0105 buffer. `u8` alignment keeps the
/// allocator happy across every wasm host and matches what
/// `Box<[u8]>::from_raw` reconstitutes.
fn layout_for(len: usize) -> Layout {
    // `Layout::from_size_align(len, 1)` is safe for any non-zero `len`
    // and we treat zero specially upstream — we never reach this with
    // `len == 0` because the host short-circuits on empty payloads.
    Layout::from_size_align(len.max(1), 1).expect("layout must be representable")
}

/// Host-callable allocator. Allocates `size` bytes of guest linear
/// memory and returns its raw address. The host MUST pair every call
/// with a `__base_free(ptr, size)` once it's done with the buffer.
///
/// Returns `0` on allocation failure (e.g. size negative / too large);
/// the host treats `0` as a fatal guest error.
#[no_mangle]
pub extern "C" fn __base_alloc(size: i32) -> i32 {
    if size <= 0 {
        return 0;
    }
    let len = size as usize;
    let layout = layout_for(len);
    // SAFETY: `alloc` requires a non-zero-size layout; `layout_for`
    // enforces that. A null return is the standard allocation-failure
    // signal, which we map to `0` so the host sees the failure.
    let ptr = unsafe { alloc(layout) };
    if ptr.is_null() {
        return 0;
    }
    ptr as i32
}

/// Host-callable deallocator. Releases a buffer previously returned by
/// `__base_alloc` or `leak_box`.
#[no_mangle]
pub extern "C" fn __base_free(ptr: i32, size: i32) {
    if ptr == 0 || size <= 0 {
        return;
    }
    let len = size as usize;
    let layout = layout_for(len);
    // SAFETY: by ABI contract the host only ever passes us pointers we
    // previously handed out via `__base_alloc` or `leak_box`, both of
    // which use `layout_for(size)`. Re-deriving the layout from the
    // passed `size` lets us call the matching `dealloc` without
    // tracking allocations.
    unsafe {
        dealloc(ptr as *mut u8, layout);
    }
}

/// Hand a `Box<[u8]>` to the host. Returns `(ptr, len)`. The box is
/// leaked — the host owns the memory and MUST call `__base_free`.
///
/// We sidestep `Box::into_raw` because we want the allocation laid out
/// the same way `__base_alloc` produces (via `Layout`) so the host's
/// `__base_free(ptr, len)` is symmetrical. Copying into a fresh
/// allocator-managed buffer is correct and trivial.
pub fn leak_box(bytes: alloc::vec::Vec<u8>) -> (i32, i32) {
    let len = bytes.len();
    if len == 0 {
        return (0, 0);
    }
    let layout = layout_for(len);
    // SAFETY: layout is non-zero by the check above.
    let dst = unsafe { alloc(layout) };
    if dst.is_null() {
        // Allocator OOM — surface a length=0 result. The host treats
        // (0, 0) as an empty response (not an error); this is the best
        // we can do without panicking, which would unwind into wasm
        // trap territory.
        drop(bytes);
        return (0, 0);
    }
    // SAFETY: dst is freshly allocated with `len` bytes, src points to
    // a Vec of exactly `len` bytes, and the regions can't overlap
    // because `dst` came from a separate allocator call.
    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr(), dst, len);
    }
    // We deliberately drop the source Vec; its backing allocation is
    // released by the allocator. The leaked `dst` is what the host owns.
    drop(bytes);
    (dst as i32, len as i32)
}

/// Pack a (ptr, len) pair into the HIP-0105 i64 return value:
/// `(ptr as i64) << 32 | (len as i64 & 0xFFFFFFFF)`.
#[inline]
pub fn pack(ptr: i32, len: i32) -> i64 {
    ((ptr as u32 as u64) << 32 | (len as u32 as u64)) as i64
}

// Vec stays unused here intentionally; keeping `Box` for symmetry with
// the docstring even though we ultimately go through a raw allocation.
#[allow(dead_code)]
fn _box_witness() -> Box<[u8]> {
    Box::<[u8]>::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    // The ABI takes raw i32 pointers, which only round-trip on 32-bit
    // targets (wasm32 is the production case). Host-side `cargo test`
    // runs on a 64-bit machine where casting `*mut u8` to `i32`
    // truncates the high bits and the resulting "pointer" is garbage.
    // Skip the round-trip tests off-wasm; the pack/zero-check logic is
    // pointer-independent and runs everywhere.

    #[cfg_attr(not(target_pointer_width = "32"), ignore = "32-bit only")]
    #[test]
    fn alloc_then_free_roundtrip() {
        let p = __base_alloc(64);
        assert_ne!(p, 0, "alloc should return non-zero pointer");
        __base_free(p, 64);
    }

    #[test]
    fn alloc_zero_or_negative_returns_zero() {
        assert_eq!(__base_alloc(0), 0);
        assert_eq!(__base_alloc(-1), 0);
    }

    #[test]
    fn free_null_or_zero_is_noop() {
        __base_free(0, 64);
        __base_free(1024, 0);
        __base_free(1024, -1);
    }

    #[cfg_attr(not(target_pointer_width = "32"), ignore = "32-bit only")]
    #[test]
    fn leak_box_roundtrip() {
        let payload = alloc::vec![0xAAu8; 32];
        let (ptr, len) = leak_box(payload);
        assert_eq!(len, 32);
        assert_ne!(ptr, 0);
        // SAFETY: we just leaked 32 bytes at `ptr`, read them back to
        // confirm the copy happened.
        let slice = unsafe { core::slice::from_raw_parts(ptr as *const u8, len as usize) };
        assert!(slice.iter().all(|&b| b == 0xAA));
        __base_free(ptr, len);
    }

    #[test]
    fn leak_empty_returns_zero_zero() {
        assert_eq!(leak_box(alloc::vec::Vec::new()), (0, 0));
    }

    #[test]
    fn pack_layout_is_high_ptr_low_len() {
        let v = pack(0x1234, 0x5678);
        assert_eq!((v >> 32) as u32, 0x1234);
        assert_eq!((v & 0xFFFF_FFFF) as u32, 0x5678);
    }

    #[test]
    fn pack_preserves_high_bit_in_unsigned_form() {
        // ptr=0x80000000 (signed -ve) must come out of the high half as
        // 0x80000000 — important because wasm linear memory addresses
        // can sit in the upper 2GiB.
        let v = pack(0x8000_0000u32 as i32, 16);
        assert_eq!((v >> 32) as u32, 0x8000_0000);
        assert_eq!((v as u32), 16);
    }
}
