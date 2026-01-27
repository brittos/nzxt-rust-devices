//! Test script to generate radial gauge preview images

use nzxt_rust_devices::utils::stats_image;

fn main() {
    println!("🎨 Generating radial gauge preview images...\n");

    // 45 degrees Celsius (example), 2100 RPM
    let liquid_temp = 45.0;
    let pump_rpm = 2150;

    if let Some(img) =
        stats_image::generate_radial_stats_image(liquid_temp, "LIQUID", pump_rpm, None)
    {
        let path = "tmp/radial_preview_45c.png";
        img.save(path).unwrap();
        println!("Generated {}", path);
    } else {
        eprintln!("Failed to generate image (font missing?)");
    }

    // 0 degrees (min), 0 RPM
    if let Some(img) = stats_image::generate_radial_stats_image(0.0, "LIQUID", 0, None) {
        let path = "tmp/radial_preview_0c.png";
        img.save(path).unwrap();
        println!("Generated {}", path);
    }

    // 100 degrees (max), 2800 RPM
    if let Some(img) = stats_image::generate_radial_stats_image(100.0, "LIQUID", 2800, None) {
        let path = "tmp/radial_preview_100c.png";
        img.save(path).unwrap();
        println!("Generated {}", path);
    }

    // Also generate a full stats version
    println!("\n📊 Generating full radial stats preview...");
    if let Some(img) = stats_image::generate_radial_full_stats_image(35.0, 1800, 1200, 70, 50) {
        match img.save("tmp/radial_full_preview.png") {
            Ok(_) => println!("✅ Generated: tmp/radial_full_preview.png"),
            Err(e) => println!("❌ Failed to save: {}", e),
        }
    }

    println!("\n✅ Preview generation complete!");
    println!("   Check the 'tmp/' folder for the generated images.");
}
