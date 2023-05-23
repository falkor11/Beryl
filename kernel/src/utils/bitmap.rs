pub struct Bitmap<'a> {
    inner: &'a mut [u8],
}

impl<'a> Bitmap<'a> {
    pub fn new(inner: &'a mut [u8]) -> Bitmap<'a> {
        Bitmap { inner }
    }
}

impl Bitmap<'_> {
    pub fn test(&self, idx: usize) -> bool {
        (self.inner[idx / 8] & (1 << (idx % 8))) != 0
    }

    pub fn set(&mut self, idx: usize) {
        self.inner[idx / 8] |= 1 << (idx % 8);
    }

    pub fn unset(&mut self, idx: usize) {
        self.inner[idx / 8] &= !(1 << (idx % 8));
    }

    pub fn len(&self) -> usize {
        self.inner.len() * 8
    }
}
