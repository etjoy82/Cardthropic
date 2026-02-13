use std::cell::RefCell;
use std::collections::HashMap;

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
struct DeckSheet {
    sheet: gdk_pixbuf::Pixbuf,
    col_edges: [i32; 14],
    row_edges: [i32; 6],
    texture_cache: RefCell<HashMap<(usize, usize), gdk::Texture>>,
    scaled_texture_cache: RefCell<HashMap<(usize, usize, i32, i32), gdk::Texture>>,
}

const MAX_SCALED_TEXTURE_CACHE_ENTRIES: usize = 512;

impl AngloDeck {
    pub fn load() -> Result<Self, String> {
        let normal =
            DeckSheet::from_svg_bytes(include_bytes!("../data/cards/anglo.svg"), "anglo.svg")?;
        Ok(Self { normal })
    }

    pub fn load_with_custom_normal_svg(custom_svg: &str) -> Result<Self, String> {
        let normal = DeckSheet::from_svg_bytes(custom_svg.as_bytes(), "custom-card-svg")?;
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
}

impl DeckSheet {
    fn from_svg_bytes(svg: &[u8], name: &str) -> Result<Self, String> {
        let tree = usvg::Tree::from_data(svg, &usvg::Options::default())
            .map_err(|err| format!("unable to parse embedded {name}: {err}"))?;

        let size = tree.size().to_int_size();
        let width = i32::try_from(size.width())
            .map_err(|_| format!("rendered SVG width for {name} does not fit i32"))?;
        let height = i32::try_from(size.height())
            .map_err(|_| format!("rendered SVG height for {name} does not fit i32"))?;
        let rowstride = width
            .checked_mul(4)
            .ok_or_else(|| format!("rowstride overflow while rendering {name}"))?;

        let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height())
            .ok_or_else(|| format!("unable to allocate render surface for {name}"))?;
        let mut canvas = pixmap.as_mut();
        resvg::render(&tree, tiny_skia::Transform::default(), &mut canvas);

        let bytes = glib::Bytes::from_owned(pixmap.take());
        let sheet = gdk_pixbuf::Pixbuf::from_bytes(
            &bytes,
            gdk_pixbuf::Colorspace::Rgb,
            true,
            8,
            width,
            height,
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
        })
    }

    fn texture_for_cell(&self, row: usize, col: usize) -> gdk::Texture {
        if let Some(texture) = self.texture_cache.borrow().get(&(row, col)) {
            return texture.clone();
        }

        let x0 = self.col_edges[col];
        let x1 = self.col_edges[col + 1];
        let y0 = self.row_edges[row];
        let y1 = self.row_edges[row + 1];

        let sub = self.sheet.new_subpixbuf(x0, y0, x1 - x0, y1 - y0);
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
            return texture.clone();
        }

        let x0 = self.col_edges[col];
        let x1 = self.col_edges[col + 1];
        let y0 = self.row_edges[row];
        let y1 = self.row_edges[row + 1];

        let sub = self.sheet.new_subpixbuf(x0, y0, x1 - x0, y1 - y0);
        let texture = if let Some(scaled) =
            sub.scale_simple(width, height, gdk_pixbuf::InterpType::Bilinear)
        {
            gdk::Texture::for_pixbuf(&scaled)
        } else {
            gdk::Texture::for_pixbuf(&sub)
        };

        let mut cache = self.scaled_texture_cache.borrow_mut();
        if cache.len() >= MAX_SCALED_TEXTURE_CACHE_ENTRIES {
            cache.clear();
        }
        cache.insert(key, texture.clone());
        texture
    }

    fn pixbuf_for_cell_scaled(
        &self,
        row: usize,
        col: usize,
        width: i32,
        height: i32,
    ) -> gdk_pixbuf::Pixbuf {
        let x0 = self.col_edges[col];
        let x1 = self.col_edges[col + 1];
        let y0 = self.row_edges[row];
        let y1 = self.row_edges[row + 1];

        let sub = self.sheet.new_subpixbuf(x0, y0, x1 - x0, y1 - y0);
        let width = width.max(1);
        let height = height.max(1);
        sub.scale_simple(width, height, gdk_pixbuf::InterpType::Bilinear)
            .unwrap_or(sub)
    }

    pub fn scaled_cache_len(&self) -> usize {
        self.scaled_texture_cache.borrow().len()
    }

    fn clear_scaled_cache(&self) {
        self.scaled_texture_cache.borrow_mut().clear();
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
