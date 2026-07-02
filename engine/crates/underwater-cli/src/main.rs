//! Applies manual color adjustments to an image from the command line.
//!
//! This is the Phase 0 test harness for `underwater-core` — not a shipping
//! product. Every flag defaults to 0.0 (no-op) so individual knobs can be
//! tweaked one at a time.

use clap::Parser;
use underwater_core::fusion::{self, FusionParams};
use underwater_core::{image_io, ColorAdjustments};

#[derive(Parser)]
#[command(name = "underwater-cli")]
struct Args {
    /// Input image path (PNG/JPEG).
    input: String,

    /// Output image path.
    output: String,

    /// One-click auto-correct (Ancuti et al. fusion) instead of manual
    /// adjustments. Ignores all manual flags below.
    #[arg(long)]
    auto: bool,

    /// Blue<->orange shift, -1.0..1.0.
    #[arg(long, default_value_t = 0.0)]
    temperature: f32,

    /// Green<->magenta shift, -1.0..1.0.
    #[arg(long, default_value_t = 0.0)]
    tint: f32,

    /// Exposure change in stops.
    #[arg(long, default_value_t = 0.0)]
    exposure: f32,

    /// Contrast, -1.0..1.0.
    #[arg(long, default_value_t = 0.0)]
    contrast: f32,

    /// Saturation, -1.0..1.0.
    #[arg(long, default_value_t = 0.0)]
    saturation: f32,

    /// Vibrance, -1.0..1.0.
    #[arg(long, default_value_t = 0.0)]
    vibrance: f32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let input = image_io::load(&args.input)?;

    let output = if args.auto {
        fusion::auto_correct(&input, &FusionParams::default())
    } else {
        let adjustments = ColorAdjustments {
            temperature: args.temperature,
            tint: args.tint,
            exposure: args.exposure,
            contrast: args.contrast,
            saturation: args.saturation,
            vibrance: args.vibrance,
        };
        adjustments.apply(&input)
    };

    image_io::save(&output, &args.output)?;
    println!("Wrote {}", args.output);
    Ok(())
}
