use nix::sys::socket::*;
use nix::errno::Errno;
use std::os::fd::RawFd;
use nix::unistd::{close, read, write};
use std::str::FromStr;
use std::env;

const K_MAX_MSG: usize = 4096;

// Guard against partial writes
fn write_full(fd: RawFd, wbuf: &mut[u8]) -> Result<usize, Errno> {
    let mut buf_start = 0;
    let mut n = wbuf.len();
    while n > 0 {
        match write(fd, &mut wbuf[buf_start..]) {
            Ok(rv) => {
                if rv <= 0 {
                    println!("Zero bytes written");
                    return Err(Errno::EIO);
                }
                assert!(rv <= n);
                n -= rv;
                buf_start += rv;
            },
            Err(e) => {
                println!("Error while writing {}", e);
                return Err(e);
            }
        }
    }
    Ok(n)
}

// Guard against partial reads
fn read_full(fd: RawFd, rbuf: &mut[u8]) -> Result<usize, Errno> {
    let mut buf_start = 0;
    let mut n = rbuf.len();
    while n > 0 {
        match read(fd, &mut rbuf[buf_start..]) {
            Ok(rv) => {
                if rv <= 0 {
                    println!("Zero bytes read");
                    return Err(Errno::EIO);
                }
                assert!(rv <= n);
                n -= rv;
                buf_start += rv;
            },
            Err(e) => {
                println!("Error while reading {}", e);
                return Err(e);
            }
        }
    }
    Ok(n)
}

fn send_request(fd: RawFd, text: &str) -> Result<usize, Errno> {
    let reply: &[u8] = text.as_bytes();
    let mut wbuf = [0; K_MAX_MSG];
    let length = u32::try_from(reply.len()).unwrap();
    wbuf[0..4].copy_from_slice(&length.to_le_bytes());
    wbuf[4..4 + reply.len()].copy_from_slice(reply);
    write_full(fd, &mut wbuf[0..4 + reply.len()])
}

fn read_response(fd: RawFd) -> Result<usize, Errno> {
    let mut len_buf: [u8; 4] = [0; 4];
    let mut res_code_buf: [u8; 4] = [0; 4];
    let mut length;
    let res_code: u32;
    let mut rbuf: [u8; K_MAX_MSG] = [0; K_MAX_MSG];
    match read_full(fd, &mut len_buf) {
        Ok(_) => {
            length = u32::from_le_bytes(len_buf);
        },
        Err(e) => {
            println!("read() error {}", e);
            return Err(e);
        }
    }

    match read_full(fd, &mut res_code_buf) {
        Ok(_) => {
            res_code = u32::from_le_bytes(res_code_buf);
        },
        Err(e) => {
            println!("read() error {}", e);
            return Err(e);
        }
    }

    length -= 4;
    if length > 0 {
        match read_full(fd, &mut rbuf[..length.try_into().unwrap()]) {
            Ok(_) => {
                println!("[{}] {}", res_code, String::from_utf8(rbuf[..length.try_into().unwrap()].to_vec()).unwrap());
            }
            Err(e) => {
                println!("read() error {}", e);
                return Err(e);
            }
        }
    } else {
        println!("[{}]", res_code);
    }
    Ok(0)
}

fn query(fd: RawFd, text: &str) -> Result<usize, Errno> {
    send_request(fd, text)?;
    read_response(fd)
}

fn main() {
    let fd = socket(AddressFamily::Inet, SockType::Stream, SockFlag::empty(), None);
    let mut args: Vec<String> = env::args().collect();
    args.remove(0);
    let req = args.join(" ");
    match fd {
        Ok(fd) => {
            let localhost = SockaddrIn::from_str("127.0.0.1:8081").unwrap();
            match connect(fd, &localhost) {
                Ok(()) => {
                    if let Err(e) = send_request(fd, &req) {
                        println!("Error {} sending request {}", e, req);
                        let _ = close(fd);
                        return;
                    }
                    if let Err(_) = read_response(fd) {
                        return;
                    }
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
