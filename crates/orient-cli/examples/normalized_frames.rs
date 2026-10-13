pub static CANCELLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
#[path = "../src/resample.rs"]
mod resample;
// Developer-only decoder diagnostics, outside the installed CLI surface.
#[path = "../src/decoder.rs"]
mod decoder;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
fn error(s: &str) -> Box<dyn std::error::Error + Send + Sync> {
    std::io::Error::other(s).into()
}
fn fail<T>(s: &str) -> Result<T> {
    Err(error(s))
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let path = std::path::Path::new(&args[0]);
    let out = std::path::Path::new(&args[1]);
    std::fs::create_dir_all(out)?;
    for (i, frame) in decoder::frames(path)?.into_iter().enumerate() {
        image::save_buffer(
            out.join(format!("{i}.png")),
            &frame.rgba,
            frame.width as u32,
            frame.height as u32,
            image::ColorType::Rgba8,
        )?;
    }
    if !decoder::is_video(path)
        && !path
            .extension()
            .is_some_and(|v| v.eq_ignore_ascii_case("gif"))
    {
        let raw = image::ImageReader::open(path)?
            .with_guessed_format()?
            .decode()?;
        raw.save(out.join("raw.png"))?;
    }
    Ok(())
}
