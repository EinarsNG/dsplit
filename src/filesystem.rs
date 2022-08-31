use std::fs::{copy, rename, remove_file};
use std::io::Error;
use std::path::Path;

pub trait FileHandler
{
    fn rename<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> Result<(), Error>
    {
        rename(from, to)
    }

    fn copy<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> Result<u64, Error>
    {
        copy(from, to)
    }

    fn remove_file<P: AsRef<Path>>(path: P) -> Result<(), Error>
    {
        remove_file(path)
    }
}
pub struct FileHandlerMain;
impl FileHandler for FileHandlerMain{}
