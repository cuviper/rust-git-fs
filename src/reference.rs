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
use std::collections::hash_map;
use std::default::Default;
use std::io;

use inode;


/// Represents a virtual directory in reference paths
/// (e.g. `refs/heads/master` needs intermediate `refs/` and `refs/heads/`)
pub struct RefDir {
    entries: hash_map::HashMap<PosixPath, inode::Id>,
}

impl RefDir {
    pub fn new() -> Box<inode::Inode+'static> {
        box RefDir {
            entries: Default::default(),
        }
    }
}

impl inode::Inode for RefDir {
    fn lookup(&mut self, _repo: &git2::Repository, name: &PosixPath
              ) -> Result<inode::Id, libc::c_int> {
        self.entries.get(name).cloned().ok_or(posix88::ENOENT)
    }

    fn getattr(&mut self, _repo: &git2::Repository, attr: inode::FileAttr
               ) -> Result<inode::FileAttr, libc::c_int> {
        let size = self.entries.len() as u64;
        Ok(inode::FileAttr {
            size: size,
            blocks: inode::st_blocks(size),
            kind: io::TypeDirectory,
            perm: io::USER_DIR,
            ..attr
        })
    }

    fn readdir(&mut self, _repo: &git2::Repository, offset: u64,
               add: |inode::Id, io::FileType, &PosixPath| -> bool
              ) -> Result<(), libc::c_int> {
        if offset < self.entries.len() as u64 {
            for (path, &id) in self.entries.iter().skip(offset as uint) {
                if add(id, io::TypeDirectory, path) {
                    break;
                }
            }
        }
        Ok(())
    }
}



