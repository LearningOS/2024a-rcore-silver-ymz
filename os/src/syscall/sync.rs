use crate::sync::{Condvar, Mutex, MutexBlocking, MutexSpin, Semaphore};
use crate::task::{block_current_and_run_next, current_process, current_task, DeadLockDetect};
use crate::timer::{add_timer, get_time_ms};
use alloc::sync::Arc;
/// sleep syscall
pub fn sys_sleep(ms: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_sleep",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let expire_ms = get_time_ms() + ms;
    let task = current_task().unwrap();
    add_timer(expire_ms, task);
    block_current_and_run_next();
    0
}
/// mutex create syscall
pub fn sys_mutex_create(blocking: bool) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mutex: Arc<dyn Mutex> = if !blocking {
        Arc::new(MutexSpin::new())
    } else {
        Arc::new(MutexBlocking::new())
    };
    let mut process_inner = process.inner_exclusive_access();
    if let Some((detect, _)) = process_inner.deadlock_detect.as_mut() {
        detect.available.push(1);
        detect.allocation.iter_mut().for_each(|v| v.push(0));
        detect.need.iter_mut().for_each(|v| v.push(0));
    }
    process_inner.mutex_list.push(mutex);
    process_inner.mutex_list.len() as isize - 1
}
/// mutex lock syscall
pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_lock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    if let Some((detect, _)) = process_inner.deadlock_detect.as_mut() {
        detect.need[tid][mutex_id] += 1;
        if detect.detect() {
            detect.need[tid][mutex_id] -= 1;
            return -0xdead;
        }
    }
    let mutex = process_inner.mutex_list[mutex_id].clone();
    drop(process_inner);

    mutex.lock();

    let mut process_inner = process.inner_exclusive_access();
    if let Some((detect, _)) = process_inner.deadlock_detect.as_mut() {
        detect.need[tid][mutex_id] -= 1;
        detect.allocation[tid][mutex_id] += 1;
        detect.available[mutex_id] -= 1;
    }

    0
}
/// mutex unlock syscall
pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_unlock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    if let Some((detect, _)) = process_inner.deadlock_detect.as_mut() {
        detect.allocation[tid][mutex_id] -= 1;
        detect.available[mutex_id] += 1;
    }
    let mutex = process_inner.mutex_list[mutex_id].clone();
    drop(process_inner);
    drop(process);
    mutex.unlock();
    0
}
/// semaphore create syscall
pub fn sys_semaphore_create(res_count: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    if let Some((_, detect)) = process_inner.deadlock_detect.as_mut() {
        detect.available.push(res_count as u32);
        detect.allocation.iter_mut().for_each(|v| v.push(0));
        detect.need.iter_mut().for_each(|v| v.push(0));
    }
    process_inner
        .semaphore_list
        .push(Arc::new(Semaphore::new(res_count)));
    process_inner.semaphore_list.len() as isize - 1
}
/// semaphore up syscall
pub fn sys_semaphore_up(sem_id: usize) -> isize {
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_up",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    if let Some((_, detect)) = process_inner.deadlock_detect.as_mut() {
        detect.allocation[tid][sem_id] -= 1;
        detect.available[sem_id] += 1;
    }
    let sem = process_inner.semaphore_list[sem_id].clone();
    drop(process_inner);
    sem.up();
    0
}
/// semaphore down syscall
pub fn sys_semaphore_down(sem_id: usize) -> isize {
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_down",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    if let Some((_, detect)) = process_inner.deadlock_detect.as_mut() {
        detect.need[tid][sem_id] += 1;
        if detect.detect() {
            detect.need[tid][sem_id] -= 1;
            return -0xdead;
        }
    }
    let sem = process_inner.semaphore_list[sem_id].clone();
    drop(process_inner);

    sem.down();

    let mut process_inner = process.inner_exclusive_access();
    if let Some((_, detect)) = process_inner.deadlock_detect.as_mut() {
        detect.need[tid][sem_id] -= 1;
        detect.allocation[tid][sem_id] += 1;
        detect.available[sem_id] -= 1;
    }

    0
}
/// condvar create syscall
pub fn sys_condvar_create() -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .condvar_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.condvar_list[id] = Some(Arc::new(Condvar::new()));
        id
    } else {
        process_inner
            .condvar_list
            .push(Some(Arc::new(Condvar::new())));
        process_inner.condvar_list.len() - 1
    };
    id as isize
}
/// condvar signal syscall
pub fn sys_condvar_signal(condvar_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_signal",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    drop(process_inner);
    condvar.signal();
    0
}
/// condvar wait syscall
pub fn sys_condvar_wait(condvar_id: usize, mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_wait",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    let mutex = process_inner.mutex_list[mutex_id].clone();
    drop(process_inner);
    condvar.wait(mutex);
    0
}
/// enable deadlock detection syscall
///
/// YOUR JOB: Implement deadlock detection, but might not all in this syscall
pub fn sys_enable_deadlock_detect(enabled: usize) -> isize {
    trace!("kernel: sys_enable_deadlock_detect");
    let enabled = match enabled {
        0 => false,
        1 => true,
        _ => return -1,
    };
    let process = current_process();
    let mut inner = process.inner_exclusive_access();
    if !inner.mutex_list.is_empty() || !inner.semaphore_list.is_empty() {
        return -1;
    }
    let threads = inner.tasks.len() as u32;
    if enabled {
        inner.deadlock_detect = Some((DeadLockDetect::new(threads), DeadLockDetect::new(threads)));
    } else {
        inner.deadlock_detect = None;
    }
    0
}
