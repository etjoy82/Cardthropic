use gtk::gdk;
use gtk::gdk_pixbuf;

use crate::game::Card;

#[derive(Debug, Default)]
pub struct AngloDeck;

#[derive(Debug, Clone, Copy, Default)]
pub struct DeckScaledCacheStats {
    pub hits: u64,
    pub misses: u64,
    pub inserts: u64,
    pub clears: u64,
}

impl AngloDeck {
    pub fn load() -> Result<Self, String> {
        Ok(Self)
    }

    pub fn texture_for_card(&self, card: Card) -> gdk::Texture {
        self.texture_for_card_scaled(card, 70, 108)
    }

    pub fn texture_for_card_scaled(&self, card: Card, width: i32, height: i32) -> gdk::Texture {
        gdk::Texture::for_pixbuf(&Self::face_pixbuf(card, width, height))
    }

    pub fn pixbuf_for_card_scaled(
        &self,
        card: Card,
        width: i32,
        height: i32,
    ) -> gdk_pixbuf::Pixbuf {
        Self::face_pixbuf(card, width, height)
    }

    pub fn back_texture(&self) -> gdk::Texture {
        self.back_texture_scaled(70, 108)
    }

    pub fn back_texture_scaled(&self, width: i32, height: i32) -> gdk::Texture {
        gdk::Texture::for_pixbuf(&Self::back_pixbuf(width, height))
    }

    pub fn back_pixbuf_scaled(&self, width: i32, height: i32) -> gdk_pixbuf::Pixbuf {
        Self::back_pixbuf(width, height)
    }

    pub fn scaled_cache_len(&self) -> usize {
        0
    }

    pub fn clear_scaled_cache(&self) {}

    pub fn scaled_cache_stats(&self) -> DeckScaledCacheStats {
        DeckScaledCacheStats::default()
    }

    fn face_pixbuf(card: Card, width: i32, height: i32) -> gdk_pixbuf::Pixbuf {
        let pixbuf = gdk_pixbuf::Pixbuf::new(
            gdk_pixbuf::Colorspace::Rgb,
            true,
            8,
            width.max(1),
            height.max(1),
        )
        .expect("failed to allocate face card pixbuf");
        let fill = if card.color_red() {
            0xFDF6F6FF
        } else {
            0xF6F8FDFF
        };
        pixbuf.fill(fill);
        pixbuf
    }

    fn back_pixbuf(width: i32, height: i32) -> gdk_pixbuf::Pixbuf {
        let pixbuf = gdk_pixbuf::Pixbuf::new(
            gdk_pixbuf::Colorspace::Rgb,
            true,
            8,
            width.max(1),
            height.max(1),
        )
        .expect("failed to allocate back card pixbuf");
        pixbuf.fill(0x2D3856FF);
        pixbuf
    }
}
