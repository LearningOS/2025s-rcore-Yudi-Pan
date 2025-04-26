//! Process management syscalls
use crate::mm::{write_struct_to_user, PageTable, VirtAddr};
use crate::task::{
    change_program_brk, current_user_token, exit_current_and_run_next, find_trace_info,
    suspend_current_and_run_next,
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

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    -1
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    -1
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
