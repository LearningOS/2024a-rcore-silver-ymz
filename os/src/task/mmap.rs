use crate::{
    config::PAGE_SIZE,
    mm::{MapPermission, VirtAddr},
};

use super::current_task;

/// mmap a virtual page to a physical page, used for `sys_mmap`
pub fn mmap_page(start: usize, len: usize, port: usize) -> bool {
    let mut flags = unsafe { MapPermission::from_bits_unchecked((port as u8) << 1) };
    flags.set(MapPermission::U, true);

    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();

    for i in 0..((len + PAGE_SIZE - 1) / PAGE_SIZE) {
        let vpn = VirtAddr::from(start + i * PAGE_SIZE).into();
        if let Some(pte) = inner.memory_set.translate(vpn) {
            if pte.is_valid() {
                log::info!("mmap_page: area {:?} already mapped", vpn);
                return false;
            }
        }
    }

    inner
        .memory_set
        .insert_framed_area(start.into(), (start + len).into(), flags);
    true
}

/// munmap a virtual page, used for `sys_munmap`
pub fn munmap_page(start: usize, len: usize) -> bool {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();

    for i in 0..(len / PAGE_SIZE) {
        let vpn = VirtAddr::from(start + i * PAGE_SIZE).into();
        if let Some(pte) = inner.memory_set.translate(vpn) {
            if pte.is_valid() {
                continue;
            }
        }
        log::info!("munmap_page: area {:?} not mapped", vpn);
        return false;
    }

    inner
        .memory_set
        .remove(VirtAddr::from(start), VirtAddr::from(start + len));
    true
}
