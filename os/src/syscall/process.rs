//! Process management syscalls
use crate::config::PAGE_SIZE;
use crate::mm::{write_struct_to_user, MapPermission, PageTable, VirtAddr};
use crate::task::{
    change_program_brk, current_user_token, erase_map_area, exit_current_and_run_next,
    find_trace_info, insert_map_area, suspend_current_and_run_next,
};
use crate::timer::get_time_us;
#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    let token = current_user_token();
    let page_table = PageTable::from_token(token);
    let start_va = VirtAddr::from(ts as usize);
    let end_va = VirtAddr::from(ts as usize + core::mem::size_of::<TimeVal>());

    let timeval = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };

    if start_va.floor() == end_va.floor() {
        // case 1: struct is fully in one page
        let pte = page_table.translate(start_va.floor()).unwrap();
        if pte.readable() && pte.writable() {
            let ptr = (pte.ppn().0 << 12 | start_va.page_offset()) as *mut TimeVal;
            assert!(ptr as usize % core::mem::align_of::<TimeVal>() == 0);
            unsafe {
                *ptr = timeval;
            }
        } else {
            trace!("the address is not readable or writable");
            return -1;
        }
    } else {
        // case 2: struct spans two pages
        if let Err(e) = write_struct_to_user(token, ts as usize, &timeval) {
            trace!("sys_get_time: failed to write timeval across pages: {}", e);
            return -1;
        }
    }

    0
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    let ret: isize = match trace_request {
        0 => {
            // read user space
            let token = current_user_token();
            let page_table = PageTable::from_token(token);
            let va = VirtAddr::from(id);
            if let Some(pte) = page_table.translate(va.floor()) {
                if pte.is_user_visible() && pte.readable() {
                    let ptr = (pte.ppn().0 << 12 | va.page_offset()) as *const u8;
                    unsafe { *ptr as isize }
                } else {
                    -1
                }
            } else {
                -1
            }
        }
        1 => {
            // write user space
            let token = current_user_token();
            let page_table = PageTable::from_token(token);
            let va = VirtAddr::from(id);
            if let Some(pte) = page_table.translate(va.floor()) {
                if pte.is_user_visible() && pte.writable() {
                    let ptr = (pte.ppn().0 << 12 | va.page_offset()) as *mut u8;
                    unsafe {
                        *ptr = data as u8;
                        0
                    }
                } else {
                    -1
                }
            } else {
                -1
            }
        }
        2 => {
            // read kernel space
            if let Some(ptr) = find_trace_info(id) {
                // SAFETY: 单线程环境且确保无其他代码修改 traces 数组
                unsafe { (*ptr).count as isize }
            } else {
                trace!("illegal syscall_id: {}", id);
                -1
            }
        }
        _ => -1,
    };
    ret
}

/// invoked by sys_mmap or sys_munmap, to check is the vitrual address range all unmapped or mapped.
fn check_vaddr_range(start: usize, end: usize, is_unmapped: bool) -> Result<(), ()> {
    let token = current_user_token();
    let page_table = PageTable::from_token(token);
    // Contact is allowed, but crossing is not allowed, [start, start + len)
    let start_vpn=VirtAddr::from(start).floor();
    let end_vpn = VirtAddr::from(end - 1).floor();
    match is_unmapped {
        true => {
            if let Some(pte) = page_table.translate(start_vpn){
                if pte.is_valid() {
                    return Err(());
                }
            } else if let Some(pte) = page_table.translate(end_vpn) {
                if pte.is_valid() {
                    return Err(());
                }
            }
        }
        false => {
            // if page_table.translate(start_vpn).is_none()
            //     || page_table.translate(end_vpn).is_none()
            // {
            //     return Err(());
            // }
            if let Some(pte) = page_table.translate(start_vpn){
                if !pte.is_valid() {
                    return Err(());
                }
            } else if let Some(pte) = page_table.translate(end_vpn) {
                if !pte.is_valid() {
                    return Err(());
                }
            }
        }
    }
    Ok(())
}
// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, prot: usize) -> isize {
    trace!("kernel: sys_mmap");
    if start & (PAGE_SIZE - 1) != 0 {
        return -1;
    }

    if (prot & !0x7) != 0 || (prot & 0x7) == 0 {
        return -1;
    }
    if len == 0 {
        return 0;
    }
    let mut flags = MapPermission::empty();
    if (prot & 0x1) != 0 {
        flags.insert(MapPermission::R);
    } // prot第0位 → R
    if (prot & 0x2) != 0 {
        flags.insert(MapPermission::W);
    } // prot第1位 → W
    if (prot & 0x4) != 0 {
        flags.insert(MapPermission::X);
    } // prot第2位 → X
    flags.insert(MapPermission::U);
    if check_vaddr_range(start, start + len, true).is_err() {
        return -1;
    }
    // let page_count = (len + PAGE_SIZE - 1) / PAGE_SIZE;
    insert_map_area(start, start + len, flags);
    0
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap");
    if start & (PAGE_SIZE - 1) != 0 {
        return -1;
    }
    if len == 0 {
        return 0;
    }
    if check_vaddr_range(start, start + len, false).is_err() {
        return -1;
    }
    if erase_map_area(start, start + len) {
        0
    } else {
        -1
    }
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
