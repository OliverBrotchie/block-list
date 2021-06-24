use std::env;
use std::io::{self, Read};
use std::io::BufReader;
use std::io::prelude::*;
use std::fs::OpenOptions;
use std::fs::File;


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
        hosts = reqwest::blocking::get(
            if args[2] == "hosts" {
                "https://raw.githubusercontent.com/StevenBlack/hosts/master/hosts".to_string()
            } else {
                format!("https://raw.githubusercontent.com/StevenBlack/hosts/master/alternates/{}/hosts",args[2])
            }
        )?.text()?;

        // Test that the file is found
        if hosts == "404: Not Found" {
            println!("404: Not Found. See 'https://github.com/StevenBlack/hosts' for options. Usage: some-list")
        } else {
            hosts = hosts.chars()
                .skip(hosts.match_indices("# End of custom host records.").next().unwrap().0 + 29)
                .collect();
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

