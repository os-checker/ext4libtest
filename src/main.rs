use ext4_rs::*;

extern crate alloc;
pub use alloc::sync::Arc;
use clap::{crate_version, Arg, ArgAction, Command};
use fuser::{
    FileAttr, FileType, Filesystem, MountOption, ReplyAttr, ReplyData, ReplyDirectory, ReplyEmpty,
    ReplyEntry, ReplyOpen, ReplyWrite, Request, TimeOrNow,
};
use log::{Level, LevelFilter, Metadata, Record};

use std::ffi::OsStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

macro_rules! with_color {
    ($color_code:expr, $($arg:tt)*) => {{
        format_args!("\u{1B}[{}m{}\u{1B}[m", $color_code as u8, format_args!($($arg)*))
    }};
}

struct SimpleLogger;

impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        let level = record.level();
        let args_color = match level {
            Level::Error => ColorCode::Red,
            Level::Warn => ColorCode::Yellow,
            Level::Info => ColorCode::Green,
            Level::Debug => ColorCode::Cyan,
            Level::Trace => ColorCode::BrightBlack,
        };

        if self.enabled(record.metadata()) {
            println!(
                "{} - {}",
                record.level(),
                with_color!(args_color, "{}", record.args())
            );
        }
    }

    fn flush(&self) {}
}

#[repr(u8)]
enum ColorCode {
    Red = 31,
    Green = 32,
    Yellow = 33,
    Cyan = 36,
    BrightBlack = 90,
}

pub const EPERM: i32 = 1;
pub const ENOENT: i32 = 2;
pub const ESRCH: i32 = 3;
pub const EINTR: i32 = 4;
pub const EIO: i32 = 5;
pub const ENXIO: i32 = 6;
pub const E2BIG: i32 = 7;
pub const ENOEXEC: i32 = 8;
pub const EBADF: i32 = 9;
pub const ECHILD: i32 = 10;
pub const EAGAIN: i32 = 11;
pub const ENOMEM: i32 = 12;
pub const EACCES: i32 = 13;
pub const EFAULT: i32 = 14;
pub const ENOTBLK: i32 = 15;
pub const EBUSY: i32 = 16;
pub const EEXIST: i32 = 17;
pub const EXDEV: i32 = 18;
pub const ENODEV: i32 = 19;
pub const ENOTDIR: i32 = 20;
pub const EISDIR: i32 = 21;
pub const EINVAL: i32 = 22;
pub const ENFILE: i32 = 23;
pub const EMFILE: i32 = 24;
pub const ENOTTY: i32 = 25;
pub const ETXTBSY: i32 = 26;
pub const EFBIG: i32 = 27;
pub const ENOSPC: i32 = 28;
pub const ESPIPE: i32 = 29;
pub const EROFS: i32 = 30;
pub const EMLINK: i32 = 31;
pub const EPIPE: i32 = 32;
pub const EDOM: i32 = 33;
pub const ERANGE: i32 = 34;
pub const EWOULDBLOCK: i32 = EAGAIN;

pub const S_IFIFO: u32 = 4096;
pub const S_IFCHR: u32 = 8192;
pub const S_IFBLK: u32 = 24576;
pub const S_IFDIR: u32 = 16384;
pub const S_IFREG: u32 = 32768;
pub const S_IFLNK: u32 = 40960;
pub const S_IFSOCK: u32 = 49152;
pub const S_IFMT: u32 = 61440;
pub const S_IRWXU: u32 = 448;
pub const S_IXUSR: u32 = 64;
pub const S_IWUSR: u32 = 128;
pub const S_IRUSR: u32 = 256;
pub const S_IRWXG: u32 = 56;
pub const S_IXGRP: u32 = 8;
pub const S_IWGRP: u32 = 16;
pub const S_IRGRP: u32 = 32;
pub const S_IRWXO: u32 = 7;
pub const S_IXOTH: u32 = 1;
pub const S_IWOTH: u32 = 2;
pub const S_IROTH: u32 = 4;
pub const F_OK: i32 = 0;
pub const R_OK: i32 = 4;
pub const W_OK: i32 = 2;
pub const X_OK: i32 = 1;
pub const STDIN_FILENO: i32 = 0;
pub const STDOUT_FILENO: i32 = 1;
pub const STDERR_FILENO: i32 = 2;
pub const SIGHUP: i32 = 1;
pub const SIGINT: i32 = 2;
pub const SIGQUIT: i32 = 3;
pub const SIGILL: i32 = 4;
pub const SIGABRT: i32 = 6;
pub const SIGFPE: i32 = 8;
pub const SIGKILL: i32 = 9;
pub const SIGSEGV: i32 = 11;
pub const SIGPIPE: i32 = 13;
pub const SIGALRM: i32 = 14;
pub const SIGTERM: i32 = 15;

const TTL: Duration = Duration::from_secs(1); // 1 second

#[derive(Debug)]
pub struct Disk {}

impl BlockDevice for Disk {
    fn read_offset(&self, offset: usize) -> Vec<u8> {
        // log::info!("read_offset: {:x?}", offset);
        use std::fs::OpenOptions;
        use std::io::{Read, Seek};
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open("ex4.img")
            .unwrap();
        let mut buf = vec![0u8; BLOCK_SIZE as usize];
        let r = file.seek(std::io::SeekFrom::Start(offset as u64));
        let r = file.read_exact(&mut buf);

        buf
    }

    fn write_offset(&self, offset: usize, data: &[u8]) {
        use std::fs::OpenOptions;
        use std::io::{Read, Seek, Write};
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open("ex4.img")
            .unwrap();

        let r = file.seek(std::io::SeekFrom::Start(offset as u64));
        let r = file.write_all(&data);
    }
}

struct Ext4Fuse {
    ext4: Arc<Ext4>,
}

impl Ext4Fuse {
    pub fn new(ext4: Arc<Ext4>) -> Self {
        Self { ext4: ext4 }
    }
}

impl Filesystem for Ext4Fuse {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {

        let mut parent = parent;
        // fuse use 1 as root inode
        if parent == 1{
            //root
            parent = 2;
        }

        let mut path = String::new();
        path += name.to_str().unwrap();

        
        let mut file = Ext4File::new();
        let result = self.ext4.ext4_open_from(parent as u32,&mut file, path.as_str(), "r", false);
        // let result = self.ext4.ext4_open_new(&mut file, path.as_str(), "r", false);
        log::info!("lookup parent {:x?} path {:?} result {:?}", parent, path, result);


        let mut inode_ref = Ext4InodeRef::get_inode_ref(self.ext4.self_ref.clone(), file.inode as u32);
        let mode = inode_ref.inner.inode.mode;
        let inode_type = InodeMode::from_bits(mode & EXT4_INODE_MODE_TYPE_MASK as u16).unwrap();

        let file_type = match inode_type {
            InodeMode::S_IFDIR => {
                FileType::Directory
            }
            InodeMode::S_IFREG => {
                FileType::RegularFile
            }
            _ => {
                FileType::RegularFile
            }
        };

        match result {
            Ok(_) => {
                log::info!("open success: {:x?}", file.inode);
                let attr = FileAttr {
                    ino: file.inode as u64,
                    size: file.fsize,
                    blocks: file.fsize / BLOCK_SIZE as u64,
                    atime: UNIX_EPOCH,
                    mtime: UNIX_EPOCH,
                    ctime: UNIX_EPOCH,
                    crtime: UNIX_EPOCH,
                    // fix me
                    kind: file_type,
                    perm: 0o644,
                    nlink: 1,
                    uid: 501,
                    gid: 20,
                    rdev: 0,
                    flags: 0,
                    blksize: BLOCK_SIZE as u32,
                };
                reply.entry(&TTL, &attr, 0);
            }
            Err(_) => reply.error(ENOENT),
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, _fh: Option<u64>, reply: ReplyAttr) {
        let inode = match ino {
            // root
            1 => 2,
            _ => ino,
        };

        log::info!("getattr: {:x?}", inode);
        let mut inode_ref = Ext4InodeRef::get_inode_ref(self.ext4.self_ref.clone(), inode as u32);
        let link_cnt = inode_ref.inner.inode.ext4_inode_get_links_cnt() as u32;
        let mode = inode_ref.inner.inode.mode;
        let inode_type = InodeMode::from_bits(mode & EXT4_INODE_MODE_TYPE_MASK as u16).unwrap();
        let file_type = match inode_type {
            InodeMode::S_IFDIR => FileType::Directory,
            InodeMode::S_IFREG => FileType::RegularFile,
            /* Reset blocks array. For inode which is not directory or file, just
             * fill in blocks with 0 */
            _ => FileType::RegularFile,
        };

        let attr = FileAttr {
            ino: inode,
            size: inode_ref.inner.inode.inode_get_size() as u64,
            blocks: inode_ref.inner.inode.inode_get_size() / BLOCK_SIZE as u64,
            atime: UNIX_EPOCH, // Example static time, adjust accordingly
            mtime: UNIX_EPOCH,
            ctime: UNIX_EPOCH,
            crtime: UNIX_EPOCH,
            kind: file_type, // Adjust according to inode type
            perm: 0o777,     // Need a method to translate inode perms to Unix perms
            nlink: link_cnt,
            uid: 501,
            gid: 20,
            rdev: 0, // Device nodes not covered here
            flags: 0,
            blksize: BLOCK_SIZE as u32,
        };
        reply.attr(&TTL, &attr);
    }

    fn setattr(
        &mut self,
        req: &Request,
        inode: u64,
        mode: Option<u32>,
        uid: Option<u32>,
        gid: Option<u32>,
        size: Option<u64>,
        atime: Option<TimeOrNow>,
        mtime: Option<TimeOrNow>,
        ctime: Option<SystemTime>,
        fh: Option<u64>,
        crtime: Option<SystemTime>,
        chgtime: Option<SystemTime>,
        bkuptime: Option<SystemTime>,
        flags: Option<u32>,
        reply: ReplyAttr,
    ) {
        let attrs = InodeAttr {
            mode,
            uid,
            gid,
            size,
            atime,
            mtime,
            ctime,
            crtime,
            chgtime,
            bkuptime,
            flags,
        };

        self.set_attr(inode as u32, &attrs);

        // 获取 inode 的引用以准备响应
        let inode_ref = Ext4InodeRef::get_inode_ref(self.ext4.self_ref.clone(), inode as u32);

        let mode = inode_ref.inner.inode.mode;
        let inode_type = InodeMode::from_bits(mode & EXT4_INODE_MODE_TYPE_MASK as u16).unwrap();
        let file_type = match inode_type {
            InodeMode::S_IFDIR => FileType::Directory,
            InodeMode::S_IFREG => FileType::RegularFile,
            _ => FileType::RegularFile,
        };

        let response_attr = FileAttr {
            ino: inode,
            size: inode_ref.inner.inode.inode_get_size(),
            blocks: inode_ref.inner.inode.ext4_inode_get_blocks_count(),
            atime: timestamp_to_system_time(inode_ref.inner.inode.ext4_inode_get_atime()),
            mtime: timestamp_to_system_time(inode_ref.inner.inode.ext4_inode_get_mtime()),
            ctime: timestamp_to_system_time(inode_ref.inner.inode.ext4_inode_get_ctime()),
            crtime: timestamp_to_system_time(inode_ref.inner.inode.ext4_inode_get_crtime()),
            kind: file_type,
            perm: inode_ref.inner.inode.ext4_get_inode_mode() & 0o777,
            nlink: inode_ref.inner.inode.ext4_inode_get_links_cnt() as u32,
            uid: inode_ref.inner.inode.uid as u32,
            gid: inode_ref.inner.inode.gid as u32,
            rdev: 0, // 设备节点处理
            flags: 0,
            blksize: BLOCK_SIZE as u32,
        };

        reply.attr(&Duration::from_secs(1), &response_attr); // 缓存时间可调整
    }

    fn read(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock: Option<u64>,
        reply: ReplyData,
    ) {
        log::info!("-----------read-----------");
        let mut file = Ext4File::new();
        file.inode = ino as u32;
        file.fpos = offset as usize;

        let mut data = vec![0u8; size as usize];
        let mut read_cnt = 0;
        let result = self
            .ext4
            .ext4_file_read(&mut file, &mut data, size as usize, &mut read_cnt);

        match result {
            Ok(_) => reply.data(&data),
            Err(_) => reply.error(ENOENT), // Adjust error handling as needed
        }
    }

    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        let inode = match ino {
            // root
            1 => 2,
            _ => ino,
        };
        // log::info!("-----------readdir-----------inode {:x?}", ino);
        let entries = self.ext4.read_dir_entry(inode);
        for (i, entry) in entries.iter().enumerate().skip(offset as usize) {
            let name = get_name(entry.name, entry.name_len as usize).unwrap();
            let detype = entry.get_de_type();
            let kind = match detype {
                1 => FileType::RegularFile,
                2 => FileType::Directory,
                _ => FileType::RegularFile,
            };
            reply.add(entry.inode as u64, (i + 1) as i64, kind, &name);
        }
        reply.ok();
    }

    fn write(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        fh: u64,
        offset: i64,
        data: &[u8],
        write_flags: u32,
        flags: i32,
        lock_owner: Option<u64>,
        reply: ReplyWrite,
    ) {
        let mut file = Ext4File::new();
        file.inode = ino as u32;
        file.fpos = offset as usize;

        self.ext4.ext4_file_write(&mut file, data, data.len());
        reply.written(data.len() as u32)
    }

    /// Remove a file.
    fn unlink(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        let path = name.to_str().unwrap_or_default();
        let mut parent_ref = Ext4InodeRef::get_inode_ref(self.ext4.self_ref.clone(), parent as u32);
        let mut child = Ext4File::new();
        let open_result = self.ext4.ext4_open(&mut child, path, "r", false);

        if let Ok(_) = open_result {
            let mut child_ref =
                Ext4InodeRef::get_inode_ref(self.ext4.self_ref.clone(), child.inode);
            let result =
                self.ext4
                    .ext4_unlink(&mut parent_ref, &mut child_ref, path, path.len() as u32);
            match result {
                EOK => reply.ok(),
                _ => reply.error(EIO),
            }
        } else {
            reply.error(ENOENT);
        }
    }

    /// Create file node.
    /// Create a regular file, character device, block device, fifo or socket node.
    fn mknod(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        mode: u32,
        umask: u32,
        rdev: u32,
        reply: ReplyEntry,
    ) {
        let file_type = mode & S_IFMT as u32;

        if file_type != S_IFREG as u32 && file_type != S_IFLNK as u32 && file_type != S_IFDIR as u32
        {
            log::warn!("mknod() implementation is incomplete. Only supports regular files, symlinks, and directories. Got {:o}", mode);
            reply.error(ENOSPC);
            return;
        }

        let actual_mode = mode & !umask;

        let path = name.to_str().unwrap();

        let mut ext4_file = Ext4File::new();
        let r = self.ext4.ext4_open(&mut ext4_file, path, "w+", true);
        match r {
            Ok(_) => {
                let attr = FileAttr {
                    ino: ext4_file.inode as u64,
                    size: 0,
                    blocks: 0,
                    atime: UNIX_EPOCH,
                    mtime: UNIX_EPOCH,
                    ctime: UNIX_EPOCH,
                    crtime: UNIX_EPOCH,
                    kind: match file_type {
                        S_IFREG => FileType::RegularFile,
                        S_IFCHR => FileType::CharDevice,
                        S_IFBLK => FileType::BlockDevice,
                        S_IFIFO => FileType::NamedPipe,
                        S_IFSOCK => FileType::Socket,
                        _ => FileType::RegularFile, // Default case, though it should never hit here
                    },
                    perm: actual_mode as u16,
                    nlink: 1,
                    uid: _req.uid(),
                    gid: _req.gid(),
                    rdev: rdev as u32,
                    flags: 0,
                    blksize: 4096,
                };
                reply.entry(&TTL, &attr, 0);
            }

            _ => {
                reply.error(EIO);
            }
        }
    }
}

fn time_now() -> (i64, u32) {
    time_from_system_time(&SystemTime::now())
}

fn time_from_system_time(system_time: &SystemTime) -> (i64, u32) {
    // Convert to signed 64-bit time with epoch at 0
    match system_time.duration_since(UNIX_EPOCH) {
        Ok(duration) => (duration.as_secs() as i64, duration.subsec_nanos()),
        Err(before_epoch_error) => (
            -(before_epoch_error.duration().as_secs() as i64),
            before_epoch_error.duration().subsec_nanos(),
        ),
    }
}

fn system_time_from_time(secs: i64, nsecs: u32) -> SystemTime {
    if secs >= 0 {
        UNIX_EPOCH + Duration::new(secs as u64, nsecs)
    } else {
        UNIX_EPOCH - Duration::new((-secs) as u64, nsecs)
    }
}

fn system_time_to_timestamp(time: SystemTime) -> u32 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs() as u32
}

fn timestamp_to_system_time(timestamp: u32) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(timestamp as u64)
}

fn system_time_to_secs(time: SystemTime) -> u32 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs() as u32
}

pub struct InodeAttr {
    mode: Option<u32>,
    uid: Option<u32>,
    gid: Option<u32>,
    size: Option<u64>,
    atime: Option<TimeOrNow>,
    mtime: Option<TimeOrNow>,
    ctime: Option<SystemTime>,
    crtime: Option<SystemTime>,
    chgtime: Option<SystemTime>,
    bkuptime: Option<SystemTime>,
    flags: Option<u32>,
}
impl Ext4Fuse {
    pub fn set_attr(&self, inode: u32, attr: &InodeAttr) {
        let mut inode_ref = Ext4InodeRef::get_inode_ref(self.ext4.self_ref.clone(), inode);

        let now = system_time_to_secs(SystemTime::now());

        if let Some(mode) = attr.mode {
            inode_ref.inner.inode.ext4_inode_set_mode(mode as u16);
        }
        if let Some(uid) = attr.uid {
            inode_ref.inner.inode.ext4_inode_set_uid(uid as u16);
        }
        if let Some(gid) = attr.gid {
            inode_ref.inner.inode.ext4_inode_set_gid(gid as u16);
        }
        if let Some(size) = attr.size {
            inode_ref.inner.inode.ext4_inode_set_size(size);
        }
        if let Some(atime) = &attr.atime {
            let secs = match atime {
                TimeOrNow::SpecificTime(t) => system_time_to_secs(*t),
                TimeOrNow::Now => now,
            };
            inode_ref.inner.inode.ext4_inode_set_atime(secs);
        }
        if let Some(mtime) = &attr.mtime {
            let secs = match mtime {
                TimeOrNow::SpecificTime(t) => system_time_to_secs(*t),
                TimeOrNow::Now => now,
            };
            inode_ref.inner.inode.ext4_inode_set_mtime(secs);
        }
        if let Some(ctime) = attr.ctime {
            let secs = system_time_to_secs(ctime);
            inode_ref.inner.inode.ext4_inode_set_ctime(secs);
        }
        if let Some(crtime) = attr.crtime {
            let secs = system_time_to_secs(crtime);
            inode_ref.inner.inode.ext4_inode_set_crtime(secs);
        }
        if let Some(flags) = attr.flags {
            inode_ref.inner.inode.ext4_inode_set_flags(flags);
        }

        inode_ref.write_back_inode();
    }
}

fn main() {
    log::set_logger(&SimpleLogger).unwrap();
    log::set_max_level(LevelFilter::Info);

    let disk = Arc::new(Disk {});
    let ext4 = Ext4::open(disk);
    let ext4_fuse = Ext4Fuse::new(ext4);
    let mountpoint = "/root/sync/ext4libtest/foo/";
    let mut options = vec![
        MountOption::RW,
        MountOption::FSName("ext4_test".to_string()),
    ];

    options.push(MountOption::AutoUnmount);
    options.push(MountOption::AllowRoot);
    fuser::mount2(ext4_fuse, mountpoint, &options).unwrap();
}
