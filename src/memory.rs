use limine::{LimineMemmapEntry, LimineMemoryMapEntryType, LimineHhdmRequest};
use core::{ptr::NonNull};
use core::fmt;
use core::error::Error;
static HHDM: LimineHhdmRequest = LimineHhdmRequest::new(0);

#[derive(Debug)]
pub enum MemoryError {
    OutOfPages,
    NotPresent,
    InvalidMap,
    MapTooDeep
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
            MemoryError::InvalidMap => "wrong page map used",
            MemoryError::MapTooDeep => "page mapped too far",
        }
    }
}

pub struct Freelist {
    address: Option<NonNull<u8>>,
    next: Option<NonNull<Freelist>>
}

#[derive(Debug)]
pub struct Pagemap {
    level: u8,
    data: *mut usize,
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
}

pub fn build_freelist(mmap: Option<&[LimineMemmapEntry]>) {
    unsafe {
        HHDM_VAL = Some(HHDM
            .get_response()
            .get()
            .expect("rotom: received no hhdm")
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
                let result = free_pages.as_mut().address.unwrap();
                FREE_PAGES = free_pages.as_mut().next;
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
    fn new_mapping(&self, entry: usize, flags: usize) -> Result<Pagemap, MemoryError> {
        if let Ok(mapping) = self.get_mapping(entry) {
            return Ok(mapping.0);
        }
        let page = Pagemap::new(self.level - 1)?;
        self.set_mapping(entry, page.data as usize, flags)
    }
    fn get_mapping(&self, entry: usize) -> Result<(Pagemap, usize), MemoryError> {
        if self.level == 0 {
            return Err(MemoryError::MapTooDeep);
        }
        unsafe {
            let mut entdata = *((self.data as usize + entry*8) as *mut usize);
            let flags = entdata & 0xFFF;
            if flags & 1 == 0 {
                return Err(MemoryError::NotPresent);
            }
            entdata &= 0xFFFFFFFFFFFFF000;
            return Ok((Pagemap{level:self.level-1,data:entdata as *mut usize}, flags));
        }
    }
    fn set_mapping(&self, entry: usize, mapping: usize, flags: usize) -> Result<Pagemap,MemoryError> {
        if self.level == 0 {
            return Err(MemoryError::MapTooDeep);
        }
        unsafe {
            let entptr = (self.data as usize + entry*8) as *mut usize;
            *entptr = mapping | flags;
            return Ok(Pagemap { level: self.level-1, data: mapping as *mut usize});
        }
    }
    pub fn set_vpage(&self, paddr: usize, vaddr: usize, flags: usize) -> Result<Pagemap,MemoryError> {
        if self.level != 4 {
            return Err(MemoryError::InvalidMap);
        }
        let entry4 = (vaddr & 0x0000FF8000000000) << 39;
        let entry3 = (vaddr & 0x0000007FC0000000) >> 30;
        let entry2 = (vaddr & 0x000000003FE00000) >> 21;
        let entry1 = (vaddr & 0x00000000001FF000) >> 12;
        let level3 = self.new_mapping(entry4, flags)?;
        let level2 = level3.new_mapping(entry3, flags)?;
        let level1 = level2.new_mapping(entry2, flags)?;
        level1.set_mapping(entry1, paddr, flags)
    }
    pub fn get_vpage(&self, vaddr: usize) -> Result<(Pagemap, usize),MemoryError> {
        if self.level != 4 {
            return Err(MemoryError::InvalidMap);
        }
        let entry4 = (vaddr & 0x0000FF8000000000) << 39;
        let entry3 = (vaddr & 0x0000007FC0000000) >> 30;
        let entry2 = (vaddr & 0x000000003FE00000) >> 21;
        let entry1 = (vaddr & 0x00000000001FF000) >> 12;
        let level3 = self.get_mapping(entry4)?.0;
        let level2 = level3.get_mapping(entry3)?.0;
        let level1 = level2.get_mapping(entry2)?.0;
        level1.get_mapping(entry1)
    }
}

pub fn get_current_pagemap() -> Pagemap {
    unsafe {
        Pagemap {level:4,data:x86::controlregs::cr3() as *mut usize}
    }
}