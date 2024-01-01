use std::{fs, path::Path};

use rand::RngCore;
use rand_chacha::{rand_core::SeedableRng, ChaCha8Rng};
use weechess_core::{Book, BookParseError, BookParser};

const BOOK_DEPTH: usize = 10;
const BOOK_SEED_ENV_VAR: &'static str = "WEECHESS_BOOK_SEED";
const BOOK_DATA_FILE_NAME: &'static str = "book_data.bin";

#[derive(Debug)]
enum BuildError {
    BookParsing(BookParseError),
    Io(std::io::Error),
    Serialization(ciborium::ser::Error<std::io::Error>),
}

fn generate_book_data() -> Result<(), BuildError> {
    let book_dir: std::path::PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("book")
        .canonicalize()
        .unwrap();

    let mut book: Book = Book::new();

    let hasher = weechess_core::ZobristHasher::with(&mut {
        // NOTE: this rng implementation and seed needs to match what we use in
        // the engine when initializing the book lookup table.
        let seed = rand::thread_rng().next_u64();
        println!("cargo:rustc-env={}={}", BOOK_SEED_ENV_VAR, seed);
        ChaCha8Rng::seed_from_u64(seed)
    });

    for entry in fs::read_dir(book_dir).unwrap() {
        let entry = entry.map_err(|e| BuildError::Io(e))?;
        if entry.file_type().unwrap().is_file() {
            println!("cargo:rerun-if-changed={}", entry.path().display());
            let book_contents = fs::read_to_string(entry.path()).map_err(|e| BuildError::Io(e))?;
            book = book_contents
                .trim()
                .split("\n\n")
                .filter(|c| c.starts_with("1."))
                .try_fold(book, |mut book, movetext| {
                    let moves = BookParser::parse_movetext(movetext, &hasher)
                        .take(BOOK_DEPTH)
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(BuildError::BookParsing)?;

                    for (hash, mov) in moves.into_iter() {
                        book.append(hash, &[mov]);
                    }

                    Ok(book)
                })?;
        }
    }

    assert!(book.len() > 0);

    // Serialize the book data
    let mut buf = Vec::new();
    ciborium::into_writer(&book, &mut buf).map_err(|e| BuildError::Serialization(e))?;

    // Write the book data to a file
    let book_data_path = Path::new(&std::env::var("OUT_DIR").unwrap()).join(BOOK_DATA_FILE_NAME);
    std::fs::write(book_data_path, buf).map_err(|e| BuildError::Io(e))?;

    Ok(())
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    generate_book_data().unwrap();
}
