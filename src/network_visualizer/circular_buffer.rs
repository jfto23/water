//taken from https://github.com/lucaspoffo/renet/blob/master/renet_visualizer/src/circular_buffer.rs
#[derive(Debug)]
pub struct CircularBuffer<const N: usize, T> {
    pub(crate) queue: [T; N],
    cursor: usize,
}

impl<const N: usize, T: Default + Copy> Default for CircularBuffer<N, T> {
    fn default() -> Self {
        Self {
            queue: [T::default(); N],
            cursor: 0,
        }
    }
}

impl<const N: usize, T: Default + Copy> CircularBuffer<N, T> {
    pub fn push(&mut self, value: T) {
        self.queue[self.cursor] = value;
        self.cursor = (self.cursor + 1) % N;
    }

    pub fn as_vec(&self) -> Vec<T> {
        let (end, start) = self.queue.split_at(self.cursor);
        let mut vec = Vec::with_capacity(N);
        vec.extend_from_slice(start);
        vec.extend_from_slice(end);

        vec
    }
}
