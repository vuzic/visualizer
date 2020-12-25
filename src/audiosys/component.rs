use specs::{Component, DenseVecStorage};

#[derive(Debug)]
pub struct AudioIntensity {
    pub frame: usize,
    pub bucket: usize,
}

impl Component for AudioIntensity {
    type Storage = DenseVecStorage<Self>;
}
