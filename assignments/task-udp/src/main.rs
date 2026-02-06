use std::{
    collections::HashMap,
    error::Error,
    io::{Read, Write},
    net::{TcpStream, UdpSocket},
    time::{Duration, Instant},
};

const TCP_PORT: u16 = 12345;
const UDP_PORT: u16 = 20000;
const PAYLOAD_SIZE: usize = 1200;

struct Datagram {
    seq: u32,
    data: Vec<u8>,
    timestamp: Instant,
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("Task-UDP starting"); 
    let start = Instant::now();
    let mut tcp_stream = TcpStream::connect(format!("10.0.0.3:{}", TCP_PORT))?;
    tcp_stream.write_all(b"TASK-UDP ford")?;

    let mut buf = [0u8; 1024];
    let bytes_read = tcp_stream.read(&mut buf)?;
    let response = String::from_utf8_lossy(&buf[..bytes_read]);
    
    let parts: Vec<&str> = response.split_whitespace().collect();
    let size: usize = parts[0].parse()?;
    let character = parts[1].as_bytes()[0];

    println!("Task: Send {} bytes of '{}'", size, parts[1]);

    let checknum = transmit_loop(size, character)?;
    let duration = start.elapsed();

    println!("Size: {} -- Checknum: {} -- Duration: {:?}", size, checknum, duration);

    Ok(())
}

fn transmit_loop(total_size: usize, character: u8) -> Result<u8, Box<dyn Error>> {
    let udp_socket = UdpSocket::bind("0.0.0.0:0")?;
    // Set a timeout for receing ACKs
    udp_socket.set_read_timeout(Some(Duration::from_millis(600)))?;
    let server_addr = format!("10.0.0.3:{}", UDP_PORT);

    // Define control variables 
    let mut unacked: HashMap<u32, Datagram> = HashMap::new();
    let mut bytes_sent: usize = 0;
    let mut next_seq: u32 = 1;
    let mut last_acked_seq: u32 = 0;
    let mut checknum: u8 = 0;
    // Define congestion control variables
    let mut cwnd: usize = 5; 
    let mut ssthresh: usize = 64;

    while (last_acked_seq as usize * PAYLOAD_SIZE) < total_size && bytes_sent <= total_size + PAYLOAD_SIZE {
        // If there are no unacked datagrams and we sent all data, break early
        if unacked.is_empty() && bytes_sent >= total_size {
            break; 
        }
        // Send packets while we have data to send and window allows
        while bytes_sent < total_size && unacked.len() < cwnd {
            // Calculate how much data to send in this packet
            let remaining = total_size - bytes_sent;
            let chunk_size = std::cmp::min(PAYLOAD_SIZE, remaining);
            // Create datagram 
            let mut datagram = Vec::with_capacity(6 + chunk_size);
            datagram.extend_from_slice(&next_seq.to_be_bytes());
            datagram.extend_from_slice(&(chunk_size as u16).to_be_bytes());
            datagram.extend_from_slice(&vec![character; chunk_size]);
            // Send datagram
            udp_socket.send_to(&datagram, &server_addr)?;

            // Final datagram logging
            if chunk_size < PAYLOAD_SIZE {
                println!("Sending last datagram seq: {} size: {}", next_seq, chunk_size);
            }
            
            // Store datagram in unacked map
            unacked.insert(next_seq, Datagram {
                seq: next_seq,
                data: datagram,
                timestamp: Instant::now(),
            });

            // Update control variables
            next_seq += 1;
            bytes_sent += chunk_size;
        }

        // Create buffer for receiving ACKs
        let mut ack_buf = [0u8; 5];
        // Match on Ack reception and handle timeouts for retransmissions
        match udp_socket.recv_from(&mut ack_buf) {
            // Handle ACK reception
            Ok(_) => {
                // 4 bytes for sequence number, 1 byte for checknum
                let ack_seq = u32::from_be_bytes(ack_buf[0..4].try_into()?);
                checknum = ack_buf[4];

                if ack_seq > last_acked_seq {
                    // Remove all datagrams strictly less than or equal to ack_seq
                    unacked.retain(|&seq, _| seq > ack_seq);
                    last_acked_seq = ack_seq;

                    // Congestion control implemetation
                    // Increase congestion window if we are below the threshold, otherwise grow linearly
                    if cwnd < ssthresh {
                        cwnd += 1;
                    } else {
                         cwnd += 1 / cwnd.max(1); 
                    }
                }
            }
            // Handle timeouts and retransmissions
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => {
                println!("timeout occured, checking for unacked datagrams...");
                let now = Instant::now();
                let mut retransmitted = false;

                // Sort datagrams by sequence number
                let mut sorted_datagrams: Vec<&mut Datagram> = unacked.values_mut().collect();
                sorted_datagrams.sort_by_key(|d| d.seq);

                // Limit retransmissions to prevent network overflow
                let mut limit = 10; 
                // Check for datagrams that have timed out and retransmit them
                for dg in sorted_datagrams {
                    // Increase timeout threshold slightly for retransmits to avoid spamming
                    if now.duration_since(dg.timestamp) > Duration::from_millis(800) {
                        // Retransmit datagram if we are within the retransmission limit
                        if limit > 0 {
                            //println!("retransmissing seq: {}", dg.seq);
                            udp_socket.send_to(&dg.data, &server_addr)?;
                            dg.timestamp = Instant::now();
                            retransmitted = true;
                            limit -= 1;
                        } else {break;}
                    }
                }
                // Reduce congestion window and threshold if we had to retransmit
                if retransmitted {
                    ssthresh = (cwnd / 2).max(2);
                    cwnd = 2; 
                    //println!("congestion window reduced to {}", cwnd);
                }
            }
            // Handle other errors
            Err(e) => return Err(Box::new(e)),
        }
    }

    Ok(checknum)
}