mod decoder;
mod export;
mod resample;
mod rpc;
mod storage;
use clap::{ArgAction, Parser, Subcommand};
use orient_core::{engine, extract, palette};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};
pub static CANCELLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub fn engine_id() -> &'static str {
    static ID: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    ID.get_or_init(|| {
        format!(
            "{}-{}",
            orient_core::ENGINE_ID,
            &env!("ORIENT_DECODER_HASH")[..12]
        )
    })
}
pub fn build_info() -> Value {
    json!({"backend":"rust", "interfaceVersion":1,"engine":engine_id(),"coreEngine":orient_core::ENGINE_ID,
           "decoder":env!("ORIENT_DECODER_HASH"),"version":env!("CARGO_PKG_VERSION").replace("-dev.",".dev")})
}
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
fn error(s: &str) -> Box<dyn std::error::Error + Send + Sync> {
    orient_core::Error::Value(s.to_owned()).into()
}
fn fail<T>(s: &str) -> Result<T> {
    Err(error(s))
}
#[derive(Parser)]
#[command(name="orient",version=engine_id(),args_override_self=true,disable_version_flag=true,about="Standalone, read-only Orient CLI. Publication belongs to the caller.")]
struct Args {
    #[arg(long="version", action=ArgAction::Version)]
    _version: Option<bool>,
    #[command(subcommand)]
    command: Commands,
}
#[derive(Subcommand)]
enum Commands {
    Analyze {
        image: PathBuf,
    },
    Sample {
        image: PathBuf,
        #[arg(allow_negative_numbers = true)]
        x: f64,
        #[arg(allow_negative_numbers = true)]
        y: f64,
    },
    Palette {
        image: PathBuf,
        #[arg(long,default_value="dark",value_parser=["dark","light"])]
        mode: String,
        #[arg(long,default_value="natural",value_parser=palette::PERSONALITIES)]
        personality: String,
        #[arg(long,action=ArgAction::SetTrue,overrides_with="no_background_from_wallpaper")]
        background_from_wallpaper: bool,
        #[arg(long,action=ArgAction::SetTrue,overrides_with="background_from_wallpaper")]
        no_background_from_wallpaper: bool,
        #[arg(long)]
        accent: Option<String>,
        #[arg(long)]
        primary: Option<String>,
        #[arg(long)]
        secondary: Option<String>,
        #[arg(long)]
        tertiary: Option<String>,
        #[arg(long, default_value = "0.0", allow_negative_numbers = true)]
        brightness: f64,
        #[arg(long, default_value = "1.0", allow_negative_numbers = true)]
        saturation: f64,
        #[arg(long, default_value = "12", allow_negative_numbers = true, value_parser=integer_hour)]
        hour: i32,
    },
    Export {
        palette: PathBuf,
        directory: PathBuf,
        #[arg(long)]
        templates: Option<PathBuf>,
    },
    Render {
        palette: PathBuf,
        template: PathBuf,
        #[arg(long)]
        companion: Option<PathBuf>,
        #[arg(long)]
        variables: Option<PathBuf>,
    },
}
fn integer_hour(value: &str) -> std::result::Result<i32, String> {
    let unsigned = value.strip_prefix(['+', '-']).unwrap_or(value);
    if unsigned.is_empty()
        || unsigned
            .split('_')
            .any(|part| part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err("invalid integer hour".into());
    }
    let normalized = value.replace('_', "");
    Ok(normalized
        .parse::<i32>()
        .unwrap_or(if value.starts_with('-') { -1 } else { 24 }))
}
pub fn resolve(path: &Path) -> Result<PathBuf> {
    let text = path.to_string_lossy();
    let path = if text == "~" || text.starts_with("~/") {
        PathBuf::from(std::env::var_os("HOME").ok_or("Missing home directory")?)
            .join(text.strip_prefix("~/").unwrap_or(""))
    } else {
        path.to_path_buf()
    };
    Ok(path.canonicalize()?)
}
pub fn from_image(path: &Path, options: &Value) -> Result<Value> {
    let path = resolve(path)?;
    let settings = engine::settings(options)?;
    let identity = storage::identity(&path, &settings)?;
    let folder = options["cache_dir"]
        .as_str()
        .map(PathBuf::from)
        .unwrap_or_else(|| storage::root().join("palettes"));
    let cache = folder.join(format!("rust-{identity}.json"));
    if let Ok(value) = storage::read(&cache) {
        if value["engine"] == engine_id()
            && value["formatVersion"] == orient_core::FORMAT_VERSION
            && value["input"] == settings
            && value["name"] == "dynamic"
            && value["colours"]
                .as_object()
                .is_some_and(|v| palette::validate(v).is_ok())
        {
            return Ok(value);
        }
    }
    let frames = decoder::frames(&path)?;
    let mut analysis = extract::analyze(&frames)?;
    analysis["path"] = json!(path);
    analysis["digest"] = json!(storage::digest(&path)?);
    analysis["engine"] = json!(engine_id());
    let classic = if settings["personality"] == "natural" {
        Some(orient_core::classic::analyze(&frames)?)
    } else {
        None
    };
    let mut result =
        engine::from_analysis(analysis, classic.as_ref(), &settings, options.get("hour"))?;
    result["engine"] = json!(engine_id());
    if storage::identity(&path, &settings)? != identity {
        return fail("Wallpaper changed during extraction; select it again");
    }
    let _ = storage::write(&cache, &result);
    Ok(result)
}
fn run(args: Args) -> Result<()> {
    let result = match args.command {
        Commands::Analyze { image } => extract::analyze(&decoder::frames(&resolve(&image)?)?)?,
        Commands::Sample { image, x, y } => {
            if !x.is_finite()
                || !y.is_finite()
                || !(0.0..=1.0).contains(&x)
                || !(0.0..=1.0).contains(&y)
            {
                return fail("Pick coordinates between zero and one");
            }
            let path = resolve(&image)?;
            let frame = if decoder::is_video(&path) {
                decoder::frames(&path)?.remove(0)
            } else {
                decoder::still(&path)?
            };
            json!({"hex":extract::sample_color(&frame,x,y)?})
        }
        Commands::Palette {
            image,
            mode,
            personality,
            background_from_wallpaper,
            no_background_from_wallpaper,
            accent,
            primary,
            secondary,
            tertiary,
            brightness,
            saturation,
            hour,
        } => {
            if !brightness.is_finite() || !saturation.is_finite() {
                return fail("Color adjustment is outside its safe bounds");
            }
            let mut overrides = serde_json::Map::new();
            for (role, value) in [
                ("primary", primary),
                ("secondary", secondary),
                ("tertiary", tertiary),
            ] {
                if let Some(value) = value.filter(|v| !v.is_empty()) {
                    overrides.insert(role.into(), json!(value));
                }
            }
            let background = if background_from_wallpaper {
                Some(true)
            } else if no_background_from_wallpaper {
                Some(false)
            } else {
                None
            };
            from_image(
                &image,
                &json!({"mode":mode,"personality":personality,"background_from_wallpaper":background,"accent":accent,"overrides":overrides,"brightness":brightness,"saturation":saturation,"hour":hour}),
            )?
        }
        Commands::Export {
            palette,
            directory,
            templates,
        } => {
            json!({"files":export::export(&storage::read(&palette)?,&directory,templates.as_deref(),&serde_json::Map::new(),None)?})
        }
        Commands::Render {
            palette,
            template,
            companion,
            variables,
        } => {
            let palette = storage::read(&palette)?;
            let mut schemes = serde_json::Map::new();
            if let Some(path) = companion {
                let companion = storage::read(&path)?;
                if !matches!(companion["mode"].as_str(), Some("dark" | "light"))
                    || companion["mode"] == palette["mode"]
                {
                    return fail("Companion palette must use the opposite mode");
                }
                schemes.insert(
                    companion["mode"].as_str().unwrap().into(),
                    companion["colours"].clone(),
                );
            }
            let custom = variables.map(|p| storage::read(&p)).transpose()?;
            if custom.as_ref().is_some_and(|v| !v.is_object()) {
                return fail("Custom variables must be a JSON object");
            }
            let text = export::render(
                &fs::read_to_string(template)?,
                &palette,
                &schemes,
                custom.as_ref().and_then(Value::as_object),
            )?;
            use std::io::Write;
            std::io::stdout().write_all(text.as_bytes())?;
            return Ok(());
        }
    };
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}
fn parse_args() -> Args {
    let mut arguments: Vec<std::ffi::OsString> = std::env::args_os().collect();
    let mut choices: Vec<&str> = vec!["--help", "--version"];
    let mut stopped = false;
    for argument in arguments.iter_mut().skip(1) {
        let Some(text) = argument.to_str() else {
            continue;
        };
        if text == "--" {
            stopped = true;
            continue;
        }
        if stopped {
            continue;
        }
        match text {
            "palette" => {
                choices = vec![
                    "--help",
                    "--mode",
                    "--personality",
                    "--background-from-wallpaper",
                    "--no-background-from-wallpaper",
                    "--accent",
                    "--primary",
                    "--secondary",
                    "--tertiary",
                    "--brightness",
                    "--saturation",
                    "--hour",
                ]
            }
            "render" => choices = vec!["--help", "--companion", "--variables"],
            "export" => choices = vec!["--help", "--templates"],
            "analyze" | "sample" => choices = vec!["--help"],
            _ => {}
        }
        if text.starts_with("--") {
            let (flag, value) = text
                .split_once('=')
                .map(|(f, v)| (f, Some(v)))
                .unwrap_or((text, None));
            if !choices.contains(&flag) {
                let matches: Vec<_> = choices
                    .iter()
                    .filter(|choice| choice.starts_with(flag))
                    .collect();
                if matches.len() == 1 {
                    *argument = if let Some(value) = value {
                        format!("{}={value}", matches[0]).into()
                    } else {
                        std::ffi::OsString::from(matches[0])
                    };
                }
            }
        }
    }
    Args::parse_from(arguments)
}

fn main() {
    if ctrlc::set_handler(|| CANCELLED.store(true, std::sync::atomic::Ordering::Relaxed)).is_err() {
        eprintln!("orient: Cannot install cancellation handler");
        std::process::exit(1);
    }
    let arguments: Vec<_> = std::env::args_os().collect();
    if arguments.len() == 2 && arguments[1] == "--rpc" {
        rpc::serve();
        return;
    }
    for arg in std::env::args().skip(1) {
        if [
            "analyze", "sample", "palette", "export", "render", "--help", "-h",
        ]
        .contains(&arg.as_str())
        {
            break;
        }
        if arg.starts_with("--") && "--version".starts_with(&arg) && !"--help".starts_with(&arg) {
            println!("{}", engine_id());
            return;
        }
    }
    if let Err(e) = run(parse_args()) {
        eprintln!("orient: {e}");
        std::process::exit(if CANCELLED.load(std::sync::atomic::Ordering::Relaxed) {
            130
        } else {
            1
        })
    }
}
