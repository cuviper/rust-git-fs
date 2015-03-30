// Copyright (C) 2014  Josh Stone
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use fuse::FileType;
use git2;
use libc;
use libc::consts::os::posix88;
use std::collections::hash_map;
use std::default::Default;
use std::path::{Path, PathBuf};

use inode;


/// Represents a virtual directory in reference paths
/// (e.g. `refs/heads/master` needs intermediate `refs/` and `refs/heads/`)
pub struct RefDir {
    entries: hash_map::HashMap<PathBuf, inode::Id>,
}

impl RefDir {
    pub fn new() -> Box<inode::Inode+'static> {
        Box::new(RefDir {
            entries: Default::default(),
        })
    }
}

impl inode::Inode for RefDir {
    fn lookup(&mut self, _repo: &git2::Repository, name: &Path
              ) -> Result<inode::Id, libc::c_int> {
        // workaround for rust-lang/rust#23649
        let name_buf = name.to_path_buf();
        let name = &name_buf;

        self.entries.get(name).cloned().ok_or(posix88::ENOENT)
    }

    fn getattr(&mut self, _repo: &git2::Repository, attr: inode::FileAttr
               ) -> Result<inode::FileAttr, libc::c_int> {
        let size = self.entries.len() as u64;
        Ok(inode::FileAttr {
            size: size,
            blocks: inode::st_blocks(size),
            kind: FileType::Directory,
            perm: 0o755,
            ..attr
        })
    }

    fn readdir<'a>(&mut self, _repo: &git2::Repository, offset: u64,
               mut add: Box<FnMut(inode::Id, FileType, &Path) -> bool + 'a>
              ) -> Result<(), libc::c_int> {
        if offset < self.entries.len() as u64 {
            for (path, &id) in self.entries.iter().skip(offset as usize) {
                if add(id, FileType::Directory, path) {
                    break;
                }
            }
        }
        Ok(())
    }
}



