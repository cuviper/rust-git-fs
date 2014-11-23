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
use std::io;

use blob;
use tree;

pub use fuse::FileAttr;


/// Let Inodes use either existing inos or git2::Oid, whichever is convenient
// FIXME Using a 1:1 mapping between oid:ino breaks down when it comes to attributes.  For
// instance, Blobs and Trees have no concept of their own timestamps or permissions.  But a Tree
// does know its children's permissions in TreeEntry, and a Commit could propagate timestamps
// recursively down its Tree.  This will require Oids to be context sensitive, with 1:N inos.
#[deriving(Clone)]
pub enum Id {
    Ino(u64),
    Oid(git2::Oid),
}


/// A generic interface for different Git object types to implement.
pub trait Inode {
    /// Find a directory entry in this Inode by name.
    fn lookup(&mut self, _repo: &git2::Repository, _name: &PosixPath
             ) -> Result<Id, libc::c_int> {
        Err(posix88::ENOTDIR)
    }

    /// Get the attributes of this Inode.
    fn getattr(&mut self, _repo: &git2::Repository, _attr: FileAttr
              ) -> Result<FileAttr, libc::c_int> {
        Err(posix88::EINVAL)
    }

    /// Open a file.
    fn open(&mut self, _repo: &git2::Repository, _flags: uint) -> Result<u32, libc::c_int> {
        Err(posix88::EISDIR)
    }

    /// Read data from this Inode.
    fn read(&mut self, _repo: &git2::Repository, _offset: u64, _size: uint
           ) -> Result<&[u8], libc::c_int> {
        Err(posix88::EISDIR)
    }

    /// Release data from an opened file.
    fn release(&mut self, _repo: &git2::Repository) -> Result<(), libc::c_int> {
        Err(posix88::EISDIR)
    }

    /// Read directory entries from this Inode.
    fn readdir(&mut self, _repo: &git2::Repository, _offset: u64,
               _add: |Id, io::FileType, &PosixPath| -> bool
              ) -> Result<(), libc::c_int> {
        Err(posix88::ENOTDIR)
    }
}


/// Assign new inode numbers, and map Oids to ino dynamically
// FIXME see the note on Id about 1:1 mapping trouble
#[deriving(Default)]
pub struct InodeMapper {
    max_ino: u64,
    oids: hash_map::HashMap<git2::Oid, u64>,
    inos: hash_map::HashMap<u64, git2::Oid>,
}

impl InodeMapper {
    /// Reserve a new inode number
    pub fn new_ino(&mut self) -> u64 {
        self.max_ino += 1;
        self.max_ino
    }

    /// Get the oid associated with this ino
    pub fn get_oid(&self, ino: u64) -> Option<git2::Oid> {
        self.inos.get(&ino).cloned()
    }

    /// Map any Id to an inode number
    pub fn get_ino(&mut self, id: Id) -> u64 {
        match id {
            Id::Ino(ino) => ino,
            Id::Oid(oid) => {
                match self.oids.entry(oid) {
                    hash_map::Occupied(entry) => *entry.get(),
                    hash_map::Vacant(entry) => {
                        // NB can't call new_ino because entry holds mut
                        self.max_ino += 1;
                        let ino = self.max_ino;
                        self.inos.insert(ino, oid);
                        *entry.set(ino)
                    },
                }
            },
        }
    }
}


/// A separate container allows mut borrowing without blocking everything else
/// in the GitFS at the same time.
#[deriving(Default)]
pub struct InodeContainer<'a> {
    inodes: hash_map::HashMap<u64, Box<Inode+'a>>,
}

impl<'a> InodeContainer<'a> {
    pub fn insert(&mut self, ino: u64, inode: Box<Inode+'a>) -> Option<Box<Inode+'a>> {
        self.inodes.insert(ino, inode)
    }

    pub fn find_mut(&'a mut self, ino: u64) -> Result<&mut Box<Inode>, libc::c_int> {
        self.inodes.get_mut(&ino).ok_or(posix88::ENOENT)
    }

    pub fn entry(&'a mut self, ino: u64) -> hash_map::Entry<u64, Box<Inode>> {
        self.inodes.entry(ino)
    }
}


/// Creates an Inode from any Oid.
// FIXME see the note on Id about 1:1 mapping trouble
pub fn new_inode(repo: &git2::Repository, oid: git2::Oid) -> Option<Box<Inode>> {
    match repo.find_object(oid, None).ok().and_then(|o| o.kind()) {
        Some(git2::ObjectType::Blob) => {
            repo.find_blob(oid).ok().map(|blob| blob::Blob::new(blob))
        },
        Some(git2::ObjectType::Tree) => {
            repo.find_tree(oid).ok().map(|tree| tree::Tree::new(tree))
        },
        Some(git2::ObjectType::Commit) => {
            // FIXME a first-class Commit might expose things like the message as xattrs,
            // but for now just redirect straight to the tree id.
            repo.find_commit(oid).ok()
                .and_then(|commit| new_inode(repo, commit.tree_id()))
        },
        _ => None,
    }
}


/// Compute the number of blocks needed to contain a given size.
pub fn st_blocks(size: u64) -> u64 {
    // NB FUSE apparently always uses 512-byte blocks.  Round up.
    (size + 511) / 512
}
