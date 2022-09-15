use limine::{LimineMemmapEntry, LimineMemoryMapEntryType, LimineHhdmRequest};

static HHDM: LimineHhdmRequest = LimineHhdmRequest::new(0);

pub enum Freelist {
    Sentinel,
    Page {
        address: *mut u8,
        next: *mut Freelist
    }
}

static mut FREE_PAGES: *mut Freelist = 0 as *mut Freelist;
static mut HHDM_VAL: u64 = 0;

impl Freelist {
    fn new(address: *mut u8, next: *mut Freelist) -> *mut Freelist {
        unsafe {
            let result = (address as u64 + HHDM_VAL) as *mut Freelist;
            *result = Freelist::Page {
                address: address,
                next: next
            };
            result
        }
    }
    fn get_address(&self) -> Option<*mut u8> {
        match self {
            Freelist::Page{address,..} => {
                Some(*address)
            }
            Freelist::Sentinel => {
                None
            }
        }
    }
    fn get_next(&self) -> Option<*mut Freelist> {
        match self {
            Freelist::Page{next,..} => {
                    Some(*next)
            }
            Freelist::Sentinel => {
                None
            }
        }
    }
}

pub fn build_freelist(mmap: Option<&[LimineMemmapEntry]>) {
    unsafe {
        HHDM_VAL = HHDM
            .get_response()
            .get()
            .expect("barebones: received no hhdm")
            .offset;
        // Create sentinel value.
        FREE_PAGES = &mut Freelist::Sentinel;
    }
    for ment in mmap.unwrap() {
        if ment.typ != LimineMemoryMapEntryType::Usable {continue};
        let end = ment.base + ment.len;
        let mut cur = ment.base;
        // Add every page to the list.
        while cur < end {
            unsafe {FREE_PAGES = Freelist::new(cur as *mut u8, FREE_PAGES);}
            cur += 4096;
        }
    }
}

pub fn allocate_page() -> Option<*mut u8> {
    unsafe {
        match FREE_PAGES.as_mut().unwrap().get_address() {
            Some(result) => {
                FREE_PAGES = FREE_PAGES.as_mut().unwrap().get_next().unwrap();
                Some(result)
            }
            None => {
                None
            }
        }
    }
}

pub fn deallocate_page(address: *mut u8) {
    unsafe {FREE_PAGES = Freelist::new(address, FREE_PAGES);}
}