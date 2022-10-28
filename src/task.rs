use core::ptr::NonNull;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct TaskContext {
    rsp:usize,
    cr3:usize
}
pub struct Task {
    regs: [usize;16],
    ctx: TaskContext,
    pid: isize,
    next: NonNull<Task>,
    prev: NonNull<Task>
}

static mut NEXT_PID: isize = 0;
static mut CURRENT_TASK: Option<NonNull<Task>> = None;

fn next_pid() -> isize {
    unsafe {
        let pid = NEXT_PID;
        NEXT_PID += 1;
        pid
    }
}

impl Task {
    fn new(mut addr: NonNull<Task>, mut next: NonNull<Task>, mut prev: NonNull<Task>, ctx: TaskContext) -> NonNull<Task> {
        unsafe {
            let mut task = addr.as_mut();
            task.ctx = ctx;
            task.regs = [0,0,0,0,0,0,ctx.rsp,0,0,0,0,0,0,0,0,0];
            task.pid = next_pid();
            task.next = next;
            next.as_mut().prev = prev;
            task.prev = prev;
            prev.as_mut().next = next;
            addr
        }
    }
}

pub extern "C" fn switch_task(rsp: usize, cr3: usize) -> TaskContext {
    unsafe {
        if let Some(mut current_task) = CURRENT_TASK {
            current_task.as_mut().ctx.rsp = rsp;
            current_task.as_mut().ctx.cr3 = cr3;
            CURRENT_TASK = Some(current_task.as_mut().next);
            CURRENT_TASK.unwrap().as_mut().ctx
        } else {
            TaskContext {rsp, cr3}
        }
    }
}