// Copyright (C) 2014  Josh Stone
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # GitFS: a FUSE filesystem for Git objects

#![feature(asm)]
#![feature(core)]
#![feature(io)]
#![feature(libc)]
#![feature(old_io)]
#![feature(old_path)]
#![feature(path)]
#![feature(std_misc)]

#![deny(missing_docs)]

#[macro_use] #[no_link]
extern crate probe;

extern crate fuse;
extern crate git2;
extern crate libc;
extern crate time;

use std::collections::hash_map;
use std::default::Default;
use std::ffi::CString;
use std::ffi::OsString;
use std::fs;
use std::old_io::FileType;
use std::old_path::PosixPath;
use std::path::{AsPath, Path, PathBuf};
use std::u64;

use inode::{Id, InodeContainer, InodeMapper};

mod inode;
mod blob;
mod tree;
mod reference;
mod root;


const TTY: time::Timespec = time::Timespec { sec: 1, nsec: 0 };


/// The main object implementing a FUSE filesystem.
pub struct GitFS {
    repo: git2::Repository,
    epoch: time::Timespec,
    uid: u32,
    gid: u32,
    mapper: InodeMapper,
    inodes: InodeContainer,
    mountdir: Option<DirHandle>,
}

impl GitFS {
    /// Create a GitFS referencing the given GIT_DIR.
    pub fn new(git_dir: &Path) -> Result<GitFS, git2::Error> {
        Ok(GitFS {
            repo: try!(git2::Repository::open(git_dir)),
            epoch: time::get_time(),
            uid: unsafe { libc::getuid() },
            gid: unsafe { libc::getgid() },
            mapper: Default::default(),
            inodes: Default::default(),
            mountdir: None,
        })
    }

    /// Get the resolved GIT_DIR.
    pub fn git_dir(&self) -> &Path {
        self.repo.path()
    }

    fn mount_options(&self) -> OsString {
        let mut options = OsString::from_str("-oro,default_permissions,fsname=");
        options.push(&self.repo.path()); // FIXME escape commas?
        options
    }

    /// Mount the filesystem and wait until the path is unmounted, e.g. with the command
    /// `fusermount -u PATH`.
    pub fn mount<P: AsPath>(mut self, mountpoint: &P) {
        // Create/remove the mount point if it doesn't exist
        self.mountdir = DirHandle::new(mountpoint.as_path());

        let options = self.mount_options();
        fuse::mount(self, mountpoint, &[&options])
    }

    /// Mount the filesystem in the background.  It will remain mounted until the returned session
    /// object is dropped, or an external umount is issued.
    pub fn spawn_mount<P: AsPath>(mut self, mountpoint: &P) -> std::io::Result<fuse::BackgroundSession> {
        // Create/remove the mount point if it doesn't exist
        self.mountdir = DirHandle::new(mountpoint.as_path());

        let options = self.mount_options();
        fuse::spawn_mount(self, mountpoint, &[&options])
    }

    fn defattr(&self, ino: u64) -> fuse::FileAttr {
        fuse::FileAttr {
            ino: ino,
            size: 0,
            blocks: 0,
            atime: self.epoch,
            mtime: self.epoch,
            ctime: self.epoch,
            crtime: self.epoch,
            kind: FileType::Unknown,
            perm: Default::default(),
            nlink: 1,
            uid: self.uid,
            gid: self.gid,
            rdev: 0,
            flags: 0,
        }
    }
}

impl fuse::Filesystem for GitFS {
    fn init (&mut self, _req: &fuse::Request) -> Result<(), libc::c_int> {
        let root_ino = self.mapper.new_ino();
        let head_ino = self.mapper.new_ino();
        let refs_ino = self.mapper.new_ino();
        assert_eq!(fuse::FUSE_ROOT_ID, root_ino);

        let root = root::Root::new(Id::Ino(head_ino), Id::Ino(refs_ino));
        self.inodes.insert(root_ino, root);

        let refs = reference::RefDir::new();
        self.inodes.insert(refs_ino, refs);

        Ok(())
    }

    fn lookup(&mut self, _req: &fuse::Request, parent: u64, name: &PosixPath, reply: fuse::ReplyEntry) {
        if let Ok(name) = CString::new(name.as_vec()) {
            probe!(gitfs, lookup, parent, name.as_ptr());
        }

        let repo = &self.repo;
        let id = {
            let inode = self.inodes.find_mut(parent);
            match inode.and_then(|inode| inode.lookup(repo, name)) {
                Ok(id) => id,
                Err(rc) => return reply.error(rc),
            }
        };
        let ino = self.mapper.get_ino(id);

        if let hash_map::Entry::Vacant(entry) = self.inodes.entry(ino) {
            if let Some(oid) = self.mapper.get_oid(ino) {
                if let Some(inode) = inode::new_inode(&self.repo, oid) {
                    entry.insert(inode);
                }
            }
        }

        let attr = self.defattr(ino);
        let inode = self.inodes.find_mut(ino);
        match inode.and_then(|inode| inode.getattr(repo, attr)) {
            Ok(attr) => reply.entry(&TTY, &attr, 1),
            Err(rc) => reply.error(rc),
        }
    }

    fn forget (&mut self, _req: &fuse::Request, _ino: u64, _nlookup: u64) {
        probe!(gitfs, forget, _ino, _nlookup);

        // TODO could probably drop Oid inodes, since they're easily recreated
    }

    fn getattr (&mut self, _req: &fuse::Request, ino: u64,
                reply: fuse::ReplyAttr) {
        probe!(gitfs, getattr, ino);

        let attr = self.defattr(ino);
        let repo = &self.repo;
        let inode = self.inodes.find_mut(ino);
        match inode.and_then(|inode| inode.getattr(repo, attr)) {
            Ok(attr) => reply.attr(&TTY, &attr),
            Err(rc) => reply.error(rc),
        }
    }

    fn open (&mut self, _req: &fuse::Request, ino: u64, flags: u32, reply: fuse::ReplyOpen) {
        probe!(gitfs, open, ino, flags);

        let repo = &self.repo;
        let inode = self.inodes.find_mut(ino);
        match inode.and_then(|inode| inode.open(repo, flags)) {
            Ok(flags) => reply.opened(0, flags),
            Err(rc) => reply.error(rc),
        }
    }
    fn read (&mut self, _req: &fuse::Request, ino: u64, _fh: u64, offset: u64, size: u32,
             reply: fuse::ReplyData) {
        probe!(gitfs, read, ino, offset, size);

        let repo = &self.repo;
        let inode = self.inodes.find_mut(ino);
        match inode.and_then(|inode| inode.read(repo, offset, size)) {
            Ok(data) => reply.data(data),
            Err(rc) => reply.error(rc),
        }
    }

    fn release (&mut self, _req: &fuse::Request, ino: u64, _fh: u64, _flags: u32,
                _lock_owner: u64, _flush: bool, reply: fuse::ReplyEmpty) {
        probe!(gitfs, release, ino);

        let repo = &self.repo;
        let inode = self.inodes.find_mut(ino);
        match inode.and_then(|inode| inode.release(repo)) {
            Ok(()) => reply.ok(),
            Err(rc) => reply.error(rc),
        }
    }

    fn readdir (&mut self, _req: &fuse::Request, ino: u64, _fh: u64, mut offset: u64,
                mut reply: fuse::ReplyDirectory) {
        probe!(gitfs, readdir, ino, offset);

        let mapper = &mut self.mapper;
        let repo = &self.repo;
        let inode = self.inodes.find_mut(ino);
        match inode.and_then(|inode| {
            if offset == 0 {
                offset += 1;
                reply.add(u64::MAX, offset, FileType::Directory, &PosixPath::new("."));
            }
            if offset == 1 {
                offset += 1;
                reply.add(u64::MAX, offset, FileType::Directory, &PosixPath::new(".."));
            }
            inode.readdir(repo, offset - 2, Box::new(|id, kind, path| {
                offset += 1;
                reply.add(mapper.get_ino(id), offset, kind, path)
            }))
        }) {
            Ok(()) => reply.ok(),
            Err(rc) => reply.error(rc),
        }
    }
}


/// Helper for mkdir, ensuring rmdir when dropped
struct DirHandle {
    path: PathBuf,
}

impl DirHandle {
    fn new(path: &Path) -> Option<DirHandle> {
        match fs::create_dir(path) {
            Ok(()) => Some(DirHandle { path: path.to_path_buf() }),
            Err(_) => None,
        }
    }
}

impl Drop for DirHandle {
    fn drop(&mut self) {
        fs::remove_dir(&self.path).ok();
    }
}
