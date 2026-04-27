use std::{
    env,
    ffi::OsString,
    os::fd::OwnedFd,
    path::{Path, PathBuf},
};

use rustix::{
    io::Errno,
    net::{self, AddressFamily, SocketAddrUnix, SocketFlags, SocketType},
};

#[derive(Clone, Debug, PartialEq, Default)]
pub struct HyprlandSocketPaths {
    pub query: PathBuf,
    pub event: PathBuf,
}

pub fn get_socket_paths() -> Option<HyprlandSocketPaths> {
    let instance_signature = env::var_os("HYPRLAND_INSTANCE_SIGNATURE")?;

    let xdg_runtime_dir =
        env::var_os("XDG_RUNTIME_DIR").unwrap_or_else(|| OsString::from("/run/user/1000"));

    let mut hypr_dir = {
        let mut runtime = xdg_runtime_dir;
        runtime.push("/hypr");
        runtime
    };

    if !Path::new(&hypr_dir).exists() {
        hypr_dir.clear();
        hypr_dir.push("/tmp/hypr");
    }

    hypr_dir.push("/");
    hypr_dir.push(&instance_signature);

    if !Path::new(&hypr_dir).exists() {
        return None;
    }

    let query_path = {
        let mut path = hypr_dir.clone();
        path.push("/.socket.sock");
        PathBuf::from(path)
    };

    let event_path = {
        let mut path = hypr_dir;
        path.push("/.socket2.sock");
        PathBuf::from(path)
    };

    if !query_path.exists() || !event_path.exists() {
        return None;
    }

    Some(HyprlandSocketPaths {
        query: query_path,
        event: event_path,
    })
}

pub fn connect_socket(path: impl AsRef<Path>) -> Result<OwnedFd, Errno> {
    let path = path.as_ref();

    let sock = net::socket_with(
        AddressFamily::UNIX,
        SocketType::STREAM,
        SocketFlags::CLOEXEC,
        None,
    )?;

    let addr = SocketAddrUnix::new(path)?;

    net::connect(&sock, &addr)?;

    Ok(sock)
}
