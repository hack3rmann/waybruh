use std::{env, sync::LazyLock};

pub const DEFAULT_SCALE: f32 = 1.0;

fn scale_from_var(name: &str) -> Option<f32> {
    let Ok(scale_string) = env::var(name) else {
        return None;
    };

    scale_string.parse::<f32>().ok()
}

pub static GDK_SCALE: LazyLock<Option<f32>> = LazyLock::new(|| scale_from_var("GDK_SCALE"));
pub static SLINT_SCALE: LazyLock<Option<f32>> = LazyLock::new(|| scale_from_var("SLINT_SCALE"));

pub static SCALE_FACTOR: LazyLock<f32> =
    LazyLock::new(|| SLINT_SCALE.or(*GDK_SCALE).unwrap_or(DEFAULT_SCALE));

pub fn get() -> f32 {
    *SCALE_FACTOR
}
