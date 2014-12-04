// Copyright (C) 2014  Josh Stone
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use fuse;
use git2;
use libc;
use libc::consts::os::posix88;
use std::io;

use inode;

/// Git blobs are represented as files
// FIXME needs context, e.g. permissions from TreeEntry and timestamps from Commit
pub struct Blob {
    oid: git2::Oid,
    size: u64,
    data: Option<Vec<u8>>,
}

impl Blob {
    pub fn new(blob: git2::Blob) -> Box<inode::Inode> {
        box Blob {
            oid: blob.id(),
            size: blob.content().len() as u64,
            data: None,
        }
    }
}

impl inode::Inode for Blob {
    fn getattr(&mut self, _repo: &git2::Repository, attr: inode::FileAttr
              ) -> Result<inode::FileAttr, libc::c_int> {
        Ok(inode::FileAttr {
            size: self.size,
            blocks: inode::st_blocks(self.size),
            kind: io::FileType::RegularFile,
            perm: io::USER_FILE,
            ..attr
        })
    }

    fn open(&mut self, repo: &git2::Repository, _flags: uint) -> Result<u32, libc::c_int> {
        if self.data.is_none() {
            if let Ok(blob) = repo.find_blob(self.oid) {
                self.data = Some(blob.content().to_vec());
            } else {
                return Err(posix88::EIO)
            }
        }
        Ok(fuse::consts::FOPEN_KEEP_CACHE)
    }

    fn read(&mut self, _repo: &git2::Repository, offset: u64, size: uint
           ) -> Result<&[u8], libc::c_int> {
        if let Some(ref data) = self.data {
            if offset <= data.len() as u64 {
                let data = data.slice_from(offset as uint);
                return Ok(if size < data.len() {
                    data.slice_to(size)
                } else {
                    data
                })
            }
        }
        Err(posix88::EINVAL)
    }

    fn release (&mut self, _repo: &git2::Repository) -> Result<(), libc::c_int> {
        self.data.take();
        Ok(())
    }
}
