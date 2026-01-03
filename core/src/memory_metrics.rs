use std::alloc::{GlobalAlloc, Layout};
use std::sync::atomic::{AtomicU64, Ordering};

static CURRENT: AtomicU64 = AtomicU64::new(0);
static PEAK: AtomicU64 = AtomicU64::new(0);

pub struct CountingAllocator<A> {
    inner: A,
}

impl<A> CountingAllocator<A> {
    pub const fn new(inner: A) -> Self {
        Self { inner }
    }
}

fn update_peak(new_current: u64) {
    let mut peak = PEAK.load(Ordering::Relaxed);
    while new_current > peak {
        match PEAK.compare_exchange_weak(
            peak,
            new_current,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => return,
            Err(p) => peak = p,
        }
    }
}

unsafe impl<A: GlobalAlloc> GlobalAlloc for CountingAllocator<A> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { self.inner.alloc(layout) };
        if !ptr.is_null() {
            let size = layout.size() as u64;
            let new_current = CURRENT.fetch_add(size, Ordering::Relaxed).saturating_add(size);
            update_peak(new_current);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { self.inner.dealloc(ptr, layout) };
        let size = layout.size() as u64;
        let mut cur = CURRENT.load(Ordering::Relaxed);
        loop {
            let next = cur.saturating_sub(size);
            match CURRENT.compare_exchange_weak(cur, next, Ordering::Relaxed, Ordering::Relaxed) {
                Ok(_) => break,
                Err(actual) => cur = actual,
            }
        }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let old_size = layout.size() as u64;
        let ptr2 = unsafe { self.inner.realloc(ptr, layout, new_size) };
        if !ptr2.is_null() {
            let new_size_u = new_size as u64;
            if new_size_u >= old_size {
                let delta = new_size_u - old_size;
                let new_current =
                    CURRENT.fetch_add(delta, Ordering::Relaxed).saturating_add(delta);
                update_peak(new_current);
            } else {
                let delta = old_size - new_size_u;
                let mut cur = CURRENT.load(Ordering::Relaxed);
                loop {
                    let next = cur.saturating_sub(delta);
                    match CURRENT.compare_exchange_weak(
                        cur,
                        next,
                        Ordering::Relaxed,
                        Ordering::Relaxed,
                    ) {
                        Ok(_) => break,
                        Err(actual) => cur = actual,
                    }
                }
            }
        }
        ptr2
    }
}

pub fn reset_peak_to_current() {
    let cur = CURRENT.load(Ordering::Relaxed);
    PEAK.store(cur, Ordering::Relaxed);
}

pub fn peak_bytes() -> u64 {
    PEAK.load(Ordering::Relaxed)
}
