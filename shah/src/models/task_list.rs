use crate::ShahError;

/// if an io operation was performed check for order's
/// if no io operation's was performed then run another task
pub struct Performed(pub bool);
pub type Task<T> = fn(&mut T) -> Result<Performed, ShahError>;

#[derive(Debug)]
pub struct TaskList<const N: usize, T> {
    tasks: [T; N],
    index: usize,
    count: usize,
}

impl<const N: usize, T: Copy> TaskList<N, T> {
    pub fn new(tasks: [T; N]) -> Self {
        Self { tasks, index: 0, count: 0 }
    }
    pub fn start(&mut self) {
        self.count = 0;
    }
}

impl<const N: usize, T: Copy> Iterator for TaskList<N, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.count >= N {
            return None;
        }
        self.count += 1;

        if self.index >= N {
            self.index = 0;
        }

        let task = Some(self.tasks[self.index]);

        self.index += 1;
        if self.index >= N {
            self.index = 0;
        }

        task
    }
}

pub trait Worker<const N: usize> {
    fn tasks(&mut self) -> &mut TaskList<N, Task<Self>>;
    fn work(&mut self) -> Result<Performed, ShahError> {
        self.tasks().start();
        while let Some(task) = self.tasks().next() {
            if task(self)?.0 {
                return Ok(Performed(true));
            }
        }
        Ok(Performed(false))
    }
}
