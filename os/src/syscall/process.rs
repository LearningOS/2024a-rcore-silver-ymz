//! Process management syscalls
use crate::{
    config::{MAX_SYSCALL_NUM, PAGE_SIZE},
    mm::translated_byte_buffer,
    task::{
        change_program_brk, current_user_token, exit_current_and_run_next, mmap_page, munmap_page,
        suspend_current_and_run_next, TaskStatus,
    },
    timer::{get_time_ms, get_time_us},
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    status: TaskStatus,
    /// The numbers of syscall called by task
    syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    time: usize,
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
    let time = get_time_us();
    let results = TimeVal {
        sec: time / 1_000_000,
        usec: time % 1_000_000,
    };
    mem_out(ts as usize, results);
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info");
    let guard = crate::task::get_task_info();
    let task_info = guard.as_ref();
    let current_time = get_time_ms();
    let time = current_time - task_info.first_schedule_time.unwrap().get();
    let mut syscall_times = [0; MAX_SYSCALL_NUM];
    for (k, v) in task_info.syscall_times.iter() {
        syscall_times[*k as usize] = *v;
    }
    mem_out(
        ti as usize,
        TaskInfo {
            status: TaskStatus::Running,
            syscall_times,
            time,
        },
    );
    0
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    trace!("kernel: sys_mmap");
    if start % PAGE_SIZE != 0 {
        log::info!("sys_mmap: start not page aligned");
        return -1;
    }
    if len == 0 {
        return 0;
    }
    if port & !0b111 != 0 || port & 0b111 == 0 {
        log::info!("sys_mmap: invalid port");
        return -1;
    }
    if !mmap_page(start, len, port) {
        log::info!("sys_mmap: failed to mmap");
        return -1;
    }

    0
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap");
    if start % PAGE_SIZE != 0 {
        log::info!("sys_munmap: start not page aligned");
        return -1;
    }
    let len = (len + PAGE_SIZE - 1) / PAGE_SIZE * PAGE_SIZE;
    if !munmap_page(start, len) {
        log::info!("sys_munmap: failed to munmap");
        return -1;
    }
    0
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

fn bytes_of<T>(value: &T) -> &[u8] {
    unsafe {
        core::slice::from_raw_parts((value as *const T) as *const u8, core::mem::size_of::<T>())
    }
}

fn mem_out<T>(virtual_ptr: usize, value: T) {
    let mut results_bytes = bytes_of(&value);
    let bufs = translated_byte_buffer(current_user_token(), virtual_ptr as _, results_bytes.len());
    for buf in bufs {
        buf.copy_from_slice(&results_bytes[..buf.len()]);
        results_bytes = &results_bytes[buf.len()..];
    }
}
