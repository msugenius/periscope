fn main() {
    ensure_windows_icon();
    tauri_build::build()
}

fn ensure_windows_icon() {
    let icon_path = std::path::Path::new("icons/icon.ico");
    if icon_path.exists() {
        return;
    }

    const SIZE: usize = 32;
    let xor_bytes = SIZE * SIZE * 4;
    let mask_bytes = SIZE * 4;
    let image_bytes = 40 + xor_bytes + mask_bytes;
    let mut icon = Vec::with_capacity(22 + image_bytes);

    icon.extend_from_slice(&0_u16.to_le_bytes());
    icon.extend_from_slice(&1_u16.to_le_bytes());
    icon.extend_from_slice(&1_u16.to_le_bytes());
    icon.extend_from_slice(&[SIZE as u8, SIZE as u8, 0, 0]);
    icon.extend_from_slice(&1_u16.to_le_bytes());
    icon.extend_from_slice(&32_u16.to_le_bytes());
    icon.extend_from_slice(&(image_bytes as u32).to_le_bytes());
    icon.extend_from_slice(&22_u32.to_le_bytes());

    icon.extend_from_slice(&40_u32.to_le_bytes());
    icon.extend_from_slice(&(SIZE as i32).to_le_bytes());
    icon.extend_from_slice(&((SIZE * 2) as i32).to_le_bytes());
    icon.extend_from_slice(&1_u16.to_le_bytes());
    icon.extend_from_slice(&32_u16.to_le_bytes());
    icon.extend_from_slice(&0_u32.to_le_bytes());
    icon.extend_from_slice(&(xor_bytes as u32).to_le_bytes());
    icon.extend_from_slice(&0_i32.to_le_bytes());
    icon.extend_from_slice(&0_i32.to_le_bytes());
    icon.extend_from_slice(&0_u32.to_le_bytes());
    icon.extend_from_slice(&0_u32.to_le_bytes());

    for y in (0..SIZE).rev() {
        for x in 0..SIZE {
            let dx = x as i32 - 16;
            let dy = y as i32 - 16;
            let radius = dx * dx + dy * dy;
            let visible = (49..=81).contains(&radius)
                || (dx.abs() <= 1 && (10..=14).contains(&dy.abs()))
                || (dy.abs() <= 1 && (10..=14).contains(&dx.abs()));
            icon.extend_from_slice(if visible {
                &[255, 232, 53, 255]
            } else {
                &[0, 0, 0, 0]
            });
        }
    }
    icon.resize(22 + image_bytes, 0);

    std::fs::create_dir_all("icons").expect("failed to create icon directory");
    std::fs::write(icon_path, icon).expect("failed to create Windows icon");
}
