#![no_std]
use core::cell::UnsafeCell;
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

pub struct Multitap<T: Num, const N: usize> {
    data: UnsafeCell<[T; N]>,
}

pub struct ReadHead<'a, T: Num, const N: usize> {
    buffer: &'a Multitap<T, N>,
    head_position : usize,
}

pub struct WriteHead<'a, T: Num, const N: usize> {
    buffer: &'a Multitap<T, N>,
    head_position : usize,
}

impl<T: Num, const N: usize> Multitap<T, N> 
where T: Default {
    pub fn new() -> Self {
        Multitap {
            data: UnsafeCell::new([T::default_value(); N]),
        }
    }

    pub fn from_buffer(data: [T; N]) -> Self {
        Multitap {
            data: UnsafeCell::new(data),
        }
    }

    pub fn from_slice(data: &mut [T]) -> Self {
            Multitap { 
                data: UnsafeCell::new(data.try_into().expect("Wrong size"))
            }
    }

    pub fn as_mut(&self) -> &mut [T; N] {
        unsafe { &mut *self.data.get() }
    }
    
    pub fn as_writehead(& self) -> WriteHead<T, N> {
        WriteHead {
            buffer: self,
            head_position: 0 
        }
    }

    pub fn as_readhead(&self, head_position: usize) -> ReadHead<T, N> {
        ReadHead {
            buffer: self,
            head_position: (N - head_position) % N,
        }
    }
}

impl<T: Num, const N: usize> From<[T; N]> for Multitap<T, N>
where T: Default
{
    fn from(data: [T; N]) -> Self {
        Multitap {
            data: UnsafeCell::new(data),
        }
    }
}

unsafe impl<'a, T: Num, const N: usize> Send for ReadHead<'a, T, N> {}

impl<'a, T: Num, const N: usize> ReadHead<'a, T, N> {
    pub fn seek(&mut self, position: usize){
        self.head_position = position % N;
    }
}

impl<'a, T: Num, const N: usize> Iterator for ReadHead<'a, T, N> 
where T: Default {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        let sample = self.buffer.as_mut()[self.head_position];
        self.head_position = (self.head_position + 1) % N;

        Some(sample)
    }
}

impl<'a, T: Num, const N: usize> Index<usize> for ReadHead<'a, T, N> 
where T: Default{
    type Output = T;
    fn index(&self, i: usize) -> &T {
        let current_position = (self.head_position + i) % N;
        &self.buffer.as_mut()[current_position]
    }
}

unsafe impl<'a, T: Num, const N: usize> Send for WriteHead<'a, T, N> {}

impl<'a, T: Num, const N: usize> WriteHead<'a, T, N>
where T: Default {
    pub fn push(&mut self, element: T) {
        let buffer = self.buffer.as_mut();
        buffer[self.head_position] = element;
        self.increment();
    }
    
    pub fn increment(&mut self) {
        self.head_position = (self.head_position + 1) % N;
    }
//
//    pub fn seek(&mut self, position: usize){
//        self.head_position = if position > self.buffer.len() { 0 } else { position };
//    }
//
//    pub fn clear(&mut self) where T: Default {
//        self.buffer.fill(T::default_value());
//    }
}

impl<'a, T: Num, const N: usize> Index<usize> for WriteHead<'a, T, N> 
where T: Default{
    type Output = T;
    fn index(&self, i: usize) -> &T {
        let current_position = (self.head_position + i) % N;
        &self.buffer.as_mut()[current_position]
    }
}

impl<'a, T: Num, const N: usize> IndexMut<usize> for WriteHead<'a, T, N> 
where T: Default {
    fn index_mut(&mut self, i: usize) -> &mut T {
        let current_position = i % N;
        &mut self.buffer.as_mut()[current_position]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn readhead_with_delay_output_equals_write_head() {
        let multitap = Multitap::<f32, 3>::new();
        let mut writehead = multitap.as_writehead();

        writehead.push(1.0);
        writehead.push(2.0);
        writehead.push(3.0);

        {
            let mut readhead = multitap.as_readhead(0);
            assert_eq!(readhead.next().unwrap(), 1.0);
            assert_eq!(readhead.next().unwrap(), 2.0);
            assert_eq!(readhead.next().unwrap(), 3.0);
        }
        
        {
            let mut readhead = multitap.as_readhead(1);
            assert_eq!(readhead.next().unwrap(), 3.0);
            assert_eq!(readhead.next().unwrap(), 1.0);
            assert_eq!(readhead.next().unwrap(), 2.0);
        }
        
        {
            let mut readhead = multitap.as_readhead(2);
            assert_eq!(readhead.next().unwrap(), 2.0);
            assert_eq!(readhead.next().unwrap(), 3.0);
            assert_eq!(readhead.next().unwrap(), 1.0);
        }
    }

    #[test]
    pub fn multiple_readhead_with_delay_output_equals_write_head() {
        let multitap = Multitap::<f32, 5>::new();
        let mut writehead = multitap.as_writehead();

        writehead.push(1.0);
        for n in 0..4 {
            let mut readhead_1 = multitap.as_readhead(n);
            let mut readhead_2 = multitap.as_readhead(n+1);
            for j in 0..4 {
                let val_1 = readhead_1.next().unwrap();
                let val_2 = readhead_2.next().unwrap();
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
    pub fn readhead_is_circular() {
        let multitap = Multitap::<f32, 3>::new();
        let mut writehead = multitap.as_writehead();
        
        writehead.push(1.0);

        let mut readhead = multitap.as_readhead(0);
        
        assert_eq!(readhead.next().unwrap(), 1.0);
        assert_eq!(readhead.next().unwrap(), 0.0);
        assert_eq!(readhead.next().unwrap(), 0.0);
        assert_eq!(readhead.next().unwrap(), 1.0);
    }

    #[test]
    pub fn readhead_index_operator() {
        let multitap = Multitap::<f32, 3>::new();
        let mut writehead = multitap.as_writehead();

        writehead.push(0.);
        writehead.push(1.);
        writehead.push(2.);

        let readhead = multitap.as_readhead(1);
        assert_eq!(readhead[0], 2.);
        assert_eq!(readhead[1], 0.);
        assert_eq!(readhead[2], 1.);
    }

    #[test]
    pub fn readhead_index_operator_is_circular() {
        let multitap = Multitap::<f32, 3>::new();
        let mut writehead = multitap.as_writehead();

        writehead.push(0.0);
        writehead.push(1.0);
        writehead.push(2.0);

        let readhead = multitap.as_readhead(1);
        assert_eq!(readhead[0], 2.0);
        assert_eq!(readhead[1], 0.0);
        assert_eq!(readhead[2], 1.0);
        assert_eq!(readhead[3], 2.0);
        assert_eq!(readhead[4], 0.0);
        assert_eq!(readhead[5], 1.0);
        assert_eq!(readhead[6], 2.0);
    }

    #[test]
    pub fn readhead_seak_is_cyclical() {
        let multitap = Multitap::<f32, 3>::new();
        let mut writehead = multitap.as_writehead();

        writehead.push(0.0);
        writehead.push(1.0);
        writehead.push(2.0);

        let mut readhead = multitap.as_readhead(0);
        readhead.seek(3);
        assert_eq!(readhead.next().unwrap(), 0.0);
    }

    #[test]
    pub fn writehead_is_circular() {
        let multitap = Multitap::<f32, 2>::new();
        let mut writehead = multitap.as_writehead();
        
        writehead.push(1.0);
        writehead.push(2.0);
        writehead.push(3.0); // wraps around

        let mut readhead = multitap.as_readhead(0);
        
        assert_eq!(readhead.next().unwrap(), 3.0);
        assert_eq!(readhead.next().unwrap(), 2.0);
    }
    
    #[test]
    pub fn writehead_index_operator() {
        let multitap = Multitap::<f32, 5>::new();
        let mut writehead = multitap.as_writehead();
        
        for n in 0..4 {
            writehead[n] = n as f32;
        }

        let mut readhead = multitap.as_readhead(0);
        for n in 0..4 {
            assert_eq!(readhead.next().unwrap(), n as f32);
        }
    }

    #[test]
    pub fn writehead_index_operator_is_circular() {
        let multitap = Multitap::<f32, 2>::new();
        let mut writehead = multitap.as_writehead();
        
        writehead[0] = 0.0;
        writehead[1] = 1.0;
        writehead[2] = 2.0;
        writehead[3] = 3.0;

        let mut readhead = multitap.as_readhead(0);
        assert_eq!(readhead.next().unwrap(), 2.0);
        assert_eq!(readhead.next().unwrap(), 3.0);
    }

    #[test]
    pub fn moved_writehead_is_valid() {
        let multitap = Multitap::<f32, 2>::new();
        let mut writehead = multitap.as_writehead();

        writehead[0] = 9.0;
        writehead[1] = 8.0;

        let mut readhead = multitap.as_readhead(0);

        (move || {
            writehead[0] = 0.0;
            writehead[1] = 1.0;
        })();

        assert_eq!(readhead.next().unwrap(), 0.0);
        assert_eq!(readhead.next().unwrap(), 1.0);

    }

    #[test]
    pub fn from_external_buffer() {
        let array: [f32; 3] = [0.; 3];
        let multitap = Multitap::from(array);

        let mut writehead = multitap.as_writehead();

        writehead[0] = 1.0;
        writehead[1] = 2.0;
        writehead[2] = 3.0;

        let mut readhead = multitap.as_readhead(0);

        assert_eq!(readhead.next().unwrap(), 1.0);
        assert_eq!(readhead.next().unwrap(), 2.0);
        assert_eq!(readhead.next().unwrap(), 3.0);
    }

    #[test]
    pub fn external_buffer_into() {
        let array: [f32; 3] = [0.; 3];
        let multitap: Multitap<f32, 3> = array.into();

        let mut writehead = multitap.as_writehead();

        writehead[0] = 1.0;
        writehead[1] = 2.0;
        writehead[2] = 3.0;

        let mut readhead = multitap.as_readhead(0);

        assert_eq!(readhead.next().unwrap(), 1.0);
        assert_eq!(readhead.next().unwrap(), 2.0);
        assert_eq!(readhead.next().unwrap(), 3.0);
    }
    
    #[test]
    pub fn from_slice() {
        let mut array: [f32; 3] = [0.; 3];
        let multitap: Multitap<f32, 3> = Multitap::from_slice(array.as_mut_slice());

        let mut writehead = multitap.as_writehead();

        writehead[0] = 1.0;
        writehead[1] = 2.0;
        writehead[2] = 3.0;

        let mut readhead = multitap.as_readhead(0);

        assert_eq!(readhead.next().unwrap(), 1.0);
        assert_eq!(readhead.next().unwrap(), 2.0);
        assert_eq!(readhead.next().unwrap(), 3.0);
    }
}
