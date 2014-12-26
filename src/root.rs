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
use inode::{FileAttr, Id, Inode};

/// The root of the filesystem, currently just revealing HEAD and refs/
pub struct Root {
    head: Id,
    refs: Id,
}

impl Root {
    pub fn new(head: Id, refs: Id) -> Box<Inode+'static> {
        box Root {
            head: head,
            refs: refs,
        }
    }
}

impl Inode for Root {
    fn lookup(&mut self, repo: &git2::Repository, name: &PosixPath
             ) -> Result<Id, libc::c_int> {
        if name.as_vec() == b"HEAD" {
            repo.head().ok()
                .and_then(|head| head.target())
                .map(|oid| Id::Oid(oid))
        }
        else if name.as_vec() == b"refs" {
            Some(self.refs)
        }
        else { None }.ok_or(posix88::ENOENT)
    }

    fn getattr(&mut self, _repo: &git2::Repository, attr: FileAttr
              ) -> Result<FileAttr, libc::c_int> {
        let size = 2; // just HEAD and refs/
        Ok(FileAttr {
            size: size,
            blocks: inode::st_blocks(size),
            kind: io::FileType::Directory,
            perm: io::USER_DIR,
            ..attr
        })
    }

    fn readdir(&mut self, _repo: &git2::Repository, offset: u64,
               add: |Id, io::FileType, &PosixPath| -> bool
              ) -> Result<(), libc::c_int> {
        if offset == 0 {
            add(self.head, io::FileType::Unknown, &PosixPath::new("HEAD"));
        }
        if offset <= 1 {
            add(self.refs, io::FileType::Unknown, &PosixPath::new("refs"));
        }
        Ok(())
    }
}



