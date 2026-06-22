use image::imageops::FilterType;
use image::{DynamicImage, ImageBuffer, Rgba};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const APP_ID: &str = "com.ppmuzyk.eaststar";
const SOURCE_ICON: &str = "assets/app-icon.png";
const ICON_SIZES: [u32; 8] = [16, 24, 32, 48, 64, 128, 256, 512];
const MINIQUAD_ICON_SIZES: [u32; 3] = [16, 32, 64];
const GENERATED_ICON_DIR: &str = "assets/generated-icons";

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("missing manifest dir"));
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("missing out dir"));
    let source_icon = manifest_dir.join(SOURCE_ICON);
    let generated_icons_dir = manifest_dir.join(GENERATED_ICON_DIR);

    println!("cargo:rerun-if-changed={}", source_icon.display());

    let source = image::open(&source_icon).unwrap_or_else(|error| {
        panic!(
            "failed to open source icon at {}: {error}",
            source_icon.display()
        )
    });

    let icons_root = out_dir.join("icons");
    for size in ICON_SIZES {
        write_icon_size(&source, &icons_root, size).unwrap_or_else(|error| {
            panic!("failed to build {size}x{size} app icon: {error}")
        });

        write_icon_size(&source, &generated_icons_dir, size).unwrap_or_else(|error| {
            panic!("failed to export {size}x{size} project icon: {error}")
        });
    }

    write_miniquad_icon_module(&source, &out_dir)
        .unwrap_or_else(|error| panic!("failed to build miniquad icon module: {error}"));
}

fn write_icon_size(
    source: &DynamicImage,
    icons_root: &Path,
    size: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let rgba = render_icon_rgba(source, size);
    let canvas = ImageBuffer::<Rgba<u8>, _>::from_raw(size, size, rgba)
        .expect("icon rgba buffer should match target size");

    let icon_dir = icons_root.join(format!("hicolor/{size}x{size}/apps"));
    fs::create_dir_all(&icon_dir)?;
    canvas.save(icon_dir.join(format!("{APP_ID}.png")))?;

    Ok(())
}

fn write_miniquad_icon_module(
    source: &DynamicImage,
    out_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let small = render_icon_rgba(source, MINIQUAD_ICON_SIZES[0]);
    let medium = render_icon_rgba(source, MINIQUAD_ICON_SIZES[1]);
    let big = render_icon_rgba(source, MINIQUAD_ICON_SIZES[2]);

    let module = format!(
        "pub fn app_icon() -> macroquad::miniquad::conf::Icon {{
    macroquad::miniquad::conf::Icon {{
        small: [{}],
        medium: [{}],
        big: [{}],
    }}
}}
",
        bytes_literal(&small),
        bytes_literal(&medium),
        bytes_literal(&big),
    );

    fs::write(out_dir.join("app_icon.rs"), module)?;
    Ok(())
}

fn render_icon_rgba(source: &DynamicImage, size: u32) -> Vec<u8> {
    let resized = source.resize(size, size, FilterType::Lanczos3);
    let x = (size - resized.width()) / 2;
    let y = (size - resized.height()) / 2;

    let mut canvas = ImageBuffer::from_pixel(size, size, Rgba([0, 0, 0, 0]));
    image::imageops::overlay(&mut canvas, &resized.to_rgba8(), i64::from(x), i64::from(y));
    canvas.into_raw()
}

fn bytes_literal(bytes: &[u8]) -> String {
    bytes.iter().map(u8::to_string).collect::<Vec<_>>().join(", ")
}
