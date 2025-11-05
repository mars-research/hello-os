use crate::serial_println;
use core::mem::{size_of, MaybeUninit};
use crate::multibootv2::BootInformation;
pub const BASE_PAGE_SIZE: usize = 4096;
pub const SUPER_SIZE: usize = 2 * 1024 * 1024;
pub const PAGES_PER_SUPER: usize = SUPER_SIZE / BASE_PAGE_SIZE;

// Will be set after copying the multiboot blob.
pub static mut KERNEL_END: u64 = 0;

unsafe extern "C" {
    // Linker provides this; we only take its address.
    static __end: u8;
}

#[inline(always)]
pub fn kernel_end() -> u64 {
    core::ptr::addr_of!(__end) as u64
}
#[inline(always)]
fn paddr_to_pfn(p: u64) -> u32 {
    (p / BASE_PAGE_SIZE as u64) as u32
}
#[inline(always)]
fn align_up(x: u64, a: u64) -> u64 {
    (x + (a - 1)) & !(a - 1)
}
#[inline(always)]
fn pfn_align_down_to_super(pfn: u32) -> u32 {
    pfn - (pfn % PAGES_PER_SUPER as u32)
}
// ----------------- Page metadata & states -----------------

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[repr(u8)]
pub enum PageState {
    Unavailable = 0,  // kernel, ACPI holes, reserved
    Free4K      = 1,  // on 4K free list
    Free2MB     = 2,  // on 2MB free list (the head page describes the whole 2MB span)
    Alloc4K     = 3,
    Alloc2MB    = 4,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct PageMeta {
    pub state: PageState,
    pub prev: u32,           // doubly-linked list (index = PFN)
    pub next: u32,
    pub head_pfn: u32,       // for 4K pages: pfn of the 2MB head; for 2MB head: == its own pfn
    pub free4k_in_super: u16 // only meaningful for the 2MB head in 4K mode
}

impl Default for PageMeta {
    fn default() -> Self {
        PageMeta {
            state: PageState::Unavailable,
            prev: u32::MAX,
            next: u32::MAX,
            head_pfn: u32::MAX,
            free4k_in_super: 0,
        }
    }
}

// Global pointer to page metadata and total page count.
static mut PAGE_ARRAY: *mut PageMeta = core::ptr::null_mut();
static mut NUM_PAGES: usize = 0;

// Marks [start_pfn, end_pfn_excl) as Unavailable.
#[inline]
unsafe fn mark_unavailable_range(start_pfn: u32, end_pfn_excl: u32) {
    let mut p = start_pfn;
    let limit = end_pfn_excl.min(NUM_PAGES as u32);
    while p < limit {
        let m = &mut *PAGE_ARRAY.add(p as usize);
        m.state = PageState::Unavailable;
        m.prev = u32::MAX;
        m.next = u32::MAX;
        m.head_pfn = u32::MAX;
        m.free4k_in_super = 0;
        p += 1;
    }
}
/// Mark a single usable region [start_paddr, end_paddr) as Free4K / Free2MB.
/// Assumes PAGE_ARRAY is initialized and [0, KERNEL_END) already Unavailable.
/// Does **not** touch any free lists.
unsafe fn seed_region_no_lists(start_paddr: u64, end_paddr: u64) {
    if end_paddr <= start_paddr {
        return;
    }

    let region_start_pfn = (start_paddr / BASE_PAGE_SIZE as u64) as u32;
    let region_end_pfn   = ((end_paddr  + (BASE_PAGE_SIZE as u64 - 1)) / BASE_PAGE_SIZE as u64) as u32; // ceil, exclusive
    let kernel_end_pfn   = (KERNEL_END / BASE_PAGE_SIZE as u64) as u32;

    // Usable part starts at max(region_start, KERNEL_END)
    let mut p = region_start_pfn.max(kernel_end_pfn);
    if p >= region_end_pfn {
        return;
    }

    // A) From p up to next 2MiB boundary → Free4K
    let first_super_boundary_bytes = align_up(KERNEL_END.max(start_paddr), SUPER_SIZE as u64);
    let preface_end_pfn = ((first_super_boundary_bytes) / BASE_PAGE_SIZE as u64) as u32;
    let preface_end_pfn = preface_end_pfn.min(region_end_pfn);

    while p < preface_end_pfn {
        let m = &mut *PAGE_ARRAY.add(p as usize);
        // Don’t touch anything below KERNEL_END; p already >= kernel_end_pfn
        m.state = PageState::Free4K;
        m.prev = u32::MAX;
        m.next = u32::MAX;
        m.head_pfn = pfn_align_down_to_super(p);
        m.free4k_in_super = 0;
        p += 1;
    }

    // B) Full 2MiB-aligned spans → Free2MB (head + followers)
    let mut cur = p as usize;
    if cur % PAGES_PER_SUPER != 0 {
        cur += PAGES_PER_SUPER - (cur % PAGES_PER_SUPER);
    }

    while cur + PAGES_PER_SUPER <= region_end_pfn as usize {
        let head = cur as u32;
        for off in 0..PAGES_PER_SUPER {
            let pf = head + off as u32;
            let m = &mut *PAGE_ARRAY.add(pf as usize);
            m.state = PageState::Free2MB;
            m.prev = u32::MAX;
            m.next = u32::MAX;
            m.head_pfn = head;        // head points to itself; followers to head
            m.free4k_in_super = 0;    // only used when split to 4K later
        }
        cur += PAGES_PER_SUPER;
    }

    // C) Trailing tail → Free4K
    let mut t = cur as u32;
    while t < region_end_pfn {
        let m = &mut *PAGE_ARRAY.add(t as usize);
        m.state = PageState::Free4K;
        m.prev = u32::MAX;
        m.next = u32::MAX;
        m.head_pfn = pfn_align_down_to_super(t);
        m.free4k_in_super = 0;
        t += 1;
    }
}

pub unsafe fn mark_free_states_no_lists(boot: &BootInformation) {
    if let Some(mmap) = boot.memory_map_tag() {
        for area in mmap.memory_areas() {
            let start = area.start_address() as u64;
            let end   = start + area.size() as u64;

            // If your iterator includes non-usable areas, add a filter here (e.g. typ==1).
            // For now we trust the iterator contains usable areas.
            seed_region_no_lists(start, end);
        }
    }
}

/// Initialize only:
///  - Lay out PAGE_ARRAY after KERNEL_END (aligned) and bump KERNEL_END
///  - Mark [0, KERNEL_END) as Unavailable
/// No free lists, no memory map processing.
pub unsafe fn init_allocator(boot: &BootInformation) {
    // 1) Size the metadata: one entry per 4KiB page up to the highest usable end.
    let max_end = boot.max_usable_phys_end();
    NUM_PAGES = (max_end as usize / BASE_PAGE_SIZE) as usize;
    let np = unsafe{NUM_PAGES};
    serial_println!(
        "Page metadata init: max_end={:#x}, NUM_PAGES={} (~{} MiB)",
        max_end,
        np,
        (np * BASE_PAGE_SIZE) / (1024 * 1024)
    );

    // 2) Place PAGE_ARRAY right after current KERNEL_END (page-aligned).
    let meta_bytes = NUM_PAGES
        .checked_mul(size_of::<PageMeta>())
        .expect("PAGE_ARRAY size overflow");
    let meta_base = align_up(KERNEL_END, BASE_PAGE_SIZE as u64) as usize;
    let meta_bytes_rounded = align_up(meta_bytes as u64, BASE_PAGE_SIZE as u64) as usize;

    PAGE_ARRAY = meta_base as *mut PageMeta;

    // 3) Default-initialize page_array (everything starts Unavailable).
    {
        let slots = core::slice::from_raw_parts_mut(
            PAGE_ARRAY as *mut MaybeUninit<PageMeta>,
            NUM_PAGES,
        );
        for slot in slots.iter_mut() {
            slot.write(PageMeta::default());
        }
    }

    // 4) Bump KERNEL_END to cover metadata region.
    KERNEL_END = (meta_base + meta_bytes_rounded) as u64;
    let ke = unsafe{KERNEL_END};
    serial_println!(
        "PAGE_ARRAY @ {:#x}, size={} (rounded {}), new KERNEL_END={:#x}",
        meta_base,
        meta_bytes,
        meta_bytes_rounded,
        ke
    );

    // 5) Explicitly mark [0, KERNEL_END) as Unavailable in page_array.
    //    (Covers kernel image, modules you copied, and the metadata itself.)
    let end_pfn = paddr_to_pfn(KERNEL_END);
    mark_unavailable_range(0, end_pfn);
    mark_free_states_no_lists(boot);
}


#[inline(always)]
fn state_str(s: PageState) -> &'static str {
    match s {
        PageState::Unavailable => "Unavailable",
        PageState::Free4K      => "Free4K",
        PageState::Free2MB     => "Free2MB",
        PageState::Alloc4K     => "Alloc4K",
        PageState::Alloc2MB    => "Alloc2MB",
    }
}

pub unsafe fn dump_allocator_header() {
    // Take copies from statics first
    let num_pages   = NUM_PAGES;
    let page_array  = PAGE_ARRAY as usize;
    let kernel_end  = KERNEL_END;
    let end_pfn     = paddr_to_pfn(kernel_end);

    serial_println!("=== allocator header ===");
    serial_println!("NUM_PAGES      = {}", num_pages);
    serial_println!("PAGE_ARRAY @   = {:#x}", page_array);
    serial_println!("KERNEL_END     = {:#x}", kernel_end);
    serial_println!("end_pfn        = {}", end_pfn);
    serial_println!("========================");
}

pub unsafe fn dump_page_array(first: usize, count: usize) {
    let num_pages = NUM_PAGES; // local copy to avoid borrowing static in format!
    let end = (first + count).min(num_pages);
    serial_println!("--- page_array[{}..{}) ---", first, end);
    for pfn in first..end {
        let m = &*PAGE_ARRAY.add(pfn);
        serial_println!(
            "[{:>6}] state={:>12} prev={:#010x} next={:#010x} head_pfn={:#010x} free4k={}",
            pfn,
            state_str(m.state),
            m.prev,
            m.next,
            m.head_pfn,
            m.free4k_in_super
        );
    }
    serial_println!("--------------------------");
}

pub unsafe fn dump_around_kernel_end(pad: usize) {
    let kernel_end = KERNEL_END;
    let num_pages  = NUM_PAGES;
    let end_pfn    = paddr_to_pfn(kernel_end) as isize;

    let start = (end_pfn - pad as isize).max(0) as usize;
    let cnt   = (pad * 2 + 1).min(num_pages.saturating_sub(start));

    serial_println!(
        "=== around KERNEL_END: end_pfn={}, window=[{}..{}) ===",
        end_pfn, start, start + cnt
    );
    dump_page_array(start, cnt);
}

pub unsafe fn dump_state_summary() {
    let num_pages = NUM_PAGES;

    let mut c_un = 0usize;
    let mut c_f4 = 0usize;
    let mut c_f2 = 0usize;
    let mut c_a4 = 0usize;
    let mut c_a2 = 0usize;

    for pfn in 0..num_pages {
        match (*PAGE_ARRAY.add(pfn)).state {
            PageState::Unavailable => c_un += 1,
            PageState::Free4K      => c_f4 += 1,
            PageState::Free2MB     => c_f2 += 1,
            PageState::Alloc4K     => c_a4 += 1,
            PageState::Alloc2MB    => c_a2 += 1,
        }
    }

    serial_println!(
        "state summary: Unavail={} Free4K={} Free2MB={} Alloc4K={} Alloc2MB={}",
        c_un, c_f4, c_f2, c_a4, c_a2
    );
}
