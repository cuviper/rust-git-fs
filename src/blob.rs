// Copyright (C) 2014  Josh Stone
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use git2;
use libc;
use libc::consts::os::posix88;
use std::io;

use inode;

/// Git blobs are represented as files
// FIXME needs context, e.g. permissions from TreeEntry and timestamps from Commit
// FIXME it's probably a waste of memory to keep git2::Blob in memory all the time.  If the Inode
// functions passed a repo parameter, we could remember just the Oid and look it up dynamically.
pub struct Blob<'a> {
    blob: git2::Blob<'a>,
}

impl<'a> Blob<'a> {
    pub fn new(blob: git2::Blob<'a>) -> Box<inode::Inode> {
        box Blob {
            blob: blob,
        }
    }
}

impl<'a> inode::Inode for Blob<'a> {
    fn getattr(&mut self, _repo: &git2::Repository, attr: inode::FileAttr
              ) -> Result<inode::FileAttr, libc::c_int> {
        let size = self.blob.content().len() as u64;
        Ok(inode::FileAttr {
            size: size,
            blocks: inode::st_blocks(size),
            kind: io::TypeFile,
            perm: io::UserFile,
            ..attr
        })
    }

    fn read(&mut self, _repo: &git2::Repository, offset: u64, size: uint
           ) -> Result<&[u8], libc::c_int> {
        let data = self.blob.content();
        if offset <= data.len() as u64 {
            let data = data.slice_from(offset as uint);
            Ok(if size < data.len() {
                data.slice_to(size)
            } else {
                data
            })
        } else {
            Err(posix88::EINVAL)
        }
    }
}
