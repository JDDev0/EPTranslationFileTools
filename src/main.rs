use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::process::ExitCode;

fn main() -> Result<ExitCode, Box<dyn Error>> {
    let mut args = env::args_os();

    let binary_name = args.next();
    let binary_name = binary_name.as_deref();

    let args = args.collect::<Vec<OsString>>();
    let args = args.as_array();

    let Some([reference, input, output]) = args else {
        println!("Usage: {} <reference file> <input file> <output file>", binary_name.
                map(|str| str.display().to_string()).
                unwrap_or("TranslationFileLineFixer".to_string()));

        return Ok(ExitCode::FAILURE);
    };

    let reference = File::open(reference)?;
    let input = File::open(input)?;
    let mut output = File::create(output)?;

    let reference = BufReader::new(reference).lines();
    let mut input = BufReader::new(input).lines();

    for line in reference {
        let line = line?;

        if line.trim().is_empty() {
            writeln!(output)?;
        }else if let Some(input) = input.next() {
            let input = input?;
            writeln!(output, "{input}")?;
        }
    }

    println!("Translation file was fixed!");

    Ok(ExitCode::SUCCESS)
}
