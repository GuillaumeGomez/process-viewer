use std::ops::Index;

#[derive(Debug)]
pub struct RotateVec<T> {
    data: Vec<T>,
    start: usize,
}

impl<T> RotateVec<T> {
    pub fn new(d: Vec<T>) -> RotateVec<T> {
        RotateVec {
            data: d,
            start: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn move_start(&mut self) {
        if self.start > 0 {
            self.start -= 1;
        } else {
            self.start = self.data.len() - 1;
        }
    }

    /*pub fn get(&self, index: usize) -> Option<&T> {
        self.data.get(self.get_real_pos(index))
    }*/

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        let pos = self.get_real_pos(index);
        self.data.get_mut(pos)
    }

    fn get_real_pos(&self, index: usize) -> usize {
        if self.start + index >= self.data.len() {
            self.start + index - self.data.len()
        } else {
            index + self.start
        }
    }
}

impl<T> Index<usize> for RotateVec<T> {
    type Output = T;

    fn index<'a>(&'a self, index: usize) -> &'a T {
        &self.data[self.get_real_pos(index)]
    }
}
