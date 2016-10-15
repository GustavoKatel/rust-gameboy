
pub struct GBMem {
    map: Vec<u8>,
}

impl GBMem {

    pub fn new() -> GBMem {
        GBMem{
            map: vec![0; 1024 * 64], // 64KB
        }
    }

    pub fn put(&mut self, pos: usize, byte: u8) {
        self.map[pos] = byte;
    }

    pub fn get(&self, pos: usize) -> u8 {
        self.map[pos].clone()
    }

}
