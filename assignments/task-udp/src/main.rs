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
    let mut tcp_stream = TcpStream::connect(format!("127.0.0.1:{}", TCP_PORT))?;
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
    udp_socket.set_read_timeout(Some(Duration::from_millis(200)))?;
    let server_addr = format!("127.0.0.1:{}", UDP_PORT);

    let mut datagrams_unacked: HashMap<u32, Datagram> = HashMap::new();
    let mut bytes_sent: usize = 0;
    let mut next_seq: u32 = 1;
    let mut last_acked_seq: u32 = 0;
    let mut checknum: u8 = 0;
    let mut congestion_window: usize = 2; 

    while (last_acked_seq as usize * PAYLOAD_SIZE) < total_size || !datagrams_unacked.is_empty() {
        while bytes_sent < total_size && datagrams_unacked.len() < congestion_window {
            //Define the size of the current chunk
            let chunk_size = std::cmp::min(PAYLOAD_SIZE, total_size - bytes_sent);
            // Define the datagram. Big endian order: 4 bytes for seq, 2 bytes for payload size, then the payload
            let mut datagram = Vec::with_capacity(6 + chunk_size);
            datagram.extend_from_slice(&next_seq.to_be_bytes());
            datagram.extend_from_slice(&(chunk_size as u16).to_be_bytes());
            datagram.extend_from_slice(&vec![character; chunk_size]);
            
            // Send the datagram
            udp_socket.send_to(&datagram, &server_addr)?;
            // Store the datagram in the unacked map with a timestamp for potential retransmission
            datagrams_unacked.insert(next_seq, Datagram {seq: next_seq, data: datagram, timestamp: Instant::now(),});
            // Update for next datagram
            next_seq += 1;
            bytes_sent += chunk_size;
        }

        // Create ACK buffer
        let mut ack_buf = [0u8; 5];
        match udp_socket.recv_from(&mut ack_buf) {
            Ok(_) => {
                let ack_seq = u32::from_be_bytes(ack_buf[0..4].try_into()?);
                checknum = ack_buf[4];

                if ack_seq > last_acked_seq {
                    datagrams_unacked.retain(|&seq, _| seq > ack_seq);
                    last_acked_seq = ack_seq;
                    congestion_window += 1;
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => {
                println!("!! TIMEOUT !! Retransmitting...");
            
                let now = Instant::now();
                let mut retransmitted = false;
            
                let mut sorted_packets: Vec<&mut Datagram> = datagrams_unacked.values_mut().collect();
                sorted_packets.sort_by_key(|d| d.seq);
            
                let mut retransmit_limit = 10; 
            
                for dg in sorted_packets {
                    if now.duration_since(dg.timestamp) > Duration::from_millis(1000) {
                        if retransmit_limit > 0 {
                            println!("-> Retransmitting SEQ: {}", dg.seq);
                            udp_socket.send_to(&dg.data, &server_addr)?;
                            dg.timestamp = Instant::now();
                            retransmitted = true;
                            retransmit_limit -= 1;
                        } else {
                            break;
                        }
                    }
                }
                
                if retransmitted {
                    congestion_window = (congestion_window / 2).max(2);
                    println!("!! Congestion Window reduced to {}", congestion_window);
                }
            }
            Err(e) => return Err(Box::new(e)),
        }
    }
    Ok(checknum)
}