

struct Task {
    registers: [u64;16],
    pagemap: u64,
    running: bool
}

static mut TASKS: LinkedList<Task> = LinkedList::new();

impl Task {
}