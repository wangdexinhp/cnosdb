use super::*;
use crate::direct_io::File;
use crate::file_manager;
use crate::FileSync;
use num_traits::ToPrimitive;
use std::borrow::Borrow;
use std::fs;
use std::path::PathBuf;

pub struct Writer {
    path: PathBuf,
    file: File,
}

impl Writer {
    pub fn new(path: &PathBuf) -> Self {
        let file = open_file(path).unwrap();
        Writer {
            path: path.clone(),
            file,
        }
    }

    pub async fn write_record(
        &mut self,
        data_version: u8,
        data_type: u8,
        data: &Vec<u8>,
    ) -> LogFileResult<()> {
        let mut buf = Vec::<u8>::with_capacity(
            RECORD_MAGIC_NUMBER_LEN
                + RECORD_DATA_SIZE_LEN
                + RECORD_DATA_VERSION_LEN
                + RECORD_DATA_TYPE_LEN
                + data.len()
                + RECORD_CRC32_NUMBER_LEN,
        );

        //build buf
        buf.append(&mut MAGIC_NUMBER.to_le_bytes().to_vec()); // magic_number
        buf.append(&mut data.len().to_u16().unwrap().to_le_bytes().to_vec()); //data_size
        buf.append(&mut data_version.to_le_bytes().to_vec()); //data_version
        buf.append(&mut data_type.to_le_bytes().to_vec()); //data_type
        buf.append(&mut data.to_vec()); //data
        buf.append(
            &mut crc32fast::hash(buf[RECORD_MAGIC_NUMBER_LEN..].borrow())
                .to_le_bytes()
                .to_vec(),
        ); // crc32_number

        //write file
        let mut p = 0;
        let mut pos = self.file.len();
        while p < buf.len() {
            let mut write_len = BLOCK_SIZE - pos.to_usize().unwrap() % BLOCK_SIZE;
            if write_len > buf.len() - p {
                write_len = buf.len() - p;
            }

            match self
                .file
                .write_at(pos, &buf[p..p + write_len])
                .map_err(|err| LogFileError::WriteFile { source: err })
            {
                Ok(_) => {
                    p += write_len;
                    pos += write_len.to_u64().unwrap();
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }

        Ok(())
    }

    pub async fn soft_sync(&self) -> LogFileResult<()> {
        self.file
            .sync_all(FileSync::Soft)
            .map_err(|err| LogFileError::SyncFile { source: err })
    }

    pub async fn hard_sync(&self) -> LogFileResult<()> {
        self.file
            .sync_all(FileSync::Hard)
            .map_err(|err| LogFileError::SyncFile { source: err })
    }

    pub async fn close(&mut self) -> LogFileResult<()> {
        self.file
            .sync_all(FileSync::Hard)
            .map_err(|err| LogFileError::SyncFile { source: err })
    }
}

impl From<&str> for Writer {
    fn from(path: &str) -> Self {
        Writer::new(&PathBuf::from(path))
    }
}

#[tokio::test]
async fn test_writer() -> Result<(), LogFileError> {
    let mut w = Writer::from("/tmp/test.log_file");
    for i in 0..10 {
        w.write_record(1, 1, &Vec::from("hello")).await?;
    }
    w.close().await?;
    Ok(())
}
