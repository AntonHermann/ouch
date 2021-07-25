use std::{
    fs,
    io::{self, BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
};

use utils::colors;

use crate::{
    archive,
    cli::Command,
    dialogs::Confirmation,
    error::FinalError,
    file::{
        extensions_from_path, separate_known_extensions_from_name,
        CompressionFormat::{self, *},
    },
    oof,
    utils::{self, to_utf},
};

pub fn run(command: Command, flags: &oof::Flags) -> crate::Result<()> {
    match command {
        Command::Compress { files, output_path } => {
            // Formats from path extension, like "file.tar.gz.xz" -> vec![Tar, Gzip, Lzma]
            let formats = extensions_from_path(&output_path);
            if formats.is_empty() {
                FinalError::with_title(format!("Cannot compress to '{}'.", to_utf(&output_path)))
                    .detail("You shall supply the compression format via the extension.")
                    .hint("Try adding something like .tar.gz or .zip to the output file.")
                    .hint("")
                    .hint("Examples:")
                    .hint(format!("  ouch compress ... {}.tar.gz", to_utf(&output_path)))
                    .hint(format!("  ouch compress ... {}.zip", to_utf(&output_path)))
                    .display_and_crash();
            }

            if matches!(&formats[0], Bzip | Gzip | Lzma) && files.len() > 1 {
                // This piece of code creates a sugestion for compressing multiple files
                // It says:
                // Change from file.bz.xz
                // To          file.tar.bz.xz
                let extensions_text: String =
                    formats.iter().map(|format| format.to_string()).collect();

                let output_path = to_utf(output_path);

                // Breaks if Lzma is .lz or .lzma and not .xz
                // Or if Bzip is .bz2 and not .bz
                let extensions_start_position = output_path.rfind(&extensions_text).unwrap();
                let pos = extensions_start_position;
                let empty_range = pos..pos;
                let mut suggested_output_path = output_path.clone();
                suggested_output_path.replace_range(empty_range, ".tar");

                FinalError::with_title(format!(
                    "Cannot compress to '{}'.",
                    to_utf(&output_path)
                ))
                .detail("You are trying to compress multiple files.")
                .detail(format!(
                    "The compression format '{}' cannot receive multiple files.",
                    &formats[0]
                ))
                .detail("The only supported formats that bundle files into an archive are .tar and .zip.")
                .hint(format!(
                    "Try inserting '.tar' or '.zip' before '{}'.",
                    &formats[0]
                ))
                .hint(format!("From: {}", output_path))
                .hint(format!(" To : {}", suggested_output_path))
                .display_and_crash();
            }

            if let Some(format) =
                formats.iter().skip(1).position(|format| matches!(format, Tar | Zip))
            {
                FinalError::with_title(format!("Cannot compress to '{}'.", to_utf(&output_path)))
                    .detail(format!("Found the format '{}' in an incorrect position.", format))
                    .detail(format!(
                        "{} can only be used at the start of the file extension.",
                        format
                    ))
                    .hint(format!(
                        "If you wish to compress multiple files, start the extension with {}.",
                        format
                    ))
                    .hint(format!("Otherwise, remove {} from '{}'.", format, to_utf(&output_path)))
                    .display_and_crash();
            }

            let confirm = Confirmation::new("Do you want to overwrite 'FILE'?", Some("FILE"));

            if output_path.exists()
                && !utils::permission_for_overwriting(&output_path, flags, &confirm)?
            {
                // The user does not want to overwrite the file
                return Ok(());
            }

            let output_file = fs::File::create(&output_path).unwrap_or_else(|err| {
                FinalError::with_title(format!("Cannot compress to '{}'.", to_utf(&output_path)))
                    .detail(format!("Could not open file '{}' for writing.", to_utf(&output_path)))
                    .detail(format!("Error: {}.", err))
                    .display_and_crash()
            });
            let compress_result = compress_files(files, formats, output_file, flags);

            // If any error occurred, delete incomplete file
            if compress_result.is_err() {
                // Print an extra alert message pointing out that we left a possibly
                // CORRUPTED FILE at `output_path`
                if let Err(err) = fs::remove_file(&output_path) {
                    eprintln!("{red}FATAL ERROR:\n", red = colors::red());
                    eprintln!("  Please manually delete '{}'.", to_utf(&output_path));
                    eprintln!(
                        "  Compression failed and we could not delete '{}'.",
                        to_utf(&output_path),
                    );
                    eprintln!(
                        "  Error:{reset} {}{red}.{reset}\n",
                        err,
                        reset = colors::reset(),
                        red = colors::red()
                    );
                }
            } else {
                println!(
                    "{}[INFO]{} Successfully compressed '{}'.",
                    colors::yellow(),
                    colors::reset(),
                    to_utf(output_path),
                );
            }

            compress_result?;
        },
        Command::Decompress { files, output_folder } => {
            let mut output_paths = vec![];
            let mut formats = vec![];

            for path in files.iter() {
                let (file_output_path, file_formats) = separate_known_extensions_from_name(path);
                output_paths.push(file_output_path);
                formats.push(file_formats);
            }

            let files_missing_format: Vec<PathBuf> = files
                .iter()
                .zip(&formats)
                .filter(|(_, formats)| formats.is_empty())
                .map(|(input_path, _)| PathBuf::from(input_path))
                .collect();

            if !files_missing_format.is_empty() {
                panic!("Throw this vec into a error variant: {:#?}", files_missing_format);
            }

            // From Option<PathBuf> to Option<&Path>
            let output_folder = output_folder.as_ref().map(|path| path.as_ref());

            for ((input_path, formats), output_path) in files.iter().zip(formats).zip(output_paths)
            {
                decompress_file(input_path, formats, output_folder, output_path, flags)?;
            }
        },
        Command::ShowHelp => crate::help_command(),
        Command::ShowVersion => crate::version_command(),
    }
    Ok(())
}

fn compress_files(
    files: Vec<PathBuf>,
    formats: Vec<CompressionFormat>,
    output_file: fs::File,
    _flags: &oof::Flags,
) -> crate::Result<()> {
    let file_writer = BufWriter::new(output_file);

    if formats.len() == 1 {
        let build_archive_from_paths = match formats[0] {
            Tar => archive::tar::build_archive_from_paths,
            Zip => archive::zip::build_archive_from_paths,
            _ => unreachable!(),
        };

        let mut bufwriter = build_archive_from_paths(&files, file_writer)?;
        bufwriter.flush()?;
    } else {
        let mut writer: Box<dyn Write> = Box::new(file_writer);

        // Grab previous encoder and wrap it inside of a new one
        let chain_writer_encoder = |format: &CompressionFormat, encoder: Box<dyn Write>| {
            let encoder: Box<dyn Write> = match format {
                Gzip => Box::new(flate2::write::GzEncoder::new(encoder, Default::default())),
                Bzip => Box::new(bzip2::write::BzEncoder::new(encoder, Default::default())),
                Lzma => Box::new(xz2::write::XzEncoder::new(encoder, 6)),
                _ => unreachable!(),
            };
            encoder
        };

        for format in formats.iter().skip(1).rev() {
            writer = chain_writer_encoder(format, writer);
        }

        match formats[0] {
            Gzip | Bzip | Lzma => {
                writer = chain_writer_encoder(&formats[0], writer);
                let mut reader = fs::File::open(&files[0]).unwrap();
                io::copy(&mut reader, &mut writer)?;
            },
            Tar => {
                let mut writer = archive::tar::build_archive_from_paths(&files, writer)?;
                writer.flush()?;
            },
            Zip => {
                eprintln!(
                    "{yellow}Warning:{reset}",
                    yellow = colors::yellow(),
                    reset = colors::reset()
                );
                eprintln!("\tCompressing .zip entirely in memory.");
                eprintln!("\tIf the file is too big, your pc might freeze!");
                eprintln!(
                    "\tThis is a limitation for formats like '{}'.",
                    formats.iter().map(|format| format.to_string()).collect::<String>()
                );
                eprintln!("\tThe design of .zip makes it impossible to compress via stream.");

                let mut vec_buffer = io::Cursor::new(vec![]);

                archive::zip::build_archive_from_paths(&files, &mut vec_buffer)?;
                io::copy(&mut vec_buffer, &mut writer)?;
            },
        }
    }

    Ok(())
}

fn decompress_file(
    input_file_path: &Path,
    formats: Vec<CompressionFormat>,
    output_folder: Option<&Path>,
    output_path: &Path,
    flags: &oof::Flags,
) -> crate::Result<()> {
    // TODO: improve error treatment
    let reader = fs::File::open(&input_file_path)?;
    let reader = BufReader::new(reader);
    let mut reader: Box<dyn Read> = Box::new(reader);

    // Grab previous decoder and wrap it inside of a new one
    let chain_reader_decoder = |format: &CompressionFormat, decoder: Box<dyn Read>| {
        let decoder: Box<dyn Read> = match format {
            Gzip => Box::new(flate2::read::GzDecoder::new(decoder)),
            Bzip => Box::new(bzip2::read::BzDecoder::new(decoder)),
            Lzma => Box::new(xz2::read::XzDecoder::new(decoder)),
            _ => unreachable!(),
        };
        decoder
    };

    for format in formats.iter().skip(1).rev() {
        reader = chain_reader_decoder(format, reader);
    }

    // Output path with folder prefix
    let output_path = if let Some(output_folder) = output_folder {
        output_folder.join(output_path)
    } else {
        output_path.to_path_buf()
    };

    let output_folder = output_folder.unwrap_or_else(|| Path::new("."));

    match formats[0] {
        Gzip | Bzip | Lzma => {
            reader = chain_reader_decoder(&formats[0], reader);

            // TODO: improve error treatment
            // TODO: provide more context for this error treatment
            let mut writer = fs::File::create(&output_path)?;

            io::copy(&mut reader, &mut writer)?;
            println!("[INFO]: Successfully uncompressed file at '{}'.", to_utf(output_path));
        },
        Tar => {
            utils::create_dir_if_non_existent(output_folder)?;
            let _ = crate::archive::tar::unpack_archive(reader, output_folder, flags)?;
            println!("[INFO]: Successfully uncompressed bundle at '{}'.", to_utf(output_folder));
        },
        Zip => {
            utils::create_dir_if_non_existent(output_folder)?;

            // If this is the only one
            if formats.len() == 1 {
                todo!("fix this!!!");
            }

            let mut vec = vec![];
            io::copy(&mut reader, &mut vec)?;
            let zip_archive = zip::ZipArchive::new(io::Cursor::new(vec))?;

            let _ = crate::archive::zip::unpack_archive(zip_archive, output_folder, flags)?;

            println!("[INFO]: Successfully uncompressed bundle at '{}'.", to_utf(output_folder));
            // let vec_buffer = vec![];
            // let mut vec_buffer = io::Cursor::new(vec_buffer);
            // // TODO: improve/change this message
            // eprintln!("Compressing first into .zip.");
            // eprintln!("Warning: .zip archives with extra extensions have a downside.");
            // eprintln!("The only way is loading everything into the RAM while compressing, and then write everything down.");
            // eprintln!("this means that by compressing .zip with extra compression formats, you can run out of RAM if the file is too large!");
            // zip::build_archive_from_paths(&files, &mut vec_buffer)?;

            // io::copy(&mut vec_buffer, &mut writer)?;
        },
    }

    Ok(())
}
