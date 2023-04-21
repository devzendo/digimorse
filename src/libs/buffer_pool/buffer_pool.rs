use std::sync::{Arc, RwLock};
use log::{debug, error};

struct Buffer {
    samples: Arc<RwLock<Vec<f32>>>,
    in_use: bool,
}

pub struct BufferPool {
    buffers: Vec<Buffer>
}

impl BufferPool {
    pub fn new(buffer_size: usize, number_of_buffers: usize) -> Self {
        let mut buffers: Vec<Buffer> = Vec::with_capacity(number_of_buffers);
        (0 .. number_of_buffers).for_each(|_| {
            let mut vec = Vec::with_capacity(buffer_size);
            vec.resize(buffer_size, 0_f32);
            buffers.push(Buffer { samples: Arc::new(RwLock::new(vec)), in_use: false });
        });
        Self {
            buffers,
        }
    }

    pub fn allocate(&mut self) -> Option<(usize, Arc<RwLock<Vec<f32>>>)> {
        for index in 0 .. self.buffers.len() {
            if !self.buffers[index].in_use {
                debug!("Allocated buffer {}", index);
                self.buffers[index].in_use = true;
                return Some( (index, self.buffers[index].samples.clone()) );
            }
        }

        error!("BufferPool exhausted!");
        None
    }

    pub fn free(&mut self, index: usize) -> bool {
        if self.buffers[index].in_use {
            self.buffers[index].in_use = false;
            debug!("Freed buffer {}", index);
            true
        } else {
            error!("Double free of buffer {}", index);
            false
        }
    }
}

#[cfg(test)]
#[path = "./buffer_pool_spec.rs"]
mod buffer_pool_spec;
