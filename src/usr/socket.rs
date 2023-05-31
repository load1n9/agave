use crate::{sys, usr, debug};
use crate::api::console::Style;
use crate::api::clock;
use crate::api::io;
use crate::api::process::ExitCode;
use crate::api::random;
use crate::api::syscall;

use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::str::{self, FromStr};
use smoltcp::iface::SocketSet;
use smoltcp::socket::tcp;
use smoltcp::time::Instant;
use smoltcp::wire::IpAddress;

pub fn main(args: &[&str]) -> Result<(), ExitCode> {
    let mut listen = false;
    let mut prompt = false;
    let mut verbose = false;
    let mut read_only = false;
    let mut interval = 0.0;
    let mut next_arg_is_interval = false;
    let mut args: Vec<&str> = args.iter().filter_map(|arg| {
        match *arg {
            "-l" | "--listen" => {
                listen = true;
                None
            }
            "-p" | "--prompt" => {
                prompt = true;
                None
            }
            "-r" | "--read-only" => {
                read_only = true;
                None
            }
            "-v" | "--verbose" => {
                verbose = true;
                None
            }
            "-i" | "--interval" => {
                next_arg_is_interval = true;
                None
            }
            _ if next_arg_is_interval => {
                next_arg_is_interval = false;
                if let Ok(i) = arg.parse() {
                    interval = i;
                }
                None
            }
            _ => {
                Some(*arg)
            }
        }
    }).collect();
    if prompt {
        println!("MOROS Socket v0.1.0\n");
    }

    let required_args_count = if listen { 2 } else { 3 };

    if args.len() == required_args_count - 1 {
        if let Some(i) = args[1].find(':') { // Split <host> and <port>
            let (host, path) = args[1].split_at(i);
            args[1] = host;
            args.push(&path[1..]);
        }
    }

    if args.len() != required_args_count {
        help();
        return Err(ExitCode::UsageError);
    }

    let host = if listen { "0.0.0.0" } else { args[1] };
    let port: u16 = args[required_args_count - 1].parse().expect("Could not parse port");

    let address = if host.ends_with(char::is_numeric) {
        IpAddress::from_str(host).expect("invalid address format")
    } else {
        match usr::host::resolve(host) {
            Ok(ip_addr) => {
                ip_addr
            }
            Err(e) => {
                error!("Could not resolve host: {:?}", e);
                return Err(ExitCode::Failure);
            }
        }
    };

    #[derive(Debug)]
    enum State { Connecting, Sending, Receiving }
    let mut state = State::Connecting;

    if let Some((ref mut iface, ref mut device)) = *sys::net::NET.lock() {
        let mut sockets = SocketSet::new(vec![]);
        let tcp_rx_buffer = tcp::SocketBuffer::new(vec![0; 1024]);
        let tcp_tx_buffer = tcp::SocketBuffer::new(vec![0; 1024]);
        let tcp_socket = tcp::Socket::new(tcp_rx_buffer, tcp_tx_buffer);
        let tcp_handle = sockets.add(tcp_socket);

        loop {
            if sys::console::end_of_text() || sys::console::end_of_transmission() {
                eprintln!();
                return Err(ExitCode::Failure);
            }

            let timestamp = Instant::from_micros((clock::realtime() * 1000000.0) as i64);
            iface.poll(timestamp, device, &mut sockets);
            let socket = sockets.get_mut::<tcp::Socket>(tcp_handle);
            let cx = iface.context();

            if verbose {
                debug!("*********************************");
                debug!("APP State: {:?}", state);
                debug!("TCP State: {:?}", socket.state());
                debug!("is active: {}", socket.is_active());
                debug!("is open: {}", socket.is_open());
                debug!("can recv: {}", socket.can_recv());
                debug!("can send: {}", socket.can_send());
                debug!("may recv: {}", socket.may_recv());
                debug!("may send: {}", socket.may_send());
            }

            state = match state {
                State::Connecting if !socket.is_active() => {
                    if listen { // Listen to a local port
                        if !socket.is_open() {
                            if verbose {
                                debug!("Listening to {}", port);
                            }
                            socket.listen(port).unwrap();
                        }
                    } else { // Connect to a remote port
                        let local_port = 49152 + random::get_u16() % 16384;
                        if verbose {
                            debug!("Connecting to {}:{}", address, port);
                        }
                        if socket.connect(cx, (address, port), local_port).is_err() {
                            error!("Could not connect to {}:{}", address, port);
                            return Err(ExitCode::Failure);
                        }
                    }
                    State::Receiving
                }
                State::Sending if socket.can_recv() => {
                    if verbose {
                        debug!("Sending -> Receiving");
                    }
                    State::Receiving
                }
                State::Sending if socket.can_send() && socket.may_recv() => {
                    if !read_only {
                        if verbose {
                            debug!("Sending ...");
                        }
                        if prompt {
                            // Print prompt
                            print!("{}>{} ", Style::color("Cyan"), Style::reset());
                        }
                        let line = io::stdin().read_line();
                        if line.is_empty() {
                            socket.close();
                        } else {
                            let line = line.replace("\n", "\r\n");
                            socket.send_slice(line.as_ref()).expect("cannot send");
                        }
                    }
                    State::Receiving
                }
                State::Receiving if socket.can_recv() => {
                    if verbose {
                        debug!("Receiving ...");
                    }
                    socket.recv(|data| {
                        let contents = String::from_utf8_lossy(data);
                        print!("{}", contents.replace("\r\n", "\n"));
                        (data.len(), ())
                    }).unwrap();
                    State::Receiving
                }
                _ if socket.state() == tcp::State::SynSent || socket.state() == tcp::State::SynReceived => {
                    state
                }
                State::Receiving if !socket.may_recv() && !listen => {
                    if verbose {
                        debug!("Break from response");
                    }
                    break;
                }
                State::Receiving if socket.can_send() => {
                    if verbose {
                        debug!("Receiving -> Sending");
                    }
                    State::Sending
                }
                _ if !socket.is_active() && !listen => {
                    if verbose {
                        debug!("Break from inactive");
                    }
                    break;
                }
                _ => state
            };

            if interval > 0.0 {
                syscall::sleep(interval);
            }
            if let Some(wait_duration) = iface.poll_delay(timestamp, &sockets) {
                syscall::sleep((wait_duration.total_micros() as f64) / 1000000.0);
            }
        }
        Ok(())
    } else {
        Err(ExitCode::Failure)
    }
}

fn help() {
    let csi_option = Style::color("LightCyan");
    let csi_title = Style::color("Yellow");
    let csi_reset = Style::reset();
    println!("{}Usage:{} socket {}[<host>] <port>{1}", csi_title, csi_reset, csi_option);
    println!();
    println!("{}Options:{}", csi_title, csi_reset);
    println!("  {0}-l{1}, {0}--listen{1}             Listen to a local port", csi_option, csi_reset);
    println!("  {0}-v{1}, {0}--verbose{1}            Increase verbosity", csi_option, csi_reset);
    println!("  {0}-p{1}, {0}--prompt{1}             Display prompt", csi_option, csi_reset);
    println!("  {0}-r{1}, {0}--read-only{1}          Read only connexion", csi_option, csi_reset);
    println!("  {0}-i{1}, {0}--interval <time>{1}    Wait <time> between packets", csi_option, csi_reset);
}
