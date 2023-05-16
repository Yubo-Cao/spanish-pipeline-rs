/// Convert cm to English metric unit
pub fn cm(cm: f32) -> usize {
    (cm * 360_000.0) as usize
}

/// Convert point to English metric unit
pub fn pixel(point: f32) -> usize {
    (point * 9525.0) as usize
}
