use std::path::{Path, PathBuf};
use std::fs;
use std::io;
use std::io::{Read, Write};
use byteorder::{BigEndian, ReadBytesExt};

pub trait VarIntegerReader {
    fn read_var_i32(&mut self) -> io::Result<i32>;
    fn read_var_i64(&mut self) -> io::Result<i64>;
}

impl <T: Read + ?Sized> VarIntegerReader for T {

    fn read_var_i32(&mut self) -> io::Result<i32> {
        let mut num_read: i32 = 0;
        let mut result: i32 = 0;
        loop{
            let read = self.read_u8()?;
            let value: i32 = read as i32 & 0b01111111;
            result |= value << (7 * num_read);
            num_read += 1;
            if num_read > 5 {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "VarInt is too big"))
            } else if (read & 0b10000000) != 0 {break}
        }
        Ok(result)
    }

    fn read_var_i64(&mut self) -> io::Result<i64> {
        let mut num_read: i32 = 0;
        let mut result: i64 = 0;
        loop {
            let read = self.read_u8()?;
            let value: i64 = (read as i32 & 0b01111111) as i64;
            result |= value << (7 * num_read);
            num_read += 1;
            if num_read > 10 {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "VarLong is too big"))
            } else if (read & 0b10000000) != 0 { break }
        }
        Ok(result)
    }
}


pub fn copy<U: AsRef<Path>, V: AsRef<Path>>(from: U, to: V) -> Result<(), std::io::Error> {
    let mut stack = Vec::new();
    stack.push(PathBuf::from(from.as_ref()));

    let output_root = PathBuf::from(to.as_ref());
    let input_root = PathBuf::from(from.as_ref()).components().count();

    while let Some(working_path) = stack.pop() {
        println!("process: {:?}", &working_path);

        // Generate a relative path
        let src: PathBuf = working_path.components().skip(input_root).collect();

        // Create a destination if missing
        let dest = if src.components().count() == 0 {
            output_root.clone()
        } else {
            output_root.join(&src)
        };
        if fs::metadata(&dest).is_err() {
            println!(" mkdir: {:?}", dest);
            fs::create_dir_all(&dest)?;
        }

        for entry in fs::read_dir(working_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                match path.file_name() {
                    Some(filename) => {
                        let dest_path = dest.join(filename);
                        println!("  copy: {:?} -> {:?}", &path, &dest_path);
                        fs::copy(&path, &dest_path)?;
                    }
                    None => {
                        println!("failed: {:?}", path);
                    }
                }
            }
        }
    }

    Ok(())
}
