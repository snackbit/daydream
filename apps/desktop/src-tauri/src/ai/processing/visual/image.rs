use super::super::super::*;
use super::types::*;

pub(super) fn file_sha1(path: &Path) -> Result<String, AiError> {
    let bytes = fs::read(path).map_err(file_error)?;
    let mut hasher = Sha1::new();
    hasher.update(bytes);
    Ok(hex::encode(hasher.finalize()))
}

pub(in crate::ai) fn decode_png_rgba(path: &Path) -> Result<VisualImage, AiError> {
    let bytes = fs::read(path).map_err(file_error)?;
    let decoder = png::Decoder::new(Cursor::new(bytes));
    let mut reader = decoder.read_info().map_err(|_error| {
        ai_error(
            "ai_processing_visual_frame_invalid",
            "Extracted visual frame PNG metadata is invalid.",
        )
    })?;
    let buffer_size = reader.output_buffer_size().ok_or_else(|| {
        ai_error(
            "ai_processing_visual_frame_invalid",
            "Extracted visual frame PNG buffer size is invalid.",
        )
    })?;
    let mut buffer = vec![0; buffer_size];
    let info = reader.next_frame(&mut buffer).map_err(|_error| {
        ai_error(
            "ai_processing_visual_frame_invalid",
            "Extracted visual frame PNG pixels are invalid.",
        )
    })?;
    let color = reader.output_color_type();
    if color.1 != png::BitDepth::Eight {
        return Err(ai_error(
            "ai_processing_visual_frame_invalid",
            "Extracted visual frame PNG bit depth is unsupported.",
        ));
    }
    let source = &buffer[..info.buffer_size()];
    let pixels = match color.0 {
        png::ColorType::Rgba => source.to_vec(),
        png::ColorType::Rgb => source
            .chunks_exact(3)
            .flat_map(|pixel| [pixel[0], pixel[1], pixel[2], 255])
            .collect(),
        png::ColorType::Grayscale => source
            .iter()
            .flat_map(|value| [*value, *value, *value, 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => source
            .chunks_exact(2)
            .flat_map(|pixel| [pixel[0], pixel[0], pixel[0], pixel[1]])
            .collect(),
        png::ColorType::Indexed => {
            return Err(ai_error(
                "ai_processing_visual_frame_invalid",
                "Extracted visual frame PNG color type is unsupported.",
            ));
        }
    };
    let width = info.width as usize;
    let height = info.height as usize;
    if width == 0 || height == 0 || pixels.len() != width * height * 4 {
        return Err(ai_error(
            "ai_processing_visual_frame_invalid",
            "Extracted visual frame PNG dimensions are invalid.",
        ));
    }

    Ok(VisualImage {
        height,
        pixels,
        width,
    })
}

pub(super) fn write_visual_crop_png(
    image: &VisualImage,
    region: &VisualRegion,
    path: &Path,
) -> Result<(), AiError> {
    let region = clamp_visual_region(region, image.width as i64, image.height as i64)?;
    let x = region.x as usize;
    let y = region.y as usize;
    let width = region.width as usize;
    let height = region.height as usize;
    let mut pixels = Vec::with_capacity(width * height * 4);
    for row in y..(y + height) {
        let start = ((row * image.width) + x) * 4;
        let end = start + (width * 4);
        pixels.extend_from_slice(&image.pixels[start..end]);
    }
    let file = File::create(path).map_err(file_error)?;
    let mut encoder = png::Encoder::new(file, width as u32, height as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|_error| {
        ai_error(
            "ai_processing_visual_crop_failed",
            "Visual crop PNG metadata could not be written.",
        )
    })?;
    writer.write_image_data(&pixels).map_err(|_error| {
        ai_error(
            "ai_processing_visual_crop_failed",
            "Visual crop PNG pixels could not be written.",
        )
    })
}

pub(in crate::ai) fn visual_delta_regions(
    previous: &VisualImage,
    current: &VisualImage,
    tile_size: usize,
) -> Vec<VisualRegion> {
    if previous.width != current.width || previous.height != current.height {
        return vec![VisualRegion {
            height: current.height as i64,
            width: current.width as i64,
            x: 0,
            y: 0,
        }];
    }
    let tile_size = tile_size.max(1);
    let tiles_x = current.width.div_ceil(tile_size);
    let tiles_y = current.height.div_ceil(tile_size);
    let mut changed_tiles = HashSet::<(usize, usize)>::new();
    for tile_y in 0..tiles_y {
        for tile_x in 0..tiles_x {
            let x_start = tile_x * tile_size;
            let y_start = tile_y * tile_size;
            let x_end = (x_start + tile_size).min(current.width);
            let y_end = (y_start + tile_size).min(current.height);
            let mut changed = 0usize;
            let mut total = 0usize;
            for y in y_start..y_end {
                for x in x_start..x_end {
                    total += 1;
                    if visual_pixel_changed(previous, current, x, y) {
                        changed += 1;
                    }
                }
            }
            let changed_ratio = if total == 0 {
                0.0
            } else {
                changed as f64 / total as f64
            };
            if changed_ratio >= VISUAL_DELTA_TILE_CHANGE_RATIO
                || changed >= VISUAL_DELTA_MIN_CHANGED_PIXELS_PER_TILE.min(total)
            {
                changed_tiles.insert((tile_x, tile_y));
            }
        }
    }

    merge_changed_tiles(
        changed_tiles,
        tiles_x,
        tiles_y,
        tile_size,
        current.width,
        current.height,
    )
}

fn merge_changed_tiles(
    mut changed_tiles: HashSet<(usize, usize)>,
    tiles_x: usize,
    tiles_y: usize,
    tile_size: usize,
    image_width: usize,
    image_height: usize,
) -> Vec<VisualRegion> {
    let mut regions = Vec::new();
    while let Some(&start) = changed_tiles.iter().next() {
        changed_tiles.remove(&start);
        let mut stack = vec![start];
        let mut min_x = start.0;
        let mut max_x = start.0;
        let mut min_y = start.1;
        let mut max_y = start.1;
        while let Some((tile_x, tile_y)) = stack.pop() {
            min_x = min_x.min(tile_x);
            max_x = max_x.max(tile_x);
            min_y = min_y.min(tile_y);
            max_y = max_y.max(tile_y);
            for neighbor in tile_neighbors(tile_x, tile_y, tiles_x, tiles_y) {
                if changed_tiles.remove(&neighbor) {
                    stack.push(neighbor);
                }
            }
        }
        let x = min_x.saturating_sub(1) * tile_size;
        let y = min_y.saturating_sub(1) * tile_size;
        let right = ((max_x + 2).min(tiles_x) * tile_size).min(image_width);
        let bottom = ((max_y + 2).min(tiles_y) * tile_size).min(image_height);
        if right > x && bottom > y {
            regions.push(VisualRegion {
                height: (bottom - y) as i64,
                width: (right - x) as i64,
                x: x as i64,
                y: y as i64,
            });
        }
    }
    regions.sort_by_key(|region| (region.y, region.x));
    regions
}

fn tile_neighbors(
    tile_x: usize,
    tile_y: usize,
    tiles_x: usize,
    tiles_y: usize,
) -> Vec<(usize, usize)> {
    let mut neighbors = Vec::with_capacity(4);
    if tile_x > 0 {
        neighbors.push((tile_x - 1, tile_y));
    }
    if tile_y > 0 {
        neighbors.push((tile_x, tile_y - 1));
    }
    if tile_x + 1 < tiles_x {
        neighbors.push((tile_x + 1, tile_y));
    }
    if tile_y + 1 < tiles_y {
        neighbors.push((tile_x, tile_y + 1));
    }
    neighbors
}

pub(super) fn visual_changed_pixel_ratio(previous: &VisualImage, current: &VisualImage) -> f64 {
    if previous.width != current.width || previous.height != current.height {
        return 1.0;
    }
    let total = current.width * current.height;
    if total == 0 {
        return 0.0;
    }
    let mut changed = 0usize;
    for y in 0..current.height {
        for x in 0..current.width {
            if visual_pixel_changed(previous, current, x, y) {
                changed += 1;
            }
        }
    }
    changed as f64 / total as f64
}

fn visual_pixel_changed(previous: &VisualImage, current: &VisualImage, x: usize, y: usize) -> bool {
    let index = ((y * current.width) + x) * 4;
    let previous_pixel = &previous.pixels[index..index + 3];
    let current_pixel = &current.pixels[index..index + 3];
    previous_pixel
        .iter()
        .zip(current_pixel)
        .map(|(left, right)| (*left as i32 - *right as i32).unsigned_abs())
        .sum::<u32>()
        >= VISUAL_DELTA_PIXEL_DIFF_THRESHOLD
}

pub(super) fn clamp_visual_region(
    region: &VisualRegion,
    frame_width: i64,
    frame_height: i64,
) -> Result<VisualRegion, AiError> {
    if region.width <= 0 || region.height <= 0 || frame_width <= 0 || frame_height <= 0 {
        return Err(ai_error(
            "ai_processing_visual_region_invalid",
            "Visual processing region is invalid.",
        ));
    }
    let x = region.x.clamp(0, frame_width.saturating_sub(1));
    let y = region.y.clamp(0, frame_height.saturating_sub(1));
    let right = (region.x + region.width).clamp(x + 1, frame_width);
    let bottom = (region.y + region.height).clamp(y + 1, frame_height);
    Ok(VisualRegion {
        height: bottom - y,
        width: right - x,
        x,
        y,
    })
}

pub(super) fn source_frame_ref_for_region(
    segment_id: &str,
    timestamp_ms: i64,
    frame_index: usize,
    region: &VisualRegion,
) -> String {
    format!(
        "segment:{segment_id}:offset:{timestamp_ms}:frame:{frame_index}:region:{},{},{},{}",
        region.x, region.y, region.width, region.height
    )
}
