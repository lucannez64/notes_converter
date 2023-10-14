use rayon::prelude::*;
use regex::Captures;
use regex::Regex;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::Command;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    let mut filename = None;
    let cwd = std::env::current_dir()?;
    if args.len() > 1 {
        if args[1] == "--single-file" {
            if args.len() > 2 {
                filename = Some(PathBuf::from(args[2].clone()));
            } else {
                eprintln!("--single-file requires a filename argument");
                std::process::exit(1);
            }
        }
    }
    let files = match filename {
        Some(f) => fs::read_dir(".")?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry.path()
                    == Path::new(".\\").join(f.strip_prefix(&cwd).unwrap_or(&f).to_str().unwrap())
            })
            .collect::<Vec<_>>(),
        None => fs::read_dir(".")?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                !entry
                    .path()
                    .to_str()
                    .unwrap_or_default()
                    .ends_with(".comp.md")
            })
            .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "md"))
            .collect::<Vec<_>>(),
    };

    files.par_iter().for_each(|file| {
        let file_path = file.path();
        let file_stem = file_path.file_stem().unwrap().to_str().unwrap();

        let output_path = format!("{}.typ", file_stem);

        let command_output = Command::new("pandoc")
            .arg("--from")
            .arg("markdown")
            .arg("--to")
            .arg("typst")
            .arg("--no-highlight")
            .arg(&file_path)
            .arg("-o")
            .arg(&output_path)
            .output();

        match command_output {
            Ok(output) if output.status.success() => {
                if let Err(err) = add_header_to_file(&output_path) {
                    eprintln!("Failed to add header to file {}: {}", output_path, err);
                }
            }
            Ok(output) => {
                eprintln!(
                    "Failed to run pandoc on file {}: {}",
                    file_path.display(),
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            Err(err) => {
                eprintln!(
                    "Failed to run pandoc on file {}: {}",
                    file_path.display(),
                    err
                );
            }
        }
    });

    Ok(())
}

use chrono::{Datelike, Local};
use std::path::Path;

fn add_header_to_file(file_path: &str) -> io::Result<()> {
    let input_file = File::open(file_path)?;
    let input_lines = BufReader::new(input_file).lines().peekable();

    let temp_file_path = format!("{}.temp", file_path);
    let mut output_file = File::create(&temp_file_path)?;

    let file_name = Path::new(file_path).file_stem().unwrap().to_str().unwrap();

    let date = Local::now();
    let date_str = format!(
        "{} {}, {}",
        date.day(),
        match date.month() {
            1 => "Janvier",
            2 => "Février",
            3 => "Mars",
            4 => "Avril",
            5 => "Mai",
            6 => "Juin",
            7 => "Juillet",
            8 => "Août",
            9 => "Septembre",
            10 => "Octobre",
            11 => "Novembre",
            12 => "Décembre",
            _ => "",
        },
        date.year(),
    );
    let title = file_name.replace("_", " ");
    writeln!(output_file, "#import \"template.typ\": *\n")?;
    writeln!(
        output_file,
        "// Take a look at the file `template.typ` in the file panel"
    )?;
    writeln!(
        output_file,
        "// to customize this template and discover how it works."
    )?;
    writeln!(
        output_file,
        "#show: project.with(\n  title: \"{}\",\n  authors: (\n    \"Lucas Duchet-Annez\",\n  ),\n  date: \"{}\",\n)\n",
        title, date_str
    )?;
    writeln!(output_file, "#set heading(numbering: \"1.1.\")\n")?;
    let pattern = Regex::new(r#"#link\("(.*?)\.md"\)"#).unwrap();
    let sys_re = Regex::new(r"systemequation \$([^\n|\$]*)\$\s*\n\s*\$([^\n|\$]*)\$").unwrap();
    let mut lines = Vec::new();

    for line in input_lines {
        lines.push(line?);
    }
    let text = lines.join("\n");
    let replaced = sys_re.replace_all(&text, |caps: &Captures| {
        let x = format!("$cases({}, {})$", caps[1].to_owned(), caps[2].to_owned());
        x
    });
    lines = replaced.lines().map(|l| l.to_string()).collect();

    for line in &lines {
        let new_line = pattern.replace_all(&line, "#link(\"$1.pdf\")");
        let new_line = new_line.replace("\\#pagebreak", "#pagebreak");
        let spaced_line = new_line.replace("%20", " ");
        let sys_re = Regex::new(r"systemequation \$([^\n]*)\$ \$([^\n]*)\$").unwrap();

        let replaced = sys_re.replace_all(&spaced_line, |caps: &Captures| {
            format!("$cases({}, {})$", &caps[1], &caps[2])
        });
        writeln!(output_file, "{}", replaced)?;
    }

    fs::rename(&temp_file_path, file_path)?;

    let typst_command_output = Command::new("typst").arg("compile").arg(file_path).output();

    match typst_command_output {
        Ok(output) if output.status.success() => {
            println!("File {} successfully compiled with typst.", file_path);
        }
        Ok(output) => {
            eprintln!(
                "Failed to compile file {} with typst: {}",
                file_path,
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Err(err) => {
            eprintln!("Failed to run typst on file {}: {}", file_path, err);
        }
    }

    Ok(())
}
