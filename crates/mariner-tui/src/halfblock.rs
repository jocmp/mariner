//! Images as rows of half blocks: each cell's `▀` takes the top pixel as
//! its foreground color and the bottom pixel as its background, so a cell
//! shows two pixels. A preview, not a picture.

use image::RgbaImage;
use image::imageops::FilterType;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

/// The most rows a preview takes.
const MAX_ROWS: u32 = 24;

/// `image` as at most `width` columns and [`MAX_ROWS`] rows of half
/// blocks, keeping its aspect ratio (terminal cells are about twice as
/// tall as wide, which two pixels per cell makes up for).
pub fn lines(image: &RgbaImage, width: u16, true_color: bool) -> Vec<Line<'static>> {
    let (source_width, source_height) = image.dimensions();
    if source_width == 0 || source_height == 0 || width == 0 {
        return Vec::new();
    }
    let mut columns = u32::from(width).min(source_width);
    let mut pixel_rows = (source_height * columns).div_ceil(source_width);
    if pixel_rows > MAX_ROWS * 2 {
        pixel_rows = MAX_ROWS * 2;
        columns = (source_width * pixel_rows).div_ceil(source_height).max(1);
    }
    let pixel_rows = pixel_rows.max(2).div_ceil(2) * 2;
    let resized = image::imageops::resize(image, columns, pixel_rows, FilterType::Triangle);

    (0..pixel_rows / 2)
        .map(|row| {
            let spans = (0..columns)
                .map(|column| {
                    let top = color(resized.get_pixel(column, row * 2).0, true_color);
                    let bottom = color(resized.get_pixel(column, row * 2 + 1).0, true_color);
                    Span::styled("▀", Style::default().fg(top).bg(bottom))
                })
                .collect::<Vec<_>>();
            Line::from(spans)
        })
        .collect()
}

/// Whether the terminal says it shows 24-bit color.
pub fn true_color_terminal() -> bool {
    std::env::var("COLORTERM").is_ok_and(|value| value == "truecolor" || value == "24bit")
}

/// A pixel as a terminal color, over a black background for transparent
/// parts.
fn color([red, green, blue, alpha]: [u8; 4], true_color: bool) -> Color {
    let blend = |channel: u8| (u16::from(channel) * u16::from(alpha) / 255) as u8;
    let (red, green, blue) = (blend(red), blend(green), blend(blue));
    if true_color {
        Color::Rgb(red, green, blue)
    } else {
        Color::Indexed(ansi256(red, green, blue))
    }
}

/// The nearest of the 256 standard colors: the 6×6×6 cube, or the gray
/// ramp for near-grays.
fn ansi256(red: u8, green: u8, blue: u8) -> u8 {
    let level = |channel: u8| ((u16::from(channel) * 5 + 127) / 255) as u8;
    let (max, min) = (red.max(green).max(blue), red.min(green).min(blue));
    if max - min < 16 {
        let average = (u16::from(red) + u16::from(green) + u16::from(blue)) / 3;
        if average < 8 {
            return 16;
        }
        if average > 238 {
            return 231;
        }
        return 232 + ((average - 8) * 24 / 231) as u8;
    }
    16 + 36 * level(red) + 6 * level(green) + level(blue)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    #[test]
    fn two_pixels_per_cell_within_the_width() {
        // 8x4: top half red, bottom half blue.
        let image = RgbaImage::from_fn(8, 4, |_, y| {
            if y < 2 {
                Rgba([255, 0, 0, 255])
            } else {
                Rgba([0, 0, 255, 255])
            }
        });
        let lines = lines(&image, 8, true);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].spans.len(), 8);
        assert_eq!(lines[0].spans[0].style.fg, Some(Color::Rgb(255, 0, 0)));
        assert_eq!(lines[1].spans[0].style.bg, Some(Color::Rgb(0, 0, 255)));

        let narrow = super::lines(&image, 4, true);
        assert_eq!((narrow.len(), narrow[0].spans.len()), (1, 4));
    }

    #[test]
    fn tall_images_are_capped() {
        let image = RgbaImage::new(10, 1_000);
        let lines = lines(&image, 80, true);
        assert_eq!(lines.len(), MAX_ROWS as usize);
        assert!(lines[0].spans.len() <= 10);
    }

    #[test]
    fn maps_to_256_colors() {
        assert_eq!(ansi256(255, 0, 0), 196);
        assert_eq!(ansi256(0, 0, 0), 16);
        assert_eq!(ansi256(128, 128, 128), 232 + 12);
    }
}
