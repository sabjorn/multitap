#![no_std]
use core::ops::{Index, IndexMut};

pub trait Num: Copy + Send{
    fn default_value() -> Self;
}

impl Num for f32 {
    fn default_value() -> Self {
        0.0
    }
}

impl Num for i32 {
    fn default_value() -> Self {
        0
    }
}

pub struct ReadHead<T: Num> {
    buffer : * const [T],
    size : usize,
    head_position : usize,
}

unsafe impl<T: Num> Send for ReadHead<T> {}

impl<T: Num> ReadHead<T> {
    pub fn seek(&mut self, position: usize){
        self.head_position = position % self.size;
    }
}

impl<T: Num> Iterator for ReadHead<T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        let sample: T;
        unsafe {
            sample = (*self.buffer)[self.head_position];
        }
        self.head_position = (self.head_position + 1) % (self.size);

        Some(sample)
    }
}

impl<T: Num> Index<usize> for ReadHead<T> {
    type Output = T;
    fn index(&self, i: usize) -> &T {
        let current_position = i % self.size;
        unsafe {
            &(*self.buffer)[current_position]
        }
    }
}


pub struct WriteHead<T: Num, const N: usize> {
    buffer : [T; N],
    head_position : usize,
}

unsafe impl<T: Num, const N: usize> Send for WriteHead<T, N> {}

impl<T: Num, const N: usize> WriteHead<T, N> {
    pub fn new() -> WriteHead<T, N> {
        let buffer = [ T::default_value(); N];
        WriteHead {buffer, head_position: 0}
    }

    pub fn push(&mut self, element: T) {
        self.buffer[self.head_position] = element;
        self.increment();
    }
    
    pub fn increment(&mut self) {
        self.head_position = (self.head_position + 1) % self.buffer.len();
    }

    pub fn seek(&mut self, position: usize){
        self.head_position = if position > self.buffer.len() { 0 } else { position };
    }

    pub fn clear(&mut self) where T: Default {
        self.buffer.fill(T::default_value());
    }
    
    pub fn as_readhead(&self, delay_samples: usize) -> ReadHead<T> {
        ReadHead {buffer: self.buffer.as_slice(), size: self.buffer.len(), head_position: (self.buffer.len() - delay_samples) % self.buffer.len()}
    }
}


impl<T: Num, const N: usize> Iterator for WriteHead<T, N> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        let sample = self.buffer[self.head_position];
        self.increment();

        Some(sample)
    }
}

impl<T: Num, const N: usize> Index<usize> for WriteHead<T, N> {
    type Output = T;
    fn index(&self, i: usize) -> &T {
        let current_position = i % self.buffer.len();
        &self.buffer[current_position]
    }
}

impl<T: Num, const N: usize> IndexMut<usize> for WriteHead<T, N> {
    fn index_mut(&mut self, i: usize) -> &mut T {
        let current_position = i % self.buffer.len();
        &mut self.buffer[current_position]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn read_head_is_generic() {
        {
            let mut write_head = WriteHead::<i32, 1>::new();
            write_head.push(0);
        }
        {
            let mut write_head = WriteHead::<f32, 1>::new();
            write_head.push(0_f32);
        }
    }

    #[test]
    pub fn read_head_with_delay_output_equals_write_head() {
        let mut write_head = WriteHead::<i32, 5>::new();

        write_head.push(1);
       
        for n in 0..4 {
            let mut read_head = write_head.as_readhead(n);
            for j in 0..4 {
                let val = read_head.next().unwrap();
                if j == n {
                    assert_eq!(val, 1)
                }
            }
        }
    }

    #[test]
    pub fn multiple_read_head_with_delay_output_equals_write_head() {
        let mut write_head = WriteHead::<f32, 5>::new();

        write_head.push(1.0);
        
        for n in 0..4 {
            let mut read_head_1 = write_head.as_readhead(n);
            let mut read_head_2 = write_head.as_readhead(n+1);
            for j in 0..4 {
                let val_1 = read_head_1.next().unwrap();
                let val_2 = read_head_2.next().unwrap();
                if j == n {
                    assert_eq!(val_1, 1.0)
                }
                if j == (n + 1) {
                    assert_eq!(val_2, 1.0)
                }
            }
        }
    }
    
    #[test]
    pub fn read_head_is_circular() {
        let mut write_head = WriteHead::<f32, 2>::new();
        
        write_head.push(1.0);

        let mut read_head = write_head.as_readhead(0);
        
        assert_eq!(read_head.next().unwrap(), 1.0);
        assert_eq!(read_head.next().unwrap(), 0.0);
        assert_eq!(read_head.next().unwrap(), 1.0);
    }

    #[test]
    pub fn read_head_index_operator() {
        let mut write_head = WriteHead::<f32, 5>::new();

        for n in 0..4 {
            write_head.push(n as f32);
        }

        let read_head = write_head.as_readhead(0);
        for n in 0..4 {
            assert_eq!(read_head[n], n as f32);
        }
    }

    #[test]
    pub fn read_head_index_operator_is_circular() {
        let mut write_head = WriteHead::<f32, 2>::new();

        write_head.push(0.0);
        write_head.push(1.0);

        let read_head = write_head.as_readhead(0);
        assert_eq!(read_head[0], 0.0);
        assert_eq!(read_head[1], 1.0);
        assert_eq!(read_head[2], 0.0);
        assert_eq!(read_head[3], 1.0);
        assert_eq!(read_head[4], 0.0);
    }
    
    #[test]
    pub fn write_head_is_circular() {
        let mut write_head = WriteHead::<f32, 2>::new();
        
        write_head.push(0.0);
        write_head.push(0.0);
        write_head.push(1.0); // wraps around

        let mut read_head = write_head.as_readhead(0);
        
        assert_eq!(read_head.next().unwrap(), 1.0);
        assert_eq!(read_head.next().unwrap(), 0.0);
    }
    
    #[test]
    pub fn write_head_index_operator() {
        let mut write_head = WriteHead::<f32, 5>::new();
        
        for n in 0..4 {
            write_head[n] = n as f32;
        }

        let mut read_head = write_head.as_readhead(0);
        for n in 0..4 {
            assert_eq!(read_head.next().unwrap(), n as f32);
        }
    }

    #[test]
    pub fn write_head_index_operator_is_circular() {
        let mut write_head = WriteHead::<f32, 2>::new();
        
        write_head[0] = 0.0;
        write_head[1] = 1.0;
        write_head[2] = 2.0;
        write_head[3] = 3.0;

        let mut read_head = write_head.as_readhead(0);
        assert_eq!(read_head.next().unwrap(), 2.0);
        assert_eq!(read_head.next().unwrap(), 3.0);
    }

    #[test]
    pub fn moved_write_head_no_longer_valid() {
        let mut write_head = WriteHead::<f32, 2>::new();

        write_head[0] = 9.0;
        write_head[1] = 8.0;

        let mut read_head = write_head.as_readhead(0);

        let mut c = move || {
            write_head[0] = 0.0;
            write_head[1] = 1.0;
        };
        c();

        assert_eq!(read_head.next().unwrap(), 9.0);
        assert_eq!(read_head.next().unwrap(), 8.0);

    }
}
