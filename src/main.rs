use std::env;
use std::io::{self, Read};
use std::io::BufReader;
use std::io::prelude::*;
use std::fs::OpenOptions;
use std::fs::File;
use curl::easy::{Easy2, Handler, WriteError};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args:Vec<String> = env::args().collect();
    let mut hosts = String::new();
    let mut custom_hosts = String::new();

    // Get piped input
    if atty::isnt(atty::Stream::Stdin) {
        custom_hosts = io::stdin().lock().lines().map(|e| e.expect("Error with standard input.")).collect()
    }

    if args.len() == 3 {
        // Get the hosts file
        struct Collector(Vec<u8>);
        impl Handler for Collector {
            fn write(&mut self, data: &[u8]) -> Result<usize, WriteError> {
                self.0.extend_from_slice(data);
                Ok(data.len())
            }
        }

        let mut curl = Easy2::new(Collector(Vec::new()));
        curl.get(true)?;
        curl.url(&       
            if args[2] == "hosts" {
                "https://raw.githubusercontent.com/StevenBlack/hosts/master/hosts".to_string()
            } else {
                format!("https://raw.githubusercontent.com/StevenBlack/hosts/master/alternates/{}/hosts",args[2])
            }[..]
        )?;
        curl.perform()?;

        if curl.response_code()? == 200 {
            hosts = format!("{}",String::from_utf8_lossy(&curl.get_ref().0));
            hosts.replace_range(..hosts.match_indices("# End of custom host records.").next().unwrap().0 + 29, "");
        } else {
            println!("Http error: {}. See 'https://github.com/StevenBlack/hosts' for options; usage: some-list", curl.response_code()?);
        }
    }
    
    // Open file
    let file = OpenOptions::new()
        .read(true)
        .open(&args[1])?;
    
    // Read contents
    let mut buf_reader = BufReader::new(&file);
    let mut contents = String::new();
    buf_reader.read_to_string(&mut contents)?;

    // Remove current block list if it exists
    if contents.contains("# Block List") {
       contents.replace_range(contents.match_indices("# Block List").next().unwrap().0.., "")
    }

    // Replace file with new contents
    let mut file = File::create(&args[1])?;
    file.write_all(format!("{}# Block List\n{}\n{}",contents,custom_hosts,hosts).as_bytes())?;
    Ok(())
}