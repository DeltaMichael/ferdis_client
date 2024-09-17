use nix::sys::socket::*;
use nix::errno::Errno;
use std::os::fd::RawFd;
use nix::unistd::{close, read, write};
use std::str::FromStr;

const K_MAX_MSG: usize = 4096;

fn query(fd: RawFd, text: &str) -> Result<usize, Errno> {
    let reply: &[u8] = text.as_bytes();
    let mut wbuf = [0; K_MAX_MSG];
    let length = u32::try_from(reply.len()).unwrap();
    wbuf[0..4].copy_from_slice(&length.to_le_bytes());
    wbuf[4..4 + reply.len()].copy_from_slice(reply);
    match write(fd, &mut wbuf[0..4 + reply.len()]) {
        Ok(rv) => {
            if rv <= 0 {
                println!("Zero bytes written");
                return Err(Errno::EIO);
            }
        },
        Err(e) => {
            println!("Error while writing {}", e);
            return Err(e);
        }
    }

    let mut len_buf: [u8; 4] = [0; 4];
    let length;
    let mut rbuf: [u8; K_MAX_MSG] = [0; K_MAX_MSG];
    match read(fd, &mut len_buf) {
        Ok(rv) => {
            length = u32::from_le_bytes(len_buf);
            if rv <= 0 {
                return Err(Errno::EIO);
            }
        },
        Err(e) => {
            println!("read() error {}", e);
            return Err(e);
        }
    }

    match read(fd, &mut rbuf[..length.try_into().unwrap()]) {
        Ok(rv) => {
            if rv <= 0 {
                return Err(Errno::EIO);
            }
            println!("Server says {}", String::from_utf8(rbuf[..length.try_into().unwrap()].to_vec()).unwrap());
        }
        Err(e) => {
            println!("read() error {}", e);
            return Err(e);
        }
    }
    Ok(0)
}

fn main() {
    let fd = socket(AddressFamily::Inet, SockType::Stream, SockFlag::empty(), None);

    match fd {
        Ok(fd) => {
            let localhost = SockaddrIn::from_str("127.0.0.1:8081").unwrap();
            match connect(fd, &localhost) {
                Ok(()) => {
                    let _ = query(fd, "hello1");
                    let _ = query(fd, "hello2");
                    let _ = query(fd, "hello3");
                },
                Err(e) => {
                    println!("Error connecting to server {}", e);
                }
            }
            let _ = close(fd);
        },
        Err(e) => { println!("Error opening socket {}", e) }
    }
}
