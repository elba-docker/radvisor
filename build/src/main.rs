use radvisor::shell::Shell;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use clap::{App, Clap, IntoApp};
use clap_generate::{generate, generators::*};
use flate2::write::GzEncoder;
use flate2::Compression;

type ShellOptions = radvisor::shell::Options;
type ParentOpts = radvisor::cli::Opts;

/// CLI version loaded from Cargo, or none if not build with cargo
pub const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

#[derive(Clap)]
#[clap(
    version = VERSION.unwrap_or("unknown"),
    author = "Joseph Azevedo",
    about = "Build tools for rAdvisor"
)]
pub struct Opts {
    /// Target directory to place generated completions/manpages in
    #[clap(
        parse(from_os_str),
        short = 'o',
        long = "out-dir",
        default_value = "./out"
    )]
    pub directory: PathBuf,

    /// Source directory which is the project root of the cloned git repo
    /// (needed for \ building docs)
    #[clap(parse(from_os_str), short = 'r', long = "repo-root")]
    pub repo_root: Option<PathBuf>,

    // Shell output-related options
    #[clap(flatten)]
    pub shell_options: ShellOptions,
}

fn main() {
    let opts = Opts::parse();
    let shell = Shell::new(&opts.shell_options);

    if let Err(err) = fs::create_dir_all(&opts.directory) {
        shell.error(format!(
            "An error occurred while creating the output directory at {:?}: {}",
            &opts.directory, err
        ))
    }

    generate_all_completions(&opts, &shell);
    generate_docs(&opts, &shell);
}

/// Generates and writes completion files for zsh, bash, fish, elvish, and
/// PowerShell
fn generate_all_completions(opts: &Opts, shell: &Shell) {
    shell.status("Generating", "shell completion files");

    let directory = opts.directory.join("completion");
    if let Err(err) = fs::create_dir_all(&directory) {
        shell.error(format!(
            "An error occurred while creating the completion file directory at {:?}: {}",
            &opts.directory, err
        ))
    }

    let app_name = "radvisor";
    try_generate::<Bash, ParentOpts>(&directory, "bash", app_name, shell);
    try_generate::<Fish, ParentOpts>(&directory, "fish", app_name, shell);
    try_generate::<PowerShell, ParentOpts>(&directory, "powershell", app_name, shell);
    try_generate::<Elvish, ParentOpts>(&directory, "elvish", app_name, shell);
    try_generate::<Zsh, ParentOpts>(&directory, "zsh", app_name, shell);
}

/// Tries to generate the given completion file, potentially failing to do so
/// and writing result status to the console.
fn try_generate<G: Generator, A: IntoApp>(
    directory: &Path,
    generator_type: &str,
    app_name: &str,
    shell: &Shell,
) {
    let path: PathBuf = directory.join(generator_type);
    match generate_completion::<G>(A::into_app(), &path, app_name) {
        Ok(()) => shell.status(
            "Generated",
            format!(
                "{} completion file successfully at {:?}",
                generator_type, path
            ),
        ),
        Err(err) => shell.error(format!(
            "An error occurred while generating the {} completion file at {:?}: {}",
            generator_type, path, err
        )),
    }
}

/// Generates a single completion file for the given generator, consuming
/// the app instance (due to unknown mutations). If file opening/writing fails,
/// returns with an io:Error
fn generate_completion<G: Generator>(
    app: App,
    path: &Path,
    app_name: &str,
) -> Result<(), io::Error> {
    let mut app = app;
    let mut buf = Vec::new();
    generate::<G, _>(&mut app, app_name, &mut buf);

    let mut file = File::create(path)?;
    file.write_all(&buf)?;

    Ok(())
}

/// Generates docs archives to eventually reside in /usr/share/docs/radvisor
fn generate_docs(opts: &Opts, shell: &Shell) {
    match &opts.repo_root {
        None => shell.info(
            "Skipping docs archive creation. To generate docs archive, clone the source repo and \
             include `--repo-root` in the run command",
        ),
        Some(root) => {
            shell.status("Generating", "docs archives for changelog/docs/readme");

            let directory = opts.directory.join("docs");
            if let Err(err) = fs::create_dir_all(&directory) {
                shell.error(format!(
                    "An error occurred while creating the docs archive directory at {:?}: {}",
                    &opts.directory, err
                ))
            }

            try_generate_docs_archive(&directory, &root.join("README.md"), "readme", shell);
            try_generate_docs_archive(&directory, &root.join("docs"), "docs", shell);
            try_generate_docs_archive(&directory, &root.join("CHANGELOG.md"), "changelog", shell);
        },
    }
}

/// Attempts to generate a docs archive, outputting errors in the console upon
/// failure. Directory is the root directory of all toolbox output, source is
/// the path of a file/folder to put at the root level of the archive, and
/// archive is the name of the resultant archive (plus .tar.gz)
fn try_generate_docs_archive(directory: &Path, source: &Path, name: &str, shell: &Shell) {
    let path = directory.join(String::from(name) + ".tar.gz");
    match generate_docs_archive(&directory, name, source, shell) {
        Ok((before, after)) => shell.status(
            "Compressed",
            format!(
                "docs archive from {:?} to {:?} ({:.2}% deflation)",
                source,
                path,
                deflation(before, after)
            ),
        ),
        Err(err) => shell.error(format!(
            "An error ocurred while generating the docs archive for {:?} at {:?}: {}",
            source, path, err
        )),
    }
}

/// Calculates the deflation/space savings percentage for the compression
/// results
fn deflation(before: usize, after: usize) -> f64 {
    if before != 0 {
        (1f64 - (after as f64) / (before as f64)) * 100f64
    } else {
        0f64
    }
}

/// Generates the docs archive from the source file/dir, returning the original
/// and deflated sizes (if successful)
fn generate_docs_archive(
    dest_path: &Path,
    archive_name: &str,
    source_path: &Path,
    shell: &Shell,
) -> Result<(usize, usize), io::Error> {
    // Determine the size of the source file/directory before compression
    let before: usize = match fs_extra::dir::get_size(source_path) {
        Ok(size) => size as usize,
        Err(err) => {
            shell.warn(format!(
                "Could not determine size of original path {:?}: {}",
                source_path, err
            ));
            0
        },
    };

    let archive_path = dest_path.join(String::from(archive_name) + ".tar.gz");
    {
        let tar_gz = File::create(&archive_path)?;
        let enc = GzEncoder::new(tar_gz, Compression::default());
        let mut tar = tar::Builder::new(enc);

        match PathType::from(source_path) {
            PathType::Directory => tar.append_dir_all(archive_name, source_path)?,
            PathType::File => {
                let file_name: &str = source_path
                    .file_name()
                    .and_then(|o| o.to_str())
                    .unwrap_or("docs");
                let internal_path: PathBuf = AsRef::<Path>::as_ref(archive_name).join(file_name);
                let mut file = File::open(source_path)?;
                tar.append_file(internal_path, &mut file)?
            },
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("path {:?} does not exist", source_path),
                ))
            },
        };
    }

    let after: usize = match fs::metadata(&archive_path) {
        Ok(md) => md.len() as usize,
        Err(err) => {
            shell.warn(format!(
                "Could not determine size of resultant archive {:?}: {}",
                archive_path, err
            ));
            0
        },
    };

    Ok((before, after))
}

/// Path type enum, used for resolving the type of a path
enum PathType {
    File,
    Directory,
    Invalid,
}

impl PathType {
    /// Gets the path type corresponding to the given path, returning
    /// PathType::Invalid if the path isn't valid
    fn from(path: &Path) -> Self {
        fs::metadata(path)
            .map(|m| match m.is_file() {
                true => PathType::File,
                false => PathType::Directory,
            })
            .unwrap_or(PathType::Invalid)
    }
}
