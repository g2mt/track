use std::fs::File;

pub trait Truncate {
    fn set_len(&self, size: u64) -> std::io::Result<()>;
}

impl Truncate for File {
    fn set_len(&self, size: u64) -> std::io::Result<()> {
        File::set_len(&self, size)
    }
}
