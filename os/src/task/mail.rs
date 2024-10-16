use crate::mm::UserBuffer;

pub struct MailBox {
    buffer: [[u8; 256]; 16],
    buffer_len: [u8; 16],
    head: u8,
    tail: u8,
}

impl MailBox {
    pub fn new() -> Self {
        MailBox {
            buffer: [[0; 256]; 16],
            buffer_len: [0; 16],
            head: 0,
            tail: 0,
        }
    }

    pub fn write(&mut self, data: UserBuffer) -> bool {
        let len = data.len();
        assert!(len <= 256);
        if self.next(self.head) == self.tail {
            return false;
        }
        if len == 0 {
            return true;
        }

        let mut results_bytes = &mut self.buffer[self.head as usize][..];
        for buf in data.buffers {
            results_bytes[..buf.len()].copy_from_slice(buf);
            results_bytes = &mut results_bytes[buf.len()..];
        }
        self.buffer_len[self.head as usize] = len as u8;
        self.head = self.next(self.head);
        true
    }

    pub fn read(&mut self, data: UserBuffer) -> isize {
        let len = data.len();
        assert!(len <= 256);
        if self.head == self.tail {
            return -1;
        }
        if len == 0 {
            return 0;
        }
        let len = len.min(self.buffer_len[self.tail as usize] as usize);

        let mut results_bytes = &self.buffer[self.tail as usize][..len];
        for buf in data.buffers {
            let size = buf.len().min(results_bytes.len());
            buf[..size].copy_from_slice(&results_bytes[..size]);
            results_bytes = &results_bytes[size..];
            if results_bytes.is_empty() {
                break;
            }
        }
        self.tail = self.next(self.tail);
        len as isize
    }

    fn next(&self, pos: u8) -> u8 {
        (pos + 1) % 16
    }
}
