mod s3;
mod types;

use anyhow::{Context as _, Result};
use image::{math::Rect, DynamicImage, GenericImage as _, GenericImageView, Rgba};
use imageproc::drawing::draw_text_mut;
use log::{error, info};
use rusttype::{point, Font, Scale};
use select::{document::Document, predicate::Attr};
use types::*;

const FONT_SIZE: f32 = 72.0;
const ICON_SIZE: Size = Size {
    width: 128,
    height: 128,
};
const ICON_OFFSET: Offset = Offset {
    ancher: Ancher::RightBottom,
    x: -(ICON_SIZE.width as i64) - 32,
    y: -(ICON_SIZE.height as i64) - 32,
};

pub async fn get_ogp_image_buffer(encoded_url: &str) -> Result<Vec<u8>> {
    let url = base64::decode(&encoded_url)?;
    let url = String::from_utf8(url)?;
    info!("get_ogp_image_buffer: url = {url}");
    let buffer = if let Ok(s3_connector) = s3::S3Connector::new().await {
        match s3_connector.get_object(&url).await {
            Ok(bytes) => {
                info!("exists {url} image in S3");
                bytes
            }
            Err(_) => {
                let ogp_info = get_ogp_info(&url).await?;
                let image = create_ogp_image(&ogp_info).await?;
                let mut buffer = Vec::<u8>::new();
                image.write_to(&mut buffer, image::ImageOutputFormat::Png)?;
                match s3_connector.put_object(&encoded_url, &buffer).await {
                    Ok(_) => info!("success put image in S3: url = {url}"),
                    Err(e) => error!("error put in S3: {e:?}"),
                };
                buffer
            }
        }
    } else {
        let ogp_info = get_ogp_info(&url).await?;
        let image = create_ogp_image(&ogp_info).await?;
        let mut buffer = Vec::<u8>::new();
        image.write_to(&mut buffer, image::ImageOutputFormat::Png)?;
        buffer
    };

    Ok(buffer)
}

async fn get_ogp_info(url: &str) -> Result<OgpInfo> {
    let html = reqwest::get(url).await?.text().await?;
    let document = Document::from(html.as_str());
    let title = document
        .find(Attr("property", "og:title"))
        .filter_map(|e| e.attr("content"))
        .next()
        .context("og:title is not found!")?
        .to_string();
    let thumbnail_url = document
        .find(Attr("name", "ogp_thumbnail"))
        .filter_map(|e| e.attr("content"))
        .next()
        .map(|e| e.to_string());

    Ok(OgpInfo {
        title,
        thumbnail_url,
    })
}

async fn create_ogp_image(ogp_info: &OgpInfo) -> Result<DynamicImage> {
    let font = Vec::from(include_bytes!("../assets/KosugiMaru-Regular.ttf") as &[u8]);
    let font = Font::try_from_vec(font).context("failed create font: try from vec")?;
    //let font = {
    //    use std::{fs::File, io::Read};
    //    let mut file = File::open("assets/KosugiMaru-Regular.ttf")?;
    //    let mut buf = Vec::new();
    //    file.read_to_end(&mut buf)?;
    //    Font::try_from_vec(buf).context("failed create font: try from vec")?
    //};

    //let base_image = image::open("assets/ogp_base.png")?;
    //let icon_image = image::open("assets/icon.png")?;
    let base_image = image::load_from_memory(include_bytes!("../assets/ogp_base.png"))?;
    let icon_image = image::load_from_memory(include_bytes!("../assets/icon.png"))?;
    let (base_w, base_h) = base_image.dimensions();
    let mut image = overwrite_image(&base_image, &icon_image, &ICON_OFFSET, Some(ICON_SIZE));

    if let Some(ref thumbnail_url) = ogp_info.thumbnail_url {
        if let Ok(thumbnail) = download_image(&thumbnail_url).await {
            const MAX_THUMBNAIL_HEIGHT: u32 = 128;
            let max_thumbnail_width = base_w / 2;
            let (width, height) = thumbnail.dimensions();
            let mut thumbnail_size = Size { width, height };
            thumbnail_size.resize_to_fit(Size {
                width: max_thumbnail_width,
                height: MAX_THUMBNAIL_HEIGHT,
            });
            let thumbnail_offset = Offset {
                ancher: Ancher::LeftBottom,
                x: 32,
                y: -(thumbnail_size.height as i64) - 32,
            };
            image = overwrite_image(&image, &thumbnail, &thumbnail_offset, Some(thumbnail_size));
        }
    }

    let text_area = Rect {
        x: 100,
        y: 150,
        width: base_w - 200,
        height: base_h - 300,
    };
    let font_scale = Scale {
        x: FONT_SIZE,
        y: FONT_SIZE,
    };
    let lines = split_lines(&ogp_info.title, &font, FONT_SIZE, text_area.width)?;
    let lines = if 5 <= lines.len() {
        let mut lines: Vec<_> = lines.into_iter().take(4).collect();
        let mut last_line: String = lines.pop().context("lines is empty")?;
        last_line.pop().context("last line is empty")?;
        last_line.push_str("…");
        lines.push(last_line);
        lines
    } else {
        lines
    };

    let mut text_offset = Offset {
        ancher: Ancher::LeftTop,
        x: text_area.x as i64,
        y: text_area.y as i64,
    };
    for line in lines.iter() {
        draw_text_mut(
            &mut image,
            Rgba([0, 0, 0, 255]),
            text_offset.x as u32,
            text_offset.y as u32,
            font_scale,
            &font,
            &line,
        );
        text_offset.y += calc_line_height(&line, &font, FONT_SIZE)? as i64;
    }

    Ok(image)
}

async fn download_image(url: &str) -> Result<DynamicImage> {
    let bytes = reqwest::get(url).await?.bytes().await?;
    let img = image::load_from_memory(&bytes)?;
    Ok(img)
}

fn calc_line_height(line_text: &str, font: &Font, font_size: f32) -> Result<u32> {
    let scale = Scale {
        x: font_size,
        y: font_size,
    };
    let point = point(0.0, font.v_metrics(scale).ascent);

    let glyphs: Vec<rusttype::Rect<i32>> = font
        .layout(line_text, scale, point)
        .map(|g| g.pixel_bounding_box())
        .filter(|g| g.is_some())
        .map(|g| g.unwrap())
        .collect();

    let min_y = glyphs
        .iter()
        .map(|g| g.min.y)
        .min()
        .context("glyphs is empty")?;
    let max_y = glyphs
        .iter()
        .map(|g| g.max.y)
        .max()
        .context("glyphs is empty")?;
    Ok((max_y - min_y).try_into()?)
}

fn split_lines(
    text: &str,
    font: &Font,
    font_size: f32,
    text_area_width: u32,
) -> Result<Vec<String>> {
    use lindera::{
        mode::Mode,
        tokenizer::{Tokenizer, TokenizerConfig},
    };
    use std::path::PathBuf;

    let config = TokenizerConfig {
        user_dict_path: Some(PathBuf::from("./assets/userdic.csv")),
        mode: Mode::Normal,
        ..TokenizerConfig::default()
    };
    let tokenizer = Tokenizer::with_config(config)?;
    let tokens = tokenizer.tokenize(text)?;
    let texts = tokens
        .iter()
        .map(|t| t.text.to_string())
        .collect::<Vec<_>>();

    let mut lines = Vec::new();
    let mut skip_count = 0;
    for i in 1..=texts.len() {
        let s = concat_strings(texts.iter().skip(skip_count).take(i - skip_count));
        if is_overflow_x(&s, &font, font_size, text_area_width)? {
            let s = concat_strings(texts.iter().skip(skip_count).take(i - skip_count - 1));
            lines.push(s);
            skip_count = i - 1;
        }
    }
    let s = concat_strings(texts.iter().skip(skip_count));
    lines.push(s);

    Ok(lines)
}

fn concat_strings<'a>(s_iter: impl Iterator<Item = &'a String>) -> String {
    s_iter.fold(String::new(), |mut s, t| {
        s.push_str(&t);
        s
    })
}

fn is_overflow_x(text: &str, font: &Font, scale: f32, text_area_width: u32) -> Result<bool> {
    let scale = Scale { x: scale, y: scale };
    let point = point(0.0, font.v_metrics(scale).ascent);

    let glyphs: Vec<rusttype::Rect<i32>> = font
        .layout(text, scale, point)
        .map(|g| g.pixel_bounding_box())
        .filter(|g| g.is_some())
        .map(|g| g.unwrap())
        .collect();

    let first = glyphs.first().context("glyphs is empty")?.min;
    let last = glyphs.last().context("glyphs is empty")?.max;
    let width = last.x - first.x;
    Ok(text_area_width < width.try_into()?)
}

fn overwrite_image(
    base: &DynamicImage,
    img: &DynamicImage,
    offset: &Offset,
    resize: Option<Size>,
) -> DynamicImage {
    let img = if let Some(Size { width, height }) = resize {
        img.resize(width, height, image::imageops::FilterType::Nearest)
    } else {
        img.clone()
    };

    let (base_w, base_h) = base.dimensions();
    let (base_w, base_h) = (base_w as i64, base_h as i64);
    let overwrite_origin = match offset.ancher {
        Ancher::LeftTop => (offset.x, offset.y),
        Ancher::LeftBottom => (offset.x, base_h + offset.y),
        Ancher::RightBottom => (base_w + offset.x, base_h + offset.y),
    };
    let (w, h) = img.dimensions();
    let (w, h) = (w as i64, h as i64);
    let mut result = base.clone();
    for y in 0..h {
        if base_h <= overwrite_origin.1 + y {
            break;
        }
        for x in 0..w {
            if base_w <= overwrite_origin.0 + x {
                break;
            }
            let p = img.get_pixel(x as u32, y as u32);
            if p.0[3] == 0 {
                continue;
            }
            result.put_pixel(
                (x + overwrite_origin.0) as u32,
                (y + overwrite_origin.1) as u32,
                p,
            );
        }
    }

    result
}
