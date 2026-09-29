use std::{
    env,
    io::IsTerminal,
    path::{Path, PathBuf},
    sync::OnceLock,
    time::Instant,
};

const LABEL_WIDTH: usize = 8;

static STARTED_AT: OnceLock<Instant> = OnceLock::new();

#[derive(Clone, Copy)]
enum Tone {
    App,
    Cwd,
    Stage,
    Done,
    Item,
    Step,
    Write,
    Copy,
    Skip,
    Warn,
    Debug,
}

impl Tone {
    fn label(self) -> &'static str {
        match self {
            Tone::App => "app",
            Tone::Cwd => "cwd",
            Tone::Stage => "stage",
            Tone::Done => "done",
            Tone::Item => "item",
            Tone::Step => "step",
            Tone::Write => "write",
            Tone::Copy => "copy",
            Tone::Skip => "skip",
            Tone::Warn => "warn",
            Tone::Debug => "debug",
        }
    }

    fn color(self) -> &'static str {
        match self {
            Tone::App => "\x1b[36;1m",
            Tone::Cwd => "\x1b[2m",
            Tone::Stage => "\x1b[35;1m",
            Tone::Done => "\x1b[32;1m",
            Tone::Item => "\x1b[36m",
            Tone::Step => "\x1b[34m",
            Tone::Write => "\x1b[32m",
            Tone::Copy => "\x1b[36m",
            Tone::Skip => "\x1b[33m",
            Tone::Warn => "\x1b[31;1m",
            Tone::Debug => "\x1b[2m",
        }
    }
}

pub fn app_start(name: &str) {
    STARTED_AT.get_or_init(Instant::now);
    println!();
    emit(Tone::App, 0, format!("{name} started"));
}

pub fn app_done(name: &str) {
    emit(Tone::Done, 0, name);
    println!();
}

pub fn cwd(path: &Path) {
    emit(Tone::Cwd, 0, display_path(path));
}

pub fn stage(name: &str) {
    println!();
    emit(Tone::Stage, 0, name);
}

pub fn done(name: &str) {
    emit(Tone::Done, 0, name);
}

pub fn item(kind: &str, name: &str) {
    emit(Tone::Item, 1, format!("{kind}: {name}"));
}

pub fn step(message: impl AsRef<str>) {
    emit(Tone::Step, 2, message.as_ref());
}

pub fn write(path: &Path) {
    emit(Tone::Write, 2, display_path(path));
}

pub fn copy(source: &Path, target: &Path) {
    emit(
        Tone::Copy,
        2,
        format!("{} -> {}", display_path(source), display_path(target)),
    );
}

pub fn skip(path: &Path, reason: impl AsRef<str>) {
    emit(
        Tone::Skip,
        2,
        format!("{} ({})", display_path(path), reason.as_ref()),
    );
}

pub fn warn(message: impl AsRef<str>) {
    emit(Tone::Warn, 2, message.as_ref());
}

pub fn verbose(message: impl AsRef<str>) {
    if is_verbose() {
        emit(Tone::Debug, 2, message.as_ref());
    }
}

pub fn verbose_path(action: &str, path: &Path) {
    if is_verbose() {
        emit(Tone::Debug, 2, format!("{action}: {}", display_path(path)));
    }
}

pub fn is_verbose() -> bool {
    env::var("APP_GEN_VERBOSE")
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "on"))
        .unwrap_or(false)
}

fn emit(tone: Tone, indent: usize, message: impl AsRef<str>) {
    let elapsed = elapsed();
    let label = format!("[{}]", tone.label());
    let indent = "  ".repeat(indent);

    if use_color() {
        println!(
            "{}{}{} {}{}{:LABEL_WIDTH$}{} {}{}{}",
            "\x1b[2m",
            elapsed,
            "\x1b[0m",
            indent,
            tone.color(),
            label,
            "\x1b[0m",
            body_style(tone),
            message.as_ref(),
            "\x1b[0m"
        );
    } else {
        println!(
            "{elapsed} {indent}{label:LABEL_WIDTH$} {}",
            message.as_ref()
        );
    }
}

fn elapsed() -> String {
    let started_at = STARTED_AT.get_or_init(Instant::now);
    let elapsed = started_at.elapsed();
    let millis = elapsed.as_millis();

    if millis < 1_000 {
        format!("{millis:>5}ms")
    } else {
        format!("{:>5.1}s", elapsed.as_secs_f32())
    }
}

fn body_style(tone: Tone) -> &'static str {
    match tone {
        Tone::Cwd | Tone::Debug => "\x1b[2m",
        _ => "",
    }
}

fn use_color() -> bool {
    if env::var_os("NO_COLOR").is_some() {
        return false;
    }

    match env::var("APP_GEN_COLOR").ok().as_deref() {
        Some("always") | Some("1") | Some("true") | Some("yes") | Some("on") => true,
        Some("never") | Some("0") | Some("false") | Some("no") | Some("off") => false,
        _ => std::io::stdout().is_terminal(),
    }
}

fn display_path(path: &Path) -> String {
    let path = normalize_path(path);
    if let Some(root) = workspace_root() {
        if let Ok(stripped) = path.strip_prefix(root) {
            return stripped.display().to_string();
        }
    }

    path.display().to_string()
}

fn normalize_path(path: &Path) -> PathBuf {
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    };

    std::fs::canonicalize(&path).unwrap_or(path)
}

fn workspace_root() -> Option<PathBuf> {
    let cwd = env::current_dir().ok()?;
    if cwd.file_name().and_then(|name| name.to_str()) == Some("app_gen") {
        cwd.parent().map(Path::to_path_buf)
    } else {
        Some(cwd)
    }
}
