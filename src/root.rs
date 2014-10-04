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

/// The root of the filesystem, currently just revealing HEAD and refs/
pub struct Root<'a> {
    head: Option<git2::Reference<'a>>,
    refs: inode::Id,
}

impl<'a> Root<'a> {
    pub fn new(repo: &git2::Repository, refs: inode::Id) -> Box<inode::Inode> {
        box Root {
            head: repo.head().ok(),
            refs: refs,
        }
    }
}

impl<'a> inode::Inode for Root<'a> {
    fn lookup(&self, name: &PosixPath) -> Result<inode::Id, libc::c_int> {
        if name.as_vec() == b"HEAD" {
            self.head.as_ref()
                .and_then(|head| head.target())
                .map(|oid| inode::Oid(oid))
        }
        else if name.as_vec() == b"refs" {
            Some(self.refs)
        }
        else { None }.ok_or(posix88::ENOENT)
    }

    fn getattr(&self, attr: inode::FileAttr) -> Result<inode::FileAttr, libc::c_int> {
        let size = 1; // just HEAD
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
        if offset == 0 {
            add(self.refs, io::TypeUnknown, &PosixPath::new("refs"));
        }
        if offset <= 1 {
            match self.head.as_ref().and_then(|head| head.target()) {
                Some(oid) => {
                    add(inode::Oid(oid), io::TypeUnknown, &PosixPath::new("HEAD"));
                },
                None => (),
            }
        }
        Ok(())
    }
}



