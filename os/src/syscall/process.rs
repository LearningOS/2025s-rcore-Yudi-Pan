//! Process management syscalls
use crate::{
    task::{exit_current_and_run_next, suspend_current_and_run_next,find_trace_info},
    timer::get_time_us,
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// get time with second and microsecond
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    unsafe {
        *ts = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    }
    0
}

// trace various syscalls of current task(process)
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    let ret:isize = match trace_request {
        0=> {
            let byte = unsafe { *(id as *const u8) };
            byte as isize
        },
        1=>{
            unsafe { *(id as *mut u8) = data as u8};
            0
        },
        2=>{
        // 获取裸指针
        if let Some(ptr) = find_trace_info(id) {
            // SAFETY: 单线程环境且确保无其他代码修改 traces 数组
            unsafe {
            (*ptr).count as isize
            }
        } else {
            panic!("illegal syscall_id: {}", id)
        }
        },
        _=> -1
    };
    ret
}
