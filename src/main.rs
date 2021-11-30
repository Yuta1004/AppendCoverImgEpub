use std::io;
use std::io::prelude::*;
use std::io::{Seek, Write};
use std::fs;
use std::fs::File;
use std::path::Path;

use walkdir::WalkDir;
use zip::write::FileOptions;
use structopt::StructOpt;

fn add_file_into_zip<W: Write+Seek>(zip: &mut zip::ZipWriter<W>, src_filename: &str, dst_filename: &str, options: &FileOptions) {
    print!("Archiving {:?}: ", dst_filename);

    zip.start_file(dst_filename, *options).unwrap();
    let mut f = File::open(src_filename).unwrap();
    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer).unwrap();
    zip.write_all(&*buffer).unwrap();

    println!("OK");
}

fn archive_epub(src_dirname: &str, dst_filename: &str) -> zip::result::ZipResult<()> {
    let dst_path = std::path::Path::new(dst_filename);
    let file = match std::fs::File::create(&dst_path) {
        Ok(f) => f,
        Err(e) => panic!("File cannot create! {}", e)
    };

    let mut zip = zip::ZipWriter::new(file);
    let options = FileOptions::default().unix_permissions(0o755);

    // mimetypeは必ずアーカイブの先頭に存在する必要がある
    add_file_into_zip(&mut zip, &(src_dirname.to_string().clone()+"/mimetype"), "mimetype", &options);

    // META-INF: ディレクトリ構造説明ディレクトリ
    // OEBPS: コンテンツディレクトリ
    for dir_in_src_dir in vec!["META-INF", "OEBPS"].iter().map(|&s| src_dirname.to_string().clone()+"/"+s).collect::<Vec<String>>() {
        for fentry in WalkDir::new(dir_in_src_dir) {
            let entry = fentry.unwrap();
            let path = entry.path();
            let name = path.strip_prefix(Path::new(&src_dirname)).unwrap();

            if path.is_file() {
                add_file_into_zip(&mut zip, &path.display().to_string(), &name.display().to_string(), &options);
            } else if name.as_os_str().len() != 0 {
                zip.add_directory(name.display().to_string(), options).unwrap();
            }
        }
    }

    zip.finish().unwrap();

    Ok(())
}

fn extract_epub(epub_filename: &str, dst_dirname: &str) {
    let file = File::open(Path::new(epub_filename)).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();

    for idx in 0..archive.len() {
        let path;
        let mut archived_file = archive.by_index(idx).unwrap();
        let outpath = match archived_file.enclosed_name() {
            Some(_path) => {
                path = String::from(dst_dirname.to_string().clone()+"/"+&_path.display().to_string());
                Path::new(&path)
            },
            None => continue
        };

        print!("Extracting {}: ", outpath.display());

        if (&*archived_file.name()).ends_with("/") {
            fs::create_dir_all(&outpath).unwrap();
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(&p).unwrap();
                }
            }
            let mut outfile = File::create(&outpath).unwrap();
            io::copy(&mut archived_file, &mut outfile).unwrap();
        }

        println!("OK");
    }
}

/// Epubに表紙画像を追加する
#[derive(StructOpt)]
struct AppendCoverImgEpub {
    /// Epubファイル
    epub: String,
    /// 表紙画像
    img: String,
    /// 画像の種類 (jpeg, png, ...)
    media: String,
    /// 生成時に使用した一時ディレクトリを保持する
    #[structopt(short, long)]
    keep_tmp: bool
}

fn main() {
    let args = AppendCoverImgEpub::from_args();

    extract_epub(&args.epub, "__extract_epub_tmp");

    let opffile = File::open("__extract_epub_tmp/OEBPS/book.opf").unwrap();
    let mut opflines = io::BufReader::new(opffile).lines().filter(|e| match e {
        Ok(_) => true,
        Err(_) => false
    }).map(|e| match e {
        Ok(l) => l,
        Err(_) => String::from(""),
    }).collect::<Vec<String>>();

    let manifest_idx = opflines.iter().position(|l| l.contains("<manifest>")).unwrap();
    opflines.insert(manifest_idx+1,
        format!("<item properties=\"cover-image\" id=\"my-cover-image\" href=\"cover.{}\" media-type=\"image/{}\"/>", args.media, args.media)
    );
    fs::copy(args.img, format!("__extract_epub_tmp/OEBPS/cover.{}", args.media)).unwrap();

    let mut opffile = File::create("__extract_epub_tmp/OEBPS/book.opf").unwrap();
    for line in opflines {
        write!(opffile, "{}\n", line).unwrap();
    }
    opffile.flush().unwrap();

    archive_epub("__extract_epub_tmp", &args.epub).unwrap();

    if !args.keep_tmp {
        fs::remove_dir_all(Path::new("__extract_epub_tmp")).unwrap();
    }
}
