
///
/// A simple utility for creating animated GIF images from typescrips of terminal sessions.
///

use std::fs::File;
use std::io;
use std::io::BufRead;
use std::io::Read;
use std::io::Write;
use std::process;
use std::str;
use std::str::FromStr;
use std::thread::sleep;
use std::time::Duration;


// Conveniences for exiting gracefully when encountering an error result.
fn exit(message: &str) -> ! {
    println!("{}", message);
    process::exit(1);
}

trait Exit<T, Error> {
    fn or_exit(self, message: &str) -> T;
}

impl<T, Error: ::std::fmt::Display> Exit<T, Error> for Result<T, Error> {
    fn or_exit(self, message: &str) -> T {
        match self {
            Ok(value) => value,
            Err(error) => {
                exit(&format!("{}: {}", message, error));
            }
        }
    }
}


// Convenience method for executing a system command.
fn execute<S: AsRef<std::ffi::OsStr> + std::fmt::Display>(args: &[S]) {
    process::Command::new(&args[0])
        .args(&args[1..])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .or_exit(&format!("Failed to execute process \"{}\"", args[0]));
}

// A silly convenience method for opening a file and returning a reader.
fn reader<P: AsRef<std::path::Path>>(path: P) -> Result<std::io::BufReader<std::fs::File>, std::io::Error> {
    File::open(path)
        .and_then(|file| Ok(io::BufReader::new(file)))
}


fn main() {

    // Check dependencies.
    execute(&["script", "-V"]);
    execute(&["xwd", "-help"]);
    execute(&["convert", "-version"]);

    // Check arguments.
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        exit("rusttygif <timingfile> <typescript>");
    }

    // Create output directory.
    let output_path = "output";
    execute(&["mkdir", "-p", &output_path]);

    // Open input files.
    let timing = reader(&args[1]).or_exit("Could not open timing file");
    let mut script = reader(&args[2]).or_exit("Could not open script file");

    // Ignore the first line containing the script's timestamp.
    let mut timestamp = String::new();
    script.read_line(&mut timestamp).or_exit("Could not read from script file");

    // Read the timing and script files, line by line, replaying the script, taking screenshots,
    // and building the command needed to assemble all screenshots into the animation as we go.
    let mut frame: usize = 1;
    let mut buffer = vec![0u8; 0];
    let mut convert: Vec<String> = Vec::new();
    convert.push(String::from("convert"));

    for line in timing.lines() {
        let l = line.or_exit("Error reading line from timing file");

        // Line format is: <delay in seconds : float> <size in bytes : integer>
        let parts: Vec<&str> = l.split(" ").collect();
        let delay = f64::from_str(&parts[0]).or_exit("Error reading delay in timing file");
        let size = usize::from_str(&parts[1]).or_exit("Error reading size in timing file");

        convert.push(String::from("-delay"));
        convert.push(parts[0].to_string());

        // Decode the delay from a floating point into integer seconds and nanosecond parts.
        let delay_parts: Vec<&str> = parts[0].split(".").collect();
        let delay_secs = u64::from_str(&delay_parts[0]).or_exit("Error parsing delay in timing file");
        let delay_nsecs = (delay - delay_secs as f64) * 1.0e9;
        let duration = Duration::new(delay_secs, delay_nsecs as u32);
        sleep(duration);

        // The first time through, don't print anything.
        // XXX  Explain why this is correct.
        if buffer.len() > 0 {
            let slice = buffer.as_mut_slice();
            let output = str::from_utf8(slice).or_exit("Error converting script output to a string");
            print!("{}", output);
            io::stdout().flush().or_exit("Could not flush stdout");

            let img_path = format!("{}/img-{}.xwd", output_path, frame);
            let window = &std::env::var("WINDOWID").or_exit("Could not determine window ID");
            execute(&["xwd", "-id", window, "-out", &img_path]);
            convert.push(img_path);
        }

        // Explicitly set the buffer size to be exactly the number of bytes we want to read.
        // XXX  Figure out a cleaner way to do this.
        buffer.resize(size, 0);
        let mut buffer_slice = buffer.as_mut_slice();
        script.read_exact(buffer_slice).or_exit("Could not read from script file");

        frame += 1;
    }

    convert.push(String::from("-layers"));
    convert.push(String::from("Optimize"));
    convert.push(format!("{}/output.gif", output_path));

    // Assemble and convert the animation.
    execute(&convert);

    // Launch the default browser to view the image.
    execute(&["exo-open", "--launch", "WebBrowser", &format!("{}/output.gif", output_path)]);

    println!("");

}