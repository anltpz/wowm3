//! Metin2 envanteri: 5 sütun × 9 satır × 2 sayfa ızgara; eşyalar 1, 2 ya da 3 hücre yüksekliğinde.
//! Ekipman yuvaları ve Yang da burada.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::items::{EquipSlot, Item};

pub const COLS: usize = 5;
pub const ROWS: usize = 9;
pub const PAGES: usize = 2;
pub const CELLS: usize = COLS * ROWS * PAGES;

/// Bir hücre: boş, eşyanın üst hücresi (eşyayı tutar) ya da başka bir eşyanın kapladığı alt hücre.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cell {
    Empty,
    Head(Item),
    /// Üst hücrenin indeksi.
    Tail(u16),
}

#[derive(Resource, Clone, Serialize, Deserialize)]
/// Kayıtta eksik alanlar varsayılandan gelir (eski save.ron uyumu).
#[serde(default)]
pub struct Inventory {
    pub cells: Vec<Cell>,
    pub equip: [Option<Item>; 8],
    pub yang: u64,
    /// Değişiklik sayacı: UI yalnızca değişince yeniden çizer.
    #[serde(skip)]
    pub rev: u64,
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            cells: vec![Cell::Empty; CELLS],
            equip: [None; 8],
            yang: 0,
            rev: 1,
        }
    }
}

/// Hücre indeksi → (sayfa, sütun, satır).
pub fn coords(i: usize) -> (usize, usize, usize) {
    let page = i / (COLS * ROWS);
    let r = i % (COLS * ROWS);
    (page, r % COLS, r / COLS)
}

pub fn index(page: usize, col: usize, row: usize) -> usize {
    page * COLS * ROWS + row * COLS + col
}

impl Inventory {
    pub fn touch(&mut self) {
        self.rev = self.rev.wrapping_add(1);
    }

    /// Hücredeki eşyanın üst hücresi.
    pub fn head_of(&self, i: usize) -> Option<usize> {
        match self.cells.get(i)? {
            Cell::Empty => None,
            Cell::Head(_) => Some(i),
            Cell::Tail(h) => Some(*h as usize),
        }
    }

    pub fn item_at(&self, i: usize) -> Option<Item> {
        let h = self.head_of(i)?;
        match self.cells[h] {
            Cell::Head(it) => Some(it),
            _ => None,
        }
    }

    /// `head` hücresine `height` yüksekliğinde eşya sığar mı (`ignore` = taşınan eşyanın kendisi).
    pub fn fits(&self, head: usize, height: u8, ignore: Option<usize>) -> bool {
        let (page, col, row) = coords(head);
        if row + height as usize > ROWS {
            return false;
        }
        (0..height as usize).all(|k| {
            let i = index(page, col, row + k);
            match self.cells[i] {
                Cell::Empty => true,
                Cell::Head(_) => ignore == Some(i),
                Cell::Tail(h) => ignore == Some(h as usize),
            }
        })
    }

    fn put(&mut self, head: usize, item: Item) {
        let (page, col, row) = coords(head);
        let h = item.def().height as usize;
        self.cells[head] = Cell::Head(item);
        for k in 1..h {
            self.cells[index(page, col, row + k)] = Cell::Tail(head as u16);
        }
    }

    /// Eşyayı kaldırır ve döndürür.
    pub fn take(&mut self, i: usize) -> Option<Item> {
        let head = self.head_of(i)?;
        let Cell::Head(item) = self.cells[head] else {
            return None;
        };
        let (page, col, row) = coords(head);
        for k in 0..item.def().height as usize {
            self.cells[index(page, col, row + k)] = Cell::Empty;
        }
        self.touch();
        Some(item)
    }

    /// Belirli bir hücreye koy (sığarsa).
    pub fn place_at(&mut self, head: usize, item: Item) -> bool {
        if head >= CELLS || !self.fits(head, item.def().height, None) {
            return false;
        }
        self.put(head, item);
        self.touch();
        true
    }

    /// İlk uygun yere ekler; istiflenebilirse önce mevcut yığınlara doldurur. Sığmayan kısmı döner.
    pub fn add(&mut self, mut item: Item) -> Option<Item> {
        let def = item.def();
        if def.stack > 1 {
            for c in self.cells.iter_mut() {
                if let Cell::Head(it) = c {
                    if it.vnum == item.vnum && it.count < def.stack {
                        let room = def.stack - it.count;
                        let n = room.min(item.count);
                        it.count += n;
                        item.count -= n;
                        if item.count == 0 {
                            self.rev = self.rev.wrapping_add(1);
                            return None;
                        }
                    }
                }
            }
        }
        for i in 0..CELLS {
            if self.fits(i, def.height, None) {
                self.put(i, item);
                self.touch();
                return None;
            }
        }
        self.touch();
        Some(item)
    }

    /// Toplam adet.
    pub fn count(&self, vnum: u32) -> u32 {
        self.cells
            .iter()
            .filter_map(|c| match c {
                Cell::Head(it) if it.vnum == vnum => Some(it.count as u32),
                _ => None,
            })
            .sum()
    }

    /// `n` adet tüketir; yetmezse hiçbir şey yapmaz, `false` döner.
    pub fn consume(&mut self, vnum: u32, n: u32) -> bool {
        if self.count(vnum) < n {
            return false;
        }
        let mut left = n;
        for i in 0..CELLS {
            if left == 0 {
                break;
            }
            if let Cell::Head(it) = &mut self.cells[i] {
                if it.vnum == vnum {
                    let k = (it.count as u32).min(left);
                    it.count -= k as u16;
                    left -= k;
                    if it.count == 0 {
                        self.take(i);
                    }
                }
            }
        }
        self.touch();
        true
    }

    /// Bir vnum'un ilk bulunduğu hücre.
    pub fn find(&self, vnum: u32) -> Option<usize> {
        self.cells
            .iter()
            .position(|c| matches!(c, Cell::Head(it) if it.vnum == vnum))
    }

    pub fn equipped(&self, slot: EquipSlot) -> Option<Item> {
        self.equip[slot.index()]
    }

    /// Envanter hücresindeki eşyayı kuşanır; eski eşya envantere döner. Başarı/sebep.
    pub fn equip_from(&mut self, cell: usize, level: u32) -> Result<(), &'static str> {
        let Some(item) = self.item_at(cell) else {
            return Err("Boş hücre");
        };
        let def = item.def();
        let Some(slot) = def.equip_slot() else {
            return Err("Bu eşya kuşanılamaz");
        };
        if level < def.level {
            return Err("Seviyen yetersiz");
        }
        let head = self.head_of(cell).unwrap_or(cell);
        self.take(head);
        if let Some(old) = self.equip[slot.index()].take() {
            // Önce eskinin yerine koymayı dene, sonra herhangi bir yere.
            if !self.place_at(head, old) {
                if let Some(back) = self.add(old) {
                    // Hiç yer yok: geri al.
                    self.equip[slot.index()] = Some(back);
                    self.place_at(head, item);
                    return Err("Envanterde yer yok");
                }
            }
        }
        self.equip[slot.index()] = Some(item);
        self.touch();
        Ok(())
    }

    /// Ekipmanı çıkarıp envantere koyar (`to` verilirse oraya).
    pub fn unequip(&mut self, slot: EquipSlot, to: Option<usize>) -> Result<(), &'static str> {
        let Some(item) = self.equip[slot.index()] else {
            return Err("Yuva boş");
        };
        let placed = match to {
            Some(c) => self.place_at(c, item),
            None => self.add(item).is_none(),
        };
        if !placed {
            return Err("Envanterde yer yok");
        }
        self.equip[slot.index()] = None;
        self.touch();
        Ok(())
    }

    /// Sürükle-bırak: `from` hücresindeki eşyayı `to` hücresine taşı; dolu ise aynı tür yığınla
    /// birleştir ya da (aynı boy ise) yer değiştir.
    pub fn move_item(&mut self, from: usize, to: usize) -> bool {
        let Some(src_head) = self.head_of(from) else {
            return false;
        };
        let Some(item) = self.item_at(src_head) else {
            return false;
        };
        let h = item.def().height;
        if to == src_head {
            return true;
        }
        // Yığın birleştirme.
        if let Some(dst_head) = self.head_of(to) {
            if dst_head != src_head {
                if let (Some(dst), true) = (self.item_at(dst_head), item.def().stack > 1) {
                    if dst.vnum == item.vnum {
                        let max = item.def().stack;
                        let room = max - dst.count;
                        let n = room.min(item.count);
                        if let Cell::Head(d) = &mut self.cells[dst_head] {
                            d.count += n;
                        }
                        if let Cell::Head(s) = &mut self.cells[src_head] {
                            s.count -= n;
                            if s.count == 0 {
                                self.take(src_head);
                            }
                        }
                        self.touch();
                        return true;
                    }
                }
                // Aynı boy tek eşya ile yer değiştir.
                if let Some(dst) = self.item_at(dst_head) {
                    if dst.def().height == h && dst_head == to {
                        self.take(src_head);
                        self.take(dst_head);
                        self.put(dst_head, item);
                        self.put(src_head, dst);
                        self.touch();
                        return true;
                    }
                }
            }
        }
        if self.fits(to, h, Some(src_head)) {
            self.take(src_head);
            self.put(to, item);
            self.touch();
            return true;
        }
        false
    }

    /// Hücredeki yığını böl (yarısı) — Shift+sürükle için.
    pub fn split(&mut self, from: usize, to: usize) -> bool {
        let Some(h) = self.head_of(from) else { return false };
        let Some(item) = self.item_at(h) else { return false };
        if item.count < 2 || !self.fits(to, 1, None) {
            return false;
        }
        let half = item.count / 2;
        if let Cell::Head(s) = &mut self.cells[h] {
            s.count -= half;
        }
        self.put(to, Item::stack(item.vnum, half));
        self.touch();
        true
    }

    /// Hücredeki eşyanın adedini değiştir (yükseltme sonucu vb.).
    pub fn set_at(&mut self, head: usize, item: Item) {
        if let Cell::Head(it) = &mut self.cells[head] {
            *it = item;
            self.touch();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tall_items_occupy_cells() {
        let mut inv = Inventory::default();
        assert!(inv.place_at(0, Item::new(10))); // kılıç 1×2
        assert_eq!(inv.cells[COLS], Cell::Tail(0));
        assert!(!inv.place_at(COLS, Item::new(27001)));
        assert!(inv.move_item(COLS, 1)); // kuyruktan tutup taşı
        assert_eq!(inv.cells[0], Cell::Empty);
        assert!(matches!(inv.cells[1], Cell::Head(_)));
        // 3 boy iki el, 8. satıra sığmaz
        assert!(!inv.place_at(index(0, 0, 7), Item::new(70)));
    }

    #[test]
    fn stacking_and_consume() {
        let mut inv = Inventory::default();
        inv.add(Item::stack(27001, 150));
        inv.add(Item::stack(27001, 100));
        assert_eq!(inv.count(27001), 250);
        assert!(inv.consume(27001, 220));
        assert_eq!(inv.count(27001), 30);
        assert!(!inv.consume(27001, 31));
    }

    #[test]
    fn equip_swap() {
        let mut inv = Inventory::default();
        inv.add(Item::new(10));
        inv.add(Item::new(20));
        let c = inv.find(10).unwrap();
        inv.equip_from(c, 1).unwrap();
        assert_eq!(inv.equipped(EquipSlot::Weapon).unwrap().vnum, 10);
        let c2 = inv.find(20).unwrap();
        assert!(inv.equip_from(c2, 1).is_err()); // seviye 5 gerekir
        inv.equip_from(c2, 5).unwrap();
        assert_eq!(inv.count(10), 1);
    }
}
