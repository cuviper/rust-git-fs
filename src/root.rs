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
pub struct Root<'a> {
    head: Option<git2::Reference<'a>>,
    refs: Id,
}

impl<'a> Root<'a> {
    pub fn new(repo: &git2::Repository, refs: Id) -> Box<Inode> {
        box Root {
            head: repo.head().ok(),
            refs: refs,
        }
    }
}

impl<'a> Inode for Root<'a> {
    fn lookup(&mut self, _repo: &git2::Repository, name: &PosixPath
             ) -> Result<Id, libc::c_int> {
        if name.as_vec() == b"HEAD" {
            self.head.as_ref()
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
        let size = 1; // just HEAD
        Ok(FileAttr {
            size: size,
            blocks: inode::st_blocks(size),
            kind: io::TypeDirectory,
            perm: io::USER_DIR,
            ..attr
        })
    }

    fn readdir(&mut self, _repo: &git2::Repository, offset: u64,
               add: |Id, io::FileType, &PosixPath| -> bool
              ) -> Result<(), libc::c_int> {
        if offset == 0 {
            add(self.refs, io::TypeUnknown, &PosixPath::new("refs"));
        }
        if offset <= 1 {
            match self.head.as_ref().and_then(|head| head.target()) {
                Some(oid) => {
                    add(Id::Oid(oid), io::TypeUnknown, &PosixPath::new("HEAD"));
                },
                None => (),
            }
        }
        Ok(())
    }
}



