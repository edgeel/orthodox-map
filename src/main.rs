use anstream::{eprintln, println};
use anstyle::{AnsiColor, Style};
use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use orthodox_map::model::{self, Diag};
use orthodox_map::render::{self, SiteConfig};
use orthodox_map::{serve, validate};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const ERR: Style = AnsiColor::Red.on_default().bold();
const WARN: Style = AnsiColor::Yellow.on_default().bold();
const OK: Style = AnsiColor::Green.on_default().bold();
const DIM: Style = Style::new().dimmed();

/// Генератор сайта «Карта изучения православия».
#[derive(Parser)]
#[command(name = "omap", version = orthodox_map::VERSION, after_help = "\
Примеры:
  omap check                  проверить content/
  omap build                  собрать сайт в dist/
  omap serve                  собрать и открыть на http://127.0.0.1:8000/
  omap build --single dist/orthodox-map.html   весь сайт одним HTML-файлом
  omap build --content ../orthodox-map-content --out /tmp/site")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(clap::Args, Clone)]
struct Paths {
    /// Папка с содержанием (sections.toml, routes.toml, topics/)
    #[arg(long, default_value = "content", value_hint = clap::ValueHint::DirPath)]
    content: PathBuf,
    /// Настройки сайта (название, адрес)
    #[arg(long, default_value = "site.toml", value_hint = clap::ValueHint::FilePath)]
    site: PathBuf,
}

#[derive(Subcommand)]
enum Cmd {
    /// Проверить содержание: id, ссылки needs, циклы, материалы
    Check {
        #[command(flatten)]
        paths: Paths,
    },
    /// Собрать сайт
    Build {
        #[command(flatten)]
        paths: Paths,
        /// Куда собрать
        #[arg(long, default_value = "dist", value_hint = clap::ValueHint::DirPath)]
        out: PathBuf,
        /// Вместо папки — один HTML-файл со всем внутри (открывается без сети, с диска)
        #[arg(long, value_name = "ФАЙЛ", conflicts_with = "out", value_hint = clap::ValueHint::FilePath)]
        single: Option<PathBuf>,
    },
    /// Собрать и показать локально
    Serve {
        #[command(flatten)]
        paths: Paths,
        #[arg(long, default_value = "dist", value_hint = clap::ValueHint::DirPath)]
        out: PathBuf,
        /// Адрес и порт
        #[arg(long, default_value = "127.0.0.1:8000")]
        addr: String,
    },
}

fn print_diags(diags: &[Diag]) {
    for d in diags {
        let (style, kind) = if d.warning {
            (WARN, "предупреждение")
        } else {
            (ERR, "ошибка")
        };
        eprintln!(
            "{style}{kind}{style:#}: {DIM}{}{DIM:#}: {}",
            d.file.display(),
            d.message
        );
    }
}

fn load(paths: &Paths) -> Result<Option<model::Content>> {
    let (content, diags) = model::load(&paths.content);
    if !diags.is_empty() {
        print_diags(&diags);
        if validate::has_errors(&diags) {
            return Ok(None);
        }
    }
    Ok(Some(content))
}

fn build(paths: &Paths, out: &Path) -> Result<bool> {
    let Some(content) = load(paths)? else {
        return Ok(false);
    };
    let site = SiteConfig::load(&paths.site)?;
    match render::build(&content, &site, out)? {
        Ok(built) => {
            print_diags(&built.warnings);
            println!(
                "{OK}собрано{OK:#}: {} тем, {} путей, {} файлов → {}",
                built.topics,
                built.routes,
                built.files.len(),
                out.display()
            );
            Ok(true)
        }
        Err(diags) => {
            print_diags(&diags);
            let n = diags.iter().filter(|d| !d.warning).count();
            eprintln!("{ERR}не собрано{ERR:#}: ошибок {n}");
            Ok(false)
        }
    }
}

fn build_single(paths: &Paths, file: &Path) -> Result<bool> {
    let Some(content) = load(paths)? else {
        return Ok(false);
    };
    let site = SiteConfig::load(&paths.site)?;
    match render::single(&content, &site)? {
        Ok(single) => {
            print_diags(&single.warnings);
            render::write_single(file, &single.html)?;
            println!(
                "{OK}собрано{OK:#}: {} тем, {} путей → {} ({} КБ)",
                single.topics,
                single.routes,
                file.display(),
                single.html.len().div_ceil(1024)
            );
            Ok(true)
        }
        Err(diags) => {
            print_diags(&diags);
            let n = diags.iter().filter(|d| !d.warning).count();
            eprintln!("{ERR}не собрано{ERR:#}: ошибок {n}");
            Ok(false)
        }
    }
}

fn run() -> Result<bool> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Check { paths } => {
            let Some(content) = load(&paths)? else {
                return Ok(false);
            };
            let diags = validate::validate(&content);
            print_diags(&diags);
            let errors = diags.iter().filter(|d| !d.warning).count();
            let warnings = diags.len() - errors;
            if errors == 0 {
                println!(
                    "{OK}ок{OK:#}: {} разделов, {} тем, {} путей; предупреждений {warnings}",
                    content.sections.len(),
                    content.topics.len(),
                    content.routes.len()
                );
                Ok(true)
            } else {
                eprintln!("{ERR}ошибок {errors}{ERR:#}, предупреждений {warnings}");
                Ok(false)
            }
        }
        Cmd::Build {
            paths,
            single: Some(file),
            ..
        } => build_single(&paths, &file),
        Cmd::Build { paths, out, .. } => build(&paths, &out),
        Cmd::Serve { paths, out, addr } => {
            if !build(&paths, &out)? {
                return Ok(false);
            }
            let site = SiteConfig::load(&paths.site)?;
            println!("{OK}смотреть{OK:#}: http://{addr}/  {DIM}(Ctrl+C — остановить){DIM:#}");
            serve::serve(&out, &site.base_path(), &addr)?;
            Ok(true)
        }
    }
}

fn main() -> ExitCode {
    clap_complete::CompleteEnv::with_factory(Cli::command).complete();
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("{ERR}ошибка{ERR:#}: {e:#}");
            ExitCode::FAILURE
        }
    }
}
