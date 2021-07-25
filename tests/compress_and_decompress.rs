use std::{
    env, fs,
    io::prelude::*,
    path::{Path, PathBuf},
};

use ouch::{cli::Command, commands::run, oof};
use rand::{rngs::SmallRng, RngCore, SeedableRng};
use tempdir::TempDir;

// #[test]
// /// Tests each format that supports multiple files with random input.
// /// TODO: test the remaining formats.
// /// TODO2: Fix testing of .tar.zip and .zip.zip
// fn test_each_format() {
//     let mut rng = SmallRng::from_entropy();
//     test_compression_and_decompression(&mut rng, "tar");
//     test_compression_and_decompression(&mut rng, "tar.gz");
//     test_compression_and_decompression(&mut rng, "tar.bz");
//     test_compression_and_decompression(&mut rng, "tar.bz2");
//     test_compression_and_decompression(&mut rng, "tar.xz");
//     test_compression_and_decompression(&mut rng, "tar.lz");
//     test_compression_and_decompression(&mut rng, "tar.lzma");
//     // test_compression_and_decompression(&mut rng, "tar.zip");
//     test_compression_and_decompression(&mut rng, "zip");
//     test_compression_and_decompression(&mut rng, "zip.gz");
//     test_compression_and_decompression(&mut rng, "zip.bz");
//     test_compression_and_decompression(&mut rng, "zip.bz2");
//     test_compression_and_decompression(&mut rng, "zip.xz");
//     test_compression_and_decompression(&mut rng, "zip.lz");
//     test_compression_and_decompression(&mut rng, "zip.lzma");
//     // test_compression_and_decompression(&mut rng, "zip.zip");
// }

#[test]
fn test_compression_and_decompression_tar() {
    test_compression_and_decompression(&mut SmallRng::from_entropy(), "tar");
}

#[test]
fn test_compression_and_decompression_tar_gz() {
    test_compression_and_decompression(&mut SmallRng::from_entropy(), "tar.gz");
}

#[test]
fn test_compression_and_decompression_tar_bz() {
    test_compression_and_decompression(&mut SmallRng::from_entropy(), "tar.bz");
}

#[test]
fn test_compression_and_decompression_tar_bz2() {
    test_compression_and_decompression(&mut SmallRng::from_entropy(), "tar.bz2");
}

#[test]
fn test_compression_and_decompression_tar_xz() {
    test_compression_and_decompression(&mut SmallRng::from_entropy(), "tar.xz");
}

#[test]
fn test_compression_and_decompression_tar_lz() {
    test_compression_and_decompression(&mut SmallRng::from_entropy(), "tar.lz");
}

#[test]
fn test_compression_and_decompression_tar_lzma() {
    test_compression_and_decompression(&mut SmallRng::from_entropy(), "tar.lzma");
}

#[test]
fn test_compression_and_decompression_zip() {
    test_compression_and_decompression(&mut SmallRng::from_entropy(), "zip");
}

#[test]
fn test_compression_and_decompression_zip_gz() {
    test_compression_and_decompression(&mut SmallRng::from_entropy(), "zip.gz");
}

#[test]
fn test_compression_and_decompression_zip_bz() {
    test_compression_and_decompression(&mut SmallRng::from_entropy(), "zip.bz");
}

#[test]
fn test_compression_and_decompression_zip_bz2() {
    test_compression_and_decompression(&mut SmallRng::from_entropy(), "zip.bz2");
}

#[test]
fn test_compression_and_decompression_zip_xz() {
    test_compression_and_decompression(&mut SmallRng::from_entropy(), "zip.xz");
}

#[test]
fn test_compression_and_decompression_zip_lz() {
    test_compression_and_decompression(&mut SmallRng::from_entropy(), "zip.lz");
}

#[test]
fn test_compression_and_decompression_zip_lzma() {
    test_compression_and_decompression(&mut SmallRng::from_entropy(), "zip.lzma");
}

type FileContent = Vec<u8>;

fn test_compression_and_decompression(rng: &mut impl RngCore, format: &str) {
    // System temporary directory depends on the platform
    // For linux it is /tmp
    let system_tmp = env::temp_dir();
    // Create a folder that will be deleted on drop
    let testing_dir = String::from("ouch-testing-") + format;
    let testing_dir = TempDir::new_in(system_tmp, &testing_dir).expect("Could not create tempdir");
    let testing_dir = testing_dir.path();

    // Quantity of compressed files vary from 1 to 10
    let quantity_of_files = rng.next_u32() % 10 + 1;

    let contents_of_files: Vec<FileContent> =
        (0..quantity_of_files).map(|_| generate_random_file_content(rng)).collect();

    let mut file_paths = create_files(&testing_dir, &contents_of_files);
    println!("am i here?");
    let archive_path = compress_files(&testing_dir, &file_paths, &format);
    let mut extracted_paths = extract_files(&archive_path);

    // // If you want to visualize the compressed and extracted files before auto-destruction:
    // std::thread::sleep(std::time::Duration::from_secs(40));

    file_paths.sort();
    extracted_paths.sort();

    compare_paths(&file_paths, &extracted_paths);
    compare_file_contents(&extracted_paths, &contents_of_files);
}

// Crate file contents from 1024 up to 8192 random bytes
fn generate_random_file_content(rng: &mut impl RngCore) -> FileContent {
    let quantity = 1024 + rng.next_u32() % (8192 - 1024);
    let mut vec = vec![0; quantity as usize];
    rng.fill_bytes(&mut vec);
    vec
}

// Create files using the indexes as file names (eg. 0, 1, 2 and 3)
// Returns the paths
fn create_files(at: &Path, contents: &[FileContent]) -> Vec<PathBuf> {
    contents
        .iter()
        .enumerate()
        .map(|(i, content)| {
            let path = at.join(i.to_string());
            let mut file = fs::File::create(&path).expect("Could not create dummy test file");
            file.write_all(content).expect("Could not write to dummy test file");
            path
        })
        .collect()
}

fn compress_files(at: &Path, paths_to_compress: &[PathBuf], format: &str) -> PathBuf {
    let archive_path = String::from("archive.") + format;
    let archive_path = at.join(archive_path);

    let command = Command::Compress {
        files: paths_to_compress.to_vec(),
        output_path: archive_path.to_path_buf(),
    };
    run(command, &oof::Flags::default()).expect("Failed to compress test dummy files");

    archive_path
}

fn extract_files(archive_path: &Path) -> Vec<PathBuf> {
    // We will extract in the same folder as the archive
    // If the archive is at:
    //   /tmp/ouch-testing-tar.Rbq4DusBrtF8/archive.tar
    // Then the extraction_output_folder will be:
    //   /tmp/ouch-testing-tar.Rbq4DusBrtF8/extraction_results/
    let mut extraction_output_folder = archive_path.to_path_buf();
    // Remove the name of the extracted archive
    assert!(extraction_output_folder.pop());
    // Add the suffix "results"
    extraction_output_folder.push("extraction_results");

    let command = Command::Decompress {
        files: vec![archive_path.to_owned()],
        output_folder: Some(extraction_output_folder.clone()),
    };
    run(command, &oof::Flags::default()).expect("Failed to extract");

    fs::read_dir(extraction_output_folder)
        .unwrap()
        .map(Result::unwrap)
        .map(|entry| entry.path())
        .collect()
}

fn compare_paths(original: &[PathBuf], extracted: &[PathBuf]) {
    assert_eq!(original.len(), extracted.len());
    for (original, extracted) in original.iter().zip(extracted) {
        assert_eq!(original.file_name(), extracted.file_name());
    }
}

fn compare_file_contents(extracted: &[PathBuf], contents: &[FileContent]) {
    for (extracted_path, expected_content) in extracted.iter().zip(contents) {
        let read_content = fs::read(extracted_path).expect("Failed to read from file");
        assert_eq!(&read_content, expected_content);
    }
}
