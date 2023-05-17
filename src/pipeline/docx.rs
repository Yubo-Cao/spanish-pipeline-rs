/// Convert cm to English metric unit
pub fn cm(cm: f32) -> u32 {
    (cm * 360_000.0) as u32
}

/// Convert point to English metric unit
pub fn pixel(point: f32) -> u32 {
    (point * 9525.0) as u32
}
