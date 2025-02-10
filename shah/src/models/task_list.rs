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
