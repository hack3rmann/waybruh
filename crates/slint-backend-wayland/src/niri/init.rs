use rustix::{
    io::Errno,
    net::{self, AddressFamily, SocketAddrUnix, SocketFlags, SocketType},
};
use std::{
    env,
    os::fd::OwnedFd,
    path::{Path, PathBuf},
    sync::LazyLock,
};

static SOCKET_PATH: LazyLock<Option<PathBuf>> = LazyLock::new(|| {
    let path = env::var_os("NIRI_SOCKET")?;
    Some(PathBuf::from(path))
});

pub fn socket_path() -> Option<&'static Path> {
    SOCKET_PATH.as_ref().map(PathBuf::as_ref)
}

pub fn connect(path: impl AsRef<Path>) -> Result<OwnedFd, Errno> {
    let path = path.as_ref();

    let sock = net::socket_with(
        AddressFamily::UNIX,
        SocketType::STREAM,
        SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
        None,
    )?;

    let addr = SocketAddrUnix::new(path)?;

    net::connect(&sock, &addr)?;

    Ok(sock)
}
