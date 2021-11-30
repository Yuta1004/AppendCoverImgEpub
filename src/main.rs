use std::io::prelude::*;
use std::io::{Seek, Write};
use std::fs::File;
use std::path::Path;

use walkdir::WalkDir;
use zip::write::FileOptions;

fn add_file_into_zip<W: Write+Seek>(zip: &mut zip::ZipWriter<W>, src_filename: &str, dst_filename: &str, options: &FileOptions) -> zip::result::ZipResult<()> {
    print!("Adding {:?}: ", dst_filename);

    zip.start_file(dst_filename, *options)?;
    let mut f = File::open(src_filename)?;
    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer)?;
    zip.write_all(&*buffer)?;

    println!("OK");

    Ok(())
}

fn zip_2_epub(src_dirname: &str, dst_filename: &str) -> zip::result::ZipResult<()> {
    let dst_path = std::path::Path::new(dst_filename);
    let file = match std::fs::File::create(&dst_path) {
        Ok(f) => f,
        Err(e) => panic!("File cannot create! {}", e)
    };

    let mut zip = zip::ZipWriter::new(file);
    let options = FileOptions::default().unix_permissions(0o755);

    // mimetypeは必ずアーカイブの先頭に存在する必要がある
    add_file_into_zip(&mut zip, &(src_dirname.to_string().clone()+"/mimetype"), "mimetype", &options)?;

    // META-INF: ディレクトリ構造説明ディレクトリ
    // OEBPS: コンテンツディレクトリ
    for dir_in_src_dir in vec!["META-INF", "OEBPS"].iter().map(|&s| src_dirname.to_string().clone()+"/"+s).collect::<Vec<String>>() {
        for fentry in WalkDir::new(dir_in_src_dir) {
            let entry = fentry.unwrap();
            let path = entry.path();
            let name = path.strip_prefix(Path::new(&src_dirname)).unwrap();

            if path.is_file() {
                add_file_into_zip(&mut zip, &path.display().to_string(), &name.display().to_string(), &options)?;
            } else if name.as_os_str().len() != 0 {
                zip.add_directory(name.display().to_string(), options)?;
            }
        }
    }

    zip.finish()?;

    Ok(())
}

fn main() {
    println!("Hello \"append_coverimg_epub\"");
}
