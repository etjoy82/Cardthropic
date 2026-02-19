use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;

use gtk::gdk;
use gtk::gdk_pixbuf;
use gtk::glib;
use resvg::{tiny_skia, usvg};

use crate::game::{Card, Suit};

#[derive(Debug)]
pub struct AngloDeck {
    normal: DeckSheet,
}

#[derive(Debug)]
pub struct DeckSheetRaster {
    pixels: Vec<u8>,
    width: i32,
    height: i32,
}

#[derive(Debug)]
struct DeckSheet {
    sheet: gdk_pixbuf::Pixbuf,
    col_edges: [i32; 14],
    row_edges: [i32; 6],
    texture_cache: RefCell<HashMap<(usize, usize), gdk::Texture>>,
    scaled_texture_cache: RefCell<HashMap<(usize, usize, i32, i32), gdk::Texture>>,
    scaled_cache_hits: Cell<u64>,
    scaled_cache_misses: Cell<u64>,
    scaled_cache_inserts: Cell<u64>,
    scaled_cache_clears: Cell<u64>,
}

const MAX_SCALED_TEXTURE_CACHE_ENTRIES: usize = 512;

#[derive(Debug, Clone, Copy, Default)]
pub struct DeckScaledCacheStats {
    pub hits: u64,
    pub misses: u64,
    pub inserts: u64,
    pub clears: u64,
}

impl AngloDeck {
    pub fn load() -> Result<Self, String> {
        let raster =
            DeckSheet::rasterize_svg_bytes(include_bytes!("../data/cards/anglo.svg"), "anglo.svg")
                .or_else(|_| {
                    DeckSheet::rasterize_png_bytes(
                        include_bytes!("../data/cards/anglo.png"),
                        "anglo.png",
                    )
                })?;
        let normal = DeckSheet::from_raster(raster)?;
        Ok(Self { normal })
    }

    pub fn load_with_custom_normal_svg(custom_svg: &str) -> Result<Self, String> {
        let raster = DeckSheet::rasterize_svg_bytes(custom_svg.as_bytes(), "custom-card-svg")?;
        let normal = DeckSheet::from_raster(raster)?;
        Ok(Self { normal })
    }

    pub fn rasterize_default_svg() -> Result<DeckSheetRaster, String> {
        DeckSheet::rasterize_svg_bytes(include_bytes!("../data/cards/anglo.svg"), "anglo.svg")
            .or_else(|_| {
                DeckSheet::rasterize_png_bytes(
                    include_bytes!("../data/cards/anglo.png"),
                    "anglo.png",
                )
            })
    }

    pub fn rasterize_custom_svg(custom_svg: &str) -> Result<DeckSheetRaster, String> {
        if let Some(path) = DeckSheet::custom_svg_cache_path(custom_svg) {
            if let Ok(raster) = DeckSheet::read_cached_raster(&path) {
                return Ok(raster);
            }

            let raster = DeckSheet::rasterize_svg_bytes(custom_svg.as_bytes(), "custom-card-svg")?;
            let _ = DeckSheet::write_cached_raster(&path, &raster);
            return Ok(raster);
        }

        DeckSheet::rasterize_svg_bytes(custom_svg.as_bytes(), "custom-card-svg")
    }

    pub fn from_raster(normal: DeckSheetRaster) -> Result<Self, String> {
        let normal = DeckSheet::from_raster(normal)?;
        Ok(Self { normal })
    }

    fn card_cell(card: Card) -> (usize, usize) {
        let row = match card.suit {
            Suit::Clubs => 0,
            Suit::Diamonds => 1,
            Suit::Hearts => 2,
            Suit::Spades => 3,
        };
        let col = usize::from(card.rank.saturating_sub(1).min(12));
        (row, col)
    }

    pub fn texture_for_card(&self, card: Card) -> gdk::Texture {
        let (row, col) = Self::card_cell(card);
        self.normal.texture_for_cell(row, col)
    }

    pub fn texture_for_card_scaled(&self, card: Card, width: i32, height: i32) -> gdk::Texture {
        let (row, col) = Self::card_cell(card);
        self.normal.texture_for_cell_scaled(row, col, width, height)
    }

    pub fn pixbuf_for_card_scaled(
        &self,
        card: Card,
        width: i32,
        height: i32,
    ) -> gdk_pixbuf::Pixbuf {
        let (row, col) = Self::card_cell(card);
        self.normal.pixbuf_for_cell_scaled(row, col, width, height)
    }

    pub fn back_texture(&self) -> gdk::Texture {
        self.normal.texture_for_cell(4, 2)
    }

    pub fn back_texture_scaled(&self, width: i32, height: i32) -> gdk::Texture {
        self.normal.texture_for_cell_scaled(4, 2, width, height)
    }

    pub fn back_pixbuf_scaled(&self, width: i32, height: i32) -> gdk_pixbuf::Pixbuf {
        self.normal.pixbuf_for_cell_scaled(4, 2, width, height)
    }

    pub fn scaled_cache_len(&self) -> usize {
        self.normal.scaled_cache_len()
    }

    pub fn clear_scaled_cache(&self) {
        self.normal.clear_scaled_cache();
    }

    pub fn scaled_cache_stats(&self) -> DeckScaledCacheStats {
        self.normal.scaled_cache_stats()
    }
}

impl DeckSheet {
    fn cell_bounds(&self, row: usize, col: usize) -> (i32, i32, i32, i32) {
        let x0 = self.col_edges[col];
        let x1 = self.col_edges[col + 1];
        let y0 = self.row_edges[row];
        let y1 = self.row_edges[row + 1];
        (x0, y0, x1 - x0, y1 - y0)
    }

    fn stable_hash64(input: &str) -> u64 {
        const FNV_OFFSET: u64 = 0xcbf29ce484222325;
        const FNV_PRIME: u64 = 0x100000001b3;
        let mut hash = FNV_OFFSET;
        for &b in input.as_bytes() {
            hash ^= u64::from(b);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        hash
    }

    fn custom_svg_cache_path(custom_svg: &str) -> Option<PathBuf> {
        let base = glib::user_cache_dir();
        let hash = Self::stable_hash64(custom_svg);
        let dir = base.join("cardthropic").join("cards");
        Some(dir.join(format!("custom-{hash:016x}.rgba")))
    }

    fn read_cached_raster(path: &PathBuf) -> Result<DeckSheetRaster, String> {
        let data = fs::read(path).map_err(|err| format!("read cache failed: {err}"))?;
        if data.len() < 16 {
            return Err("cached raster too small".to_string());
        }
        if &data[0..4] != b"CTCR" {
            return Err("cached raster magic mismatch".to_string());
        }
        let width = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        let height = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
        let pixels_len = u32::from_le_bytes([data[12], data[13], data[14], data[15]]) as usize;
        let body = &data[16..];
        if body.len() != pixels_len {
            return Err("cached raster length mismatch".to_string());
        }
        let width_i32 =
            i32::try_from(width).map_err(|_| "cached width out of range".to_string())?;
        let height_i32 =
            i32::try_from(height).map_err(|_| "cached height out of range".to_string())?;
        Ok(DeckSheetRaster {
            pixels: body.to_vec(),
            width: width_i32,
            height: height_i32,
        })
    }

    fn write_cached_raster(path: &PathBuf, raster: &DeckSheetRaster) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|err| format!("create cache dir failed: {err}"))?;
        }
        let width =
            u32::try_from(raster.width).map_err(|_| "cache width out of range".to_string())?;
        let height =
            u32::try_from(raster.height).map_err(|_| "cache height out of range".to_string())?;
        let pixels_len = u32::try_from(raster.pixels.len())
            .map_err(|_| "cache pixel payload too large".to_string())?;
        let mut out = Vec::with_capacity(16 + raster.pixels.len());
        out.extend_from_slice(b"CTCR");
        out.extend_from_slice(&width.to_le_bytes());
        out.extend_from_slice(&height.to_le_bytes());
        out.extend_from_slice(&pixels_len.to_le_bytes());
        out.extend_from_slice(&raster.pixels);
        fs::write(path, out).map_err(|err| format!("write cache failed: {err}"))?;
        Ok(())
    }

    fn rasterize_png_bytes(png_bytes: &[u8], name: &str) -> Result<DeckSheetRaster, String> {
        let decoder = png::Decoder::new(Cursor::new(png_bytes));
        let mut reader = decoder
            .read_info()
            .map_err(|err| format!("unable to decode {name}: {err}"))?;
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader
            .next_frame(&mut buf)
            .map_err(|err| format!("unable to read PNG frame for {name}: {err}"))?;
        let width = i32::try_from(info.width)
            .map_err(|_| format!("PNG width for {name} does not fit i32"))?;
        let height = i32::try_from(info.height)
            .map_err(|_| format!("PNG height for {name} does not fit i32"))?;
        let frame_len = info.buffer_size();
        let pixels = match info.color_type {
            png::ColorType::Rgba => {
                buf.truncate(frame_len);
                buf
            }
            png::ColorType::Rgb => {
                let bytes = &buf[..frame_len];
                let mut out = Vec::with_capacity((width as usize) * (height as usize) * 4);
                for chunk in bytes.chunks_exact(3) {
                    out.extend_from_slice(chunk);
                    out.push(255);
                }
                out
            }
            png::ColorType::Grayscale => {
                let bytes = &buf[..frame_len];
                let mut out = Vec::with_capacity((width as usize) * (height as usize) * 4);
                for &g in bytes {
                    out.extend_from_slice(&[g, g, g, 255]);
                }
                out
            }
            png::ColorType::GrayscaleAlpha => {
                let bytes = &buf[..frame_len];
                let mut out = Vec::with_capacity((width as usize) * (height as usize) * 4);
                for chunk in bytes.chunks_exact(2) {
                    let g = chunk[0];
                    out.extend_from_slice(&[g, g, g, chunk[1]]);
                }
                out
            }
            png::ColorType::Indexed => {
                return Err(format!("indexed PNG not supported for {name}"));
            }
        };

        Ok(DeckSheetRaster {
            pixels,
            width,
            height,
        })
    }

    fn rasterize_svg_bytes(svg: &[u8], name: &str) -> Result<DeckSheetRaster, String> {
        let tree = usvg::Tree::from_data(svg, &usvg::Options::default())
            .map_err(|err| format!("unable to parse embedded {name}: {err}"))?;

        let size = tree.size().to_int_size();
        let width = i32::try_from(size.width())
            .map_err(|_| format!("rendered SVG width for {name} does not fit i32"))?;
        let height = i32::try_from(size.height())
            .map_err(|_| format!("rendered SVG height for {name} does not fit i32"))?;

        let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height())
            .ok_or_else(|| format!("unable to allocate render surface for {name}"))?;
        let mut canvas = pixmap.as_mut();
        resvg::render(&tree, tiny_skia::Transform::default(), &mut canvas);

        Ok(DeckSheetRaster {
            pixels: pixmap.take(),
            width,
            height,
        })
    }

    fn from_raster(raster: DeckSheetRaster) -> Result<Self, String> {
        let rowstride = raster
            .width
            .checked_mul(4)
            .ok_or_else(|| "rowstride overflow while creating deck sheet".to_string())?;

        let bytes = glib::Bytes::from_owned(raster.pixels);
        let sheet = gdk_pixbuf::Pixbuf::from_bytes(
            &bytes,
            gdk_pixbuf::Colorspace::Rgb,
            true,
            8,
            raster.width,
            raster.height,
            rowstride,
        );

        let col_edges = compute_col_edges(sheet.width());
        let row_edges = compute_row_edges(sheet.height());

        Ok(Self {
            sheet,
            col_edges,
            row_edges,
            texture_cache: RefCell::new(HashMap::new()),
            scaled_texture_cache: RefCell::new(HashMap::new()),
            scaled_cache_hits: Cell::new(0),
            scaled_cache_misses: Cell::new(0),
            scaled_cache_inserts: Cell::new(0),
            scaled_cache_clears: Cell::new(0),
        })
    }

    fn texture_for_cell(&self, row: usize, col: usize) -> gdk::Texture {
        if let Some(texture) = self.texture_cache.borrow().get(&(row, col)) {
            return texture.clone();
        }

        let (x0, y0, cell_w, cell_h) = self.cell_bounds(row, col);

        let sub = self.sheet.new_subpixbuf(x0, y0, cell_w, cell_h);
        let texture = gdk::Texture::for_pixbuf(&sub);
        self.texture_cache
            .borrow_mut()
            .insert((row, col), texture.clone());
        texture
    }

    fn texture_for_cell_scaled(
        &self,
        row: usize,
        col: usize,
        width: i32,
        height: i32,
    ) -> gdk::Texture {
        let width = width.max(1);
        let height = height.max(1);
        let key = (row, col, width, height);
        if let Some(texture) = self.scaled_texture_cache.borrow().get(&key) {
            self.scaled_cache_hits
                .set(self.scaled_cache_hits.get().saturating_add(1));
            return texture.clone();
        }
        self.scaled_cache_misses
            .set(self.scaled_cache_misses.get().saturating_add(1));

        let scaled = self.pixbuf_for_cell_scaled(row, col, width, height);
        let texture = gdk::Texture::for_pixbuf(&scaled);

        let mut cache = self.scaled_texture_cache.borrow_mut();
        if cache.len() >= MAX_SCALED_TEXTURE_CACHE_ENTRIES {
            cache.clear();
            self.scaled_cache_clears
                .set(self.scaled_cache_clears.get().saturating_add(1));
        }
        cache.insert(key, texture.clone());
        self.scaled_cache_inserts
            .set(self.scaled_cache_inserts.get().saturating_add(1));
        texture
    }

    fn pixbuf_for_cell_scaled(
        &self,
        row: usize,
        col: usize,
        width: i32,
        height: i32,
    ) -> gdk_pixbuf::Pixbuf {
        let width = width.max(1);
        let height = height.max(1);
        let (x0, y0, cell_w, cell_h) = self.cell_bounds(row, col);

        // Prevent atlas neighbor bleed: pad by 1px with duplicated edge pixels
        // before bilinear scaling, then crop back to requested size.
        let pad = 1_i32;
        if cell_w <= 1 || cell_h <= 1 {
            let sub = self
                .sheet
                .new_subpixbuf(x0, y0, cell_w.max(1), cell_h.max(1));
            return sub
                .scale_simple(width, height, gdk_pixbuf::InterpType::Bilinear)
                .unwrap_or(sub);
        }

        let padded_w = cell_w + (pad * 2);
        let padded_h = cell_h + (pad * 2);
        let padded =
            gdk_pixbuf::Pixbuf::new(gdk_pixbuf::Colorspace::Rgb, true, 8, padded_w, padded_h)
                .expect("failed to allocate padded cell pixbuf");
        // Transparent init; content is fully overwritten below.
        padded.fill(0x00000000);

        // Center body.
        self.sheet
            .copy_area(x0, y0, cell_w, cell_h, &padded, pad, pad);
        // Edge rows/cols.
        self.sheet.copy_area(x0, y0, cell_w, 1, &padded, pad, 0);
        self.sheet
            .copy_area(x0, y0 + cell_h - 1, cell_w, 1, &padded, pad, pad + cell_h);
        self.sheet.copy_area(x0, y0, 1, cell_h, &padded, 0, pad);
        self.sheet
            .copy_area(x0 + cell_w - 1, y0, 1, cell_h, &padded, pad + cell_w, pad);
        // Corners.
        self.sheet.copy_area(x0, y0, 1, 1, &padded, 0, 0);
        self.sheet
            .copy_area(x0 + cell_w - 1, y0, 1, 1, &padded, pad + cell_w, 0);
        self.sheet
            .copy_area(x0, y0 + cell_h - 1, 1, 1, &padded, 0, pad + cell_h);
        self.sheet.copy_area(
            x0 + cell_w - 1,
            y0 + cell_h - 1,
            1,
            1,
            &padded,
            pad + cell_w,
            pad + cell_h,
        );

        let scaled_w = width + (pad * 2);
        let scaled_h = height + (pad * 2);
        let Some(scaled_padded) =
            padded.scale_simple(scaled_w, scaled_h, gdk_pixbuf::InterpType::Bilinear)
        else {
            let sub = self.sheet.new_subpixbuf(x0, y0, cell_w, cell_h);
            return sub
                .scale_simple(width, height, gdk_pixbuf::InterpType::Bilinear)
                .unwrap_or(sub);
        };

        let center = scaled_padded.new_subpixbuf(pad, pad, width, height);
        center.copy().unwrap_or(center)
    }

    pub fn scaled_cache_len(&self) -> usize {
        self.scaled_texture_cache.borrow().len()
    }

    fn clear_scaled_cache(&self) {
        self.scaled_texture_cache.borrow_mut().clear();
        self.scaled_cache_clears
            .set(self.scaled_cache_clears.get().saturating_add(1));
    }

    fn scaled_cache_stats(&self) -> DeckScaledCacheStats {
        DeckScaledCacheStats {
            hits: self.scaled_cache_hits.get(),
            misses: self.scaled_cache_misses.get(),
            inserts: self.scaled_cache_inserts.get(),
            clears: self.scaled_cache_clears.get(),
        }
    }
}

fn compute_col_edges(total: i32) -> [i32; 14] {
    let mut edges = [0_i32; 14];
    for (index, edge) in edges.iter_mut().enumerate() {
        *edge = ((index as f64) * (total as f64) / 13.0).round() as i32;
    }
    edges
}

fn compute_row_edges(total: i32) -> [i32; 6] {
    let mut edges = [0_i32; 6];
    for (index, edge) in edges.iter_mut().enumerate() {
        *edge = ((index as f64) * (total as f64) / 5.0).round() as i32;
    }
    edges
}
