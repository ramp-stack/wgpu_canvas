use wgpu::{COPY_BUFFER_ALIGNMENT, BufferDescriptor, BufferAddress, BufferUsages, Buffer, Device, Queue,};

pub struct DynamicBufferDescriptor<'a> {
    pub label: Option<&'a str>,
    pub usage: BufferUsages,
}

pub struct DynamicBuffer {
    buffer: Buffer,
    size: BufferAddress,
    label: Option<String>
}

impl DynamicBuffer {
    pub fn new(device: &Device, descriptor: &DynamicBufferDescriptor) -> Self {
        let size = 4096;
        DynamicBuffer{
            buffer: device.create_buffer(&BufferDescriptor{
                label: descriptor.label,
                size,
                usage: descriptor.usage,
                mapped_at_creation: false
            }),
            size,
            label: descriptor.label.map(|s| s.to_string())
        }
    }

    pub fn write_buffer(&mut self, device: &Device, queue: &Queue, contents: &[u8]) {
        let pad: usize = contents.len() % 4;
        let contents = if pad != 0 {
            &[contents, &vec![0u8; pad]].concat()
        } else {contents};

        if self.size >= contents.len() as u64 {
            queue.write_buffer(&self.buffer, 0, contents);
        } else {
            let size = Self::next_copy_buffer_size(contents.len() as u64);
            self.buffer = device.create_buffer(&BufferDescriptor {
                label: self.label.as_deref(),
                size,
                usage: self.buffer.usage(),
                mapped_at_creation: true,
            });
            self.buffer.slice(..).get_mapped_range_mut()[..contents.len()].copy_from_slice(contents);
            self.buffer.unmap();
            self.size = size;
        }
    }

    fn next_copy_buffer_size(size: u64) -> u64 {
        let align_mask = COPY_BUFFER_ALIGNMENT - 1;
        ((size.next_power_of_two() + align_mask) & !align_mask).max(COPY_BUFFER_ALIGNMENT)
    }
}

impl AsRef<Buffer> for DynamicBuffer {
    fn as_ref(&self) -> &Buffer {
        &self.buffer
    }
}
