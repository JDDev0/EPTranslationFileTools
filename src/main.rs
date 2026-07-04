use std::env;
use std::env::ArgsOs;
use std::error::Error;
use std::ffi::OsString;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::process::ExitCode;

fn main() -> Result<ExitCode, Box<dyn Error>> {
    let mut args = env::args_os();

    let binary_name = args.next();
    let binary_name = binary_name.as_deref().map(|str| str.display().to_string()).unwrap_or("TranslationFileLineFixer".to_string());

    let subcommand_name = args.next();
    let subcommand_name = subcommand_name.as_deref().map(|str| str.display().to_string());
    match subcommand_name.as_deref() {
        Some("fix_lines") => fix_lines(&binary_name, args),
        Some("update_templates") => update_templates(&binary_name, args),
        _ => {
            println!("\
            Usage: {binary_name} <subcommand> <args...>\n\
            Supported subcommands:\n\
            - \"fix_lines\": Adds missing whitespaces to translation files\n\
            - \"update_templates\": Combines \"block.energizedpower.\" and \"item.energizedpower.\" to \"_template.epblock.\" and\
                \"block.energizedpower.\", \"item.energizedpower.\", and \"container.energizedpower.\" to \"_template.epcontainer.\"");

            Ok(ExitCode::FAILURE)
        }
    }
}

fn fix_lines(binary_name: &str, args: ArgsOs) -> Result<ExitCode, Box<dyn Error>> {
    let args = args.collect::<Vec<OsString>>();
    let args = args.as_array();

    let Some([reference, input, output]) = args else {
        println!("Usage: {binary_name} fix_lines <reference file> <input file> <output file>");

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

fn update_templates(binary_name: &str, args: ArgsOs) -> Result<ExitCode, Box<dyn Error>> {
    let args = args.collect::<Vec<OsString>>();
    let args = args.as_array();

    let Some([input, output]) = args else {
        println!("Usage: {binary_name} update_templates <input file> <output file>");

        return Ok(ExitCode::FAILURE);
    };

    let input = File::open(input)?;
    let mut output = File::create(output)?;

    let mut line_buffer = Vec::new();
    let mut has_item = false;
    let mut has_block = false;
    let mut has_container = false;

    fn write_and_clear_combined_translations_line_buffer(
        output: &mut File, line_buffer: &mut Vec<String>,
        has_item: &mut bool, has_block: &mut bool, has_container: &mut bool,
    ) -> Result<(), Box<dyn Error>> {
        if *has_item && *has_block {
            let line_with_block_key = &**line_buffer.iter().find(|line| line.contains("\"block.energizedpower.")).unwrap();
            let line = if *has_container {
                line_with_block_key.replace("\"block.energizedpower.", "\"_template.epcontainer.")
            }else {
                line_with_block_key.replace("\"block.energizedpower.", "\"_template.epblock.")
            };

            line_buffer.clear();
            writeln!(output, "{line}")?;
        }else {
            for line in line_buffer.drain(..) {
                writeln!(output, "{line}")?;
            }
        }

        *has_item = false;
        *has_block = false;
        *has_container = false;

        Ok(())
    }

    let input = BufReader::new(input).lines();
    for line in input {
        let line = line?;

        if line.contains("\"item.energizedpower.") {
            if has_item {
                write_and_clear_combined_translations_line_buffer(
                    &mut output, &mut line_buffer,
                    &mut has_item, &mut has_block, &mut has_container,
                )?;
            }

            has_item = true;
            line_buffer.push(line);
        }else if line.contains("\"block.energizedpower.") {
            if has_block {
                write_and_clear_combined_translations_line_buffer(
                    &mut output, &mut line_buffer,
                    &mut has_item, &mut has_block, &mut has_container,
                )?;
            }

            has_block = true;
            line_buffer.push(line);
        }else if line.contains("\"container.energizedpower.") &&
                //Ignore special case: Separate translations for Item Conveyor Belt machine containers
                !line.contains("item_conveyor_belt") {
            if has_container {
                write_and_clear_combined_translations_line_buffer(
                    &mut output, &mut line_buffer,
                    &mut has_item, &mut has_block, &mut has_container,
                )?;
            }

            has_container = true;
            line_buffer.push(line);
        }else {
            if has_item || has_block || has_container {
                write_and_clear_combined_translations_line_buffer(
                    &mut output, &mut line_buffer,
                    &mut has_item, &mut has_block, &mut has_container,
                )?;
            }

            writeln!(output, "{line}")?;
        }
    }

    write_and_clear_combined_translations_line_buffer(
        &mut output, &mut line_buffer,
        &mut has_item, &mut has_block, &mut has_container,
    )?;

    println!("Translation file was fixed!");

    Ok(ExitCode::SUCCESS)
}
