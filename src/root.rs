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
use std::old_io::{FileType, USER_DIR};

use inode;
use inode::{FileAttr, Id, Inode};

/// The root of the filesystem, currently just revealing HEAD and refs/
pub struct Root {
    head: Id,
    refs: Id,
}

impl Root {
    pub fn new(head: Id, refs: Id) -> Box<Inode+'static> {
        Box::new(Root {
            head: head,
            refs: refs,
        })
    }
}

impl Inode for Root {
    fn lookup(&mut self, repo: &git2::Repository, name: &Path
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
            kind: FileType::Directory,
            perm: USER_DIR,
            ..attr
        })
    }

    fn readdir<'a>(&mut self, _repo: &git2::Repository, offset: u64,
               mut add: Box<FnMut(Id, FileType, &Path) -> bool + 'a>
              ) -> Result<(), libc::c_int> {
        if offset == 0 {
            add(self.head, FileType::Unknown, &Path::new("HEAD"));
        }
        if offset <= 1 {
            add(self.refs, FileType::Unknown, &Path::new("refs"));
        }
        Ok(())
    }
}



