use limine::{LimineMemmapEntry, LimineMemoryMapEntryType, LimineHhdmRequest};
use x86::bits32::paging::Page;
use core::{ptr::NonNull};
use core::fmt;
use core::error::Error;
static HHDM: LimineHhdmRequest = LimineHhdmRequest::new(0);

#[derive(Debug)]
pub enum MemoryError {
    OutOfPages,
    NotPresent
}

impl fmt::Display for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,"{}",self)
    }
}

impl Error for MemoryError {
    fn description(&self) -> &str {
        match self {
            MemoryError::OutOfPages => "out of memory pages",
            MemoryError::NotPresent => "mapping not present",
        }
    }
}

pub struct Freelist {
    address: Option<NonNull<u8>>,
    next: Option<NonNull<Freelist>>
}

pub struct Pagemap {
    level: u8,
    data: *mut usize
}

static mut FREE_PAGES: Option<NonNull<Freelist>> = None;
static mut HHDM_VAL: Option<u64> = None;

impl Freelist {
    fn new(address: NonNull<u8>, next: Option<NonNull<Freelist>>) -> Option<NonNull<Freelist>> {
        unsafe {
            let result = NonNull::<Freelist>::new((address.as_ptr() as u64 + HHDM_VAL.unwrap()) as *mut Freelist);
            *result.unwrap().as_mut() = Freelist {
                address: Some(address),
                next
            };
            result
        }
    }
    fn get_address(&self) -> Option<NonNull<u8>> {
        return self.address;
    }
    fn get_next(&self) -> Option<NonNull<Freelist>> {
        return self.next;
    }
}

pub fn build_freelist(mmap: Option<&[LimineMemmapEntry]>) {
    unsafe {
        HHDM_VAL = Some(HHDM
            .get_response()
            .get()
            .expect("barebones: received no hhdm")
            .offset);
    }
    for ment in mmap.unwrap() {
        if ment.typ != LimineMemoryMapEntryType::Usable {continue};
        let end = ment.base + ment.len;
        let mut cur = ment.base;
        // Add every page to the list.
        while cur < end {
            unsafe {FREE_PAGES = Freelist::new(NonNull::new(cur as *mut u8).unwrap(), FREE_PAGES);}
            cur += 4096;
        }
    }
}

pub fn allocate_page() -> Result<NonNull<u8>, MemoryError> {
    unsafe {
        match FREE_PAGES {
            Some(mut free_pages) => {
                let result = free_pages.as_mut().get_address().unwrap();
                FREE_PAGES = free_pages.as_mut().get_next();
                Ok(result)
            }
            None => Err(MemoryError::OutOfPages)
        }
    }
}

pub fn deallocate_page(address: NonNull<u8>) {
    unsafe {FREE_PAGES = Freelist::new(address, FREE_PAGES);}
}

impl Pagemap {
    fn new(level: u8) -> Result<Pagemap, MemoryError> {
        let res = allocate_page();
        match res {
            Ok(page) => Ok(Pagemap{level, data:page.as_ptr() as *mut usize}),
            Err(err) => Err(err)
        }
    }
    fn get_mapping(&self, entry: usize) -> Result<Pagemap, MemoryError> {
        unsafe {
            let mut entdata = *((self.data as usize + entry*8) as *mut usize);
            if entdata & 1 == 0 {
                return Err(MemoryError::NotPresent);
            }
            entdata &= 0xFFFFFFFFFFFFF000;
            return Ok(Pagemap{level:self.level-1,data:entdata as *mut usize});
        }
    }
    fn set_mapping(&self, entry: usize, mapping: usize) -> Result<Pagemap,MemoryError> {
        unsafe {
            let entptr = (self.data as usize + entry*8) as *mut usize;
            *entptr = mapping;
            return Ok(Pagemap { level: self.level-1, data: (mapping & 0xFFFFFFFFFFFFF000) as *mut usize});
        }
    }
}