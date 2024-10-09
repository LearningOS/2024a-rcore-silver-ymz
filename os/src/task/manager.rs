//!Implementation of [`TaskManager`]
use super::TaskControlBlock;
use crate::sync::UPSafeCell;
use alloc::collections::binary_heap::BinaryHeap;
use alloc::sync::Arc;
use lazy_static::*;

struct TCBCmp(Arc<TaskControlBlock>);

impl PartialEq for TCBCmp {
    fn eq(&self, other: &Self) -> bool {
        self.0.inner_exclusive_access().stride == other.0.inner_exclusive_access().stride
    }
}

impl Eq for TCBCmp {}

impl PartialOrd for TCBCmp {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TCBCmp {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.0
            .inner_exclusive_access()
            .stride
            .cmp(&other.0.inner_exclusive_access().stride)
            .reverse()
    }
}

/// A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    ready_queue: BinaryHeap<TCBCmp>,
}

/// A simple FIFO scheduler.
impl TaskManager {
    ///Creat an empty TaskManager
    pub fn new() -> Self {
        Self {
            ready_queue: BinaryHeap::new(),
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push(TCBCmp(task));
    }
    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        let res = self.ready_queue.pop()?.0;
        let mut inner = res.inner_exclusive_access();
        inner.stride += inner.pass;
        drop(inner);
        Some(res)
    }
}

lazy_static! {
    /// TASK_MANAGER instance through lazy_static!
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

/// Add process to ready queue
pub fn add_task(task: Arc<TaskControlBlock>) {
    //trace!("kernel: TaskManager::add_task");
    TASK_MANAGER.exclusive_access().add(task);
}

/// Take a process out of the ready queue
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    //trace!("kernel: TaskManager::fetch_task");
    TASK_MANAGER.exclusive_access().fetch()
}
