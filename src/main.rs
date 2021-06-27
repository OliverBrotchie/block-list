use std::fs;
use std::env;
use std::path::Path;
use std::io::{self};
use std::io::prelude::*;
use curl::easy::{Easy2, Handler, WriteError};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args:Vec<String> = env::args().collect();

    // Test for incorrect usage or help
    if args.len() < 2 || &args[1] == "-h" || !Path::new(&args[1]).exists(){
        if args.len() < 2 || !(&args[1] == "-h") { println!("Error! Please specify a valid path to your desired hosts file.\n") }
        println!("USAGE:\n\tblock-list /path/to/hosts/file your-desired-block-list");
    } else {
        let mut hosts = String::new();
        let mut custom_hosts = String::new();

        // Get piped input
        if atty::isnt(atty::Stream::Stdin) {
            custom_hosts = io::stdin().lock().lines().map(|e| format!("\n{}",e.expect("Error with standard input."))).collect()
        }

        if args.len() == 3 {
            // Get the new hosts file
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
        
        // Read current hosts file to string
        let mut contents = fs::read_to_string(&args[1])?;
        
        // Remove current block list if it exists
        if contents.contains("# Block List") {
            contents.replace_range(contents.match_indices("# Block List").next().unwrap().0.., "")
        }

        fs::write(&args[1], format!("{}# Block List\n{}\n{}",contents,custom_hosts,hosts))?;
        println!("Block List updated! 🔒")
    }
    Ok(())
}