use core::{cell::RefMut, num::NonZeroUsize};

use crate::{
    config::{MAX_APP_NUM, MAX_SYSCALL_NUM},
    sync::UPSafeCell,
    timer::get_time_ms,
};
use alloc::collections::btree_map::BTreeMap;
use lazy_static::lazy_static;

use super::current_task_id;

/// Task information, used for `sys_task_info`
pub struct TaskInfo {
    /// The first time the task is scheduled
    pub first_schedule_time: Option<NonZeroUsize>,
    /// The number of times each syscall is called
    pub syscall_times: BTreeMap<u32, u32>,
}

/// Task information manager
pub struct TaskInfoManager {
    task_infos: UPSafeCell<[TaskInfo; MAX_APP_NUM]>,
}

/// Guard for `TaskInfo`
pub struct TaskInfoGuard<'a> {
    guard: RefMut<'a, [TaskInfo; MAX_APP_NUM]>,
    task_id: usize,
}

lazy_static! {
    /// Global variable: TASK_INFOS
    static ref TASK_INFO_MANAGER: TaskInfoManager = {
        assert!(MAX_SYSCALL_NUM < u32::MAX as usize);
        TaskInfoManager {
            task_infos: unsafe { UPSafeCell::new(core::mem::zeroed()) },
        }
    };
}

impl TaskInfoManager {
    fn get_task_info(&self, task_id: usize) -> TaskInfoGuard {
        TaskInfoGuard {
            guard: self.task_infos.exclusive_access(),
            task_id,
        }
    }

    fn set_first_schedule_time(&self, task_id: usize, time: usize) {
        let mut guard = self.task_infos.exclusive_access();
        if guard[task_id].first_schedule_time.is_none() {
            guard[task_id].first_schedule_time = Some(NonZeroUsize::new(time).unwrap());
        }
    }

    fn inc_syscall_times(&self, task_id: usize, syscall_id: usize) {
        let mut guard = self.task_infos.exclusive_access();
        guard[task_id]
            .syscall_times
            .entry(syscall_id as u32)
            .and_modify(|e| *e += 1)
            .or_insert(1);
    }
}

impl AsRef<TaskInfo> for TaskInfoGuard<'_> {
    fn as_ref(&self) -> &TaskInfo {
        &self.guard[self.task_id]
    }
}

/// Get the task information of the task with `task_id`
pub fn get_task_info<'a>() -> TaskInfoGuard<'a> {
    TASK_INFO_MANAGER.get_task_info(current_task_id())
}

/// Set the first schedule time of the task with `task_id`
pub fn set_first_schedule_time() {
    TASK_INFO_MANAGER.set_first_schedule_time(current_task_id(), get_time_ms());
}

/// Increase the syscall times of the task with `task_id` and `syscall_id`
pub fn inc_syscall_times(syscall_id: usize) {
    TASK_INFO_MANAGER.inc_syscall_times(current_task_id(), syscall_id);
}
