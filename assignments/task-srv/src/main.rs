use std::env;
use std::error::Error;
use std::net::SocketAddr;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    task,
};

async fn handle_client(mut socket: TcpStream, addr: SocketAddr) {
    loop {
        // Read the length of data transmitted by client (first 32 bits)
        let mut data_length = [0u8; 4];
        if socket.read_exact(&mut data_length).await.is_err() {
            println!("Client {} was disconnected", addr);
            return;
        }

        // Convert the 4 bytes to a u32 integer
        let all_bytes = u32::from_be_bytes(data_length);

        // Creating a buffer to store the data byte sent by client
        let mut byte_buf = [0u8; 1];
        if socket.read_exact(&mut byte_buf).await.is_err() {
            println!("Client {} was disconnected", addr);
            return;
        }

        // Extract data byte
        let data_byte = byte_buf[0];

        // Create a vector of repeated data_bytes
        let mut buffer = Vec::new();
        for _ in 0..all_bytes {
            buffer.push(data_byte);
        }

        // Send the data back to the client
        if socket.write_all(&buffer).await.is_err() {
            println!("Unable to write to {}", addr);
            return;
        }
        println!("Wrote {} bytes of byte {}", all_bytes, data_byte);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Collect command line arguments
    let args: Vec<String> = env::args().collect();

    // Print error and exit if the number of arguments is incorrect
    if args.len() != 3 {
        eprintln!("Usage: <keyword> <port>");
        std::process::exit(1);
    }

    let keyword = &args[1];
    let port: u16 = args[2].parse().expect("Invalid port number");

    // let ip = "127.0.0.1";
    // let agent_address = "127.0.0.1:12345";

    let ip = "0.0.0.0";
    let agent_address = "10.0.0.3:12345";

    // Start the TCP listener
    let server_address = format!("{}:{}", ip, port);
    let listener = TcpListener::bind(&server_address).await?;
    println!("Listening on {}", server_address);

    // Build the control message
    let control_message = format!( "TASK-SRV {} {}:{}", keyword, ip, port);

    // Sending control message to the agent
    let mut agent_socket = TcpStream::connect(agent_address).await?;
    agent_socket.write_all(control_message.as_bytes()).await?;
    println!("Control message {} sent to agent", control_message);

    loop {
        let (socket, addr) = listener.accept().await?;
        println!("{} connected", addr);
        // Spawn a new task to handle multiple clients concurrently
        task::spawn(handle_client(socket, addr));
    }
}