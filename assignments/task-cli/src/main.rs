use std::{
    env,
    io::{Read, Write},
    net::TcpStream,
    time::Instant,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Task-CLI starting");

    // Start clock to measure the time it takes to finish transmission
    let start = Instant::now();

    // Creating a vector of command line arguments
    let args: Vec<String> = env::args().collect();
    // Checking if the number of arguments is correct
    if args.len() != 2 {
        eprintln!("Arguments: host:port");
        return Err("Invalid number of arguments".into());
    }
    println!("{}", args[1].as_str());

    // Connect to the server specified in the command line arguments
    let mut socket = TcpStream::connect(&args[1])?;
    // Send the TASK-CLI cheetah as the message
    socket.write(b"TASK-CLI cheetah")?;

    // Creating a vector to store all received data
    let mut data = Vec::new();
    // Read all data from the socket
    socket.read_to_end(&mut data)?;
    // println!("{:x?}", data);

    // Measure the duration taken for the transmission
    let duration = start.elapsed();
    // Get the total size of the received data in bytes
    let total_size = data.len();

    // Get the last 8 bytes of the received data as a string
    let last_bytes: String = String::from_utf8_lossy(&data[total_size.saturating_sub(8)..]).to_string();
    
    // println!("{}", String::from_utf8_lossy(&data[total_size.saturating_sub(100)..]));

    println!("Total size: {} -- Last bytes: {} -- Duration: {:?}", total_size, last_bytes, duration);

    Ok(())
}