// Copyright (C) 2014  Josh Stone
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// use git2;
use libc;
use libc::consts::os::posix88;
use std::collections::hashmap;
use std::default::Default;
use std::io;

use inode;


/// Represents a virtual directory in reference paths
/// (e.g. `refs/heads/master` needs intermediate `refs/` and `refs/heads/`)
pub struct RefDir<'a> {
    entries: hashmap::HashMap<PosixPath, inode::Id>,
}

impl<'a> RefDir<'a> {
    pub fn new() -> Box<inode::Inode+'a> {
        box RefDir {
            entries: Default::default(),
        }
    }
}

impl<'a> inode::Inode for RefDir<'a> {
    fn lookup(&self, name: &PosixPath) -> Result<inode::Id, libc::c_int> {
        self.entries.find_copy(name).ok_or(posix88::ENOENT)
    }

    fn getattr(&self, attr: inode::FileAttr) -> Result<inode::FileAttr, libc::c_int> {
        let size = self.entries.len() as u64;
        Ok(inode::FileAttr {
            size: size,
            blocks: inode::st_blocks(size),
            kind: io::TypeDirectory,
            perm: io::UserDir,
            ..attr
        })
    }

    fn readdir (&self, offset: u64, add: |inode::Id, io::FileType, &PosixPath| -> bool
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



