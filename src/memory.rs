use limine::{LimineMemmapEntry, LimineMemoryMapEntryType};

pub enum Freelist {
    Sentinel,
    Page {
        address: *mut u8,
        next: *mut Freelist
    }
}

impl Freelist {
    fn new(address: *mut u8, next: *mut Freelist) -> *mut Freelist {
        let result = address as *mut Freelist;
        unsafe {
            *result = Freelist::Page {
                address: address,
                next: next
            };
            result
        }
    }
}

pub fn build_freelist(mmap: Option<&[LimineMemmapEntry]>) -> *mut Freelist {
    let mut free_pages = &mut Freelist::Sentinel as *mut Freelist;
    for ment in mmap.unwrap() {
        if ment.typ != LimineMemoryMapEntryType::Usable {continue};
        let end = ment.base + ment.len;
        let mut cur = ment.base;
        while cur < end {
            free_pages = Freelist::new(cur as *mut u8, free_pages);
            cur += 4096;
        }
    }
    free_pages
}