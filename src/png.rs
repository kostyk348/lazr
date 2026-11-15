//! Свой PNG-декодер: без зависимостей, только `std`.
//!
//! Зачем: окну нужна иконка (`egui::IconData` — это RGBA-пиксели), а тянуть
//! `image` и весь его лес зависимостей ради 32×32 противоречит правилу «ноль
//! зависимостей, пока не доказано обратное». Здесь ровно то, что нужно PNG:
//! zlib-обёртка с полным inflate (stored + fixed + dynamic Huffman), пять
//! фильтров строк и основные цветовые типы.
//!
//! Чего нет и почему: интерлейс (Adam7) и битовые глубины < 8 не поддержаны —
//! иконки такими не делают, а молчаливая подмена была бы хуже ошибки.

#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Image {
    /// Один пиксель — для проверок и тестов.
    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let index = ((y * self.width + x) * 4) as usize;
        [
            self.rgba[index],
            self.rgba[index + 1],
            self.rgba[index + 2],
            self.rgba[index + 3],
        ]
    }
}

/// Прочитать PNG целиком в RGBA8.
pub fn decode(bytes: &[u8]) -> Result<Image, String> {
    if bytes.len() < 8 || &bytes[..8] != b"\x89PNG\r\n\x1a\n" {
        return Err("это не PNG: нет сигнатуры".to_string());
    }
    let mut pos = 8;
    let mut header: Option<(u32, u32, u8, u8)> = None;
    let mut palette: Vec<[u8; 3]> = Vec::new();
    let mut alpha_palette: Vec<u8> = Vec::new();
    let mut data: Vec<u8> = Vec::new();
    while pos + 8 <= bytes.len() {
        let length =
            u32::from_be_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]])
                as usize;
        let kind = &bytes[pos + 4..pos + 8];
        let start = pos + 8;
        let end = start
            .checked_add(length)
            .ok_or_else(|| "PNG: испорченная длина блока".to_string())?;
        if end + 4 > bytes.len() {
            return Err("PNG: блок обрывается".to_string());
        }
        let body = &bytes[start..end];
        match kind {
            b"IHDR" => {
                if body.len() < 13 {
                    return Err("PNG: короткий IHDR".to_string());
                }
                let width = u32::from_be_bytes([body[0], body[1], body[2], body[3]]);
                let height = u32::from_be_bytes([body[4], body[5], body[6], body[7]]);
                let depth = body[8];
                let color = body[9];
                let interlace = body[12];
                if interlace != 0 {
                    return Err("PNG: интерлейс (Adam7) не поддержан".to_string());
                }
                if depth != 8 && depth != 16 {
                    return Err(format!(
                        "PNG: глубина {depth} бит не поддержана, нужны 8 или 16"
                    ));
                }
                header = Some((width, height, color, depth));
            }
            b"PLTE" => {
                palette = body.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect();
            }
            b"tRNS" => alpha_palette = body.to_vec(),
            b"IDAT" => data.extend_from_slice(body),
            b"IEND" => break,
            _ => {}
        }
        pos = end + 4;
    }
    let (width, height, color, depth) = header.ok_or_else(|| "PNG: нет IHDR".to_string())?;
    if width == 0 || height == 0 || width > 4096 || height > 4096 {
        return Err(format!("PNG: размер {width}x{height} не годится"));
    }
    let channels = match color {
        0 => 1,
        2 => 3,
        3 => 1,
        4 => 2,
        6 => 4,
        other => return Err(format!("PNG: цветовой тип {other} не поддержан")),
    };
    let raw = inflate(&data)?;
    // 16-битные файлы читаем старшим байтом: для иконки этого достаточно.
    let sample = if depth == 16 { 2 } else { 1 };
    let stride = width as usize * channels * sample;
    let expected = (stride + 1) * height as usize;
    if raw.len() < expected {
        return Err(format!(
            "PNG: данных меньше, чем нужно ({}, ждали {expected})",
            raw.len()
        ));
    }
    // Снимаем фильтры построчно.
    let mut pixels = vec![0u8; stride * height as usize];
    for row in 0..height as usize {
        let filter = raw[row * (stride + 1)];
        let line = &raw[row * (stride + 1) + 1..row * (stride + 1) + 1 + stride];
        let (before, current) = pixels.split_at_mut(row * stride);
        let previous = if row == 0 {
            &vec![0u8; stride][..]
        } else {
            &before[(row - 1) * stride..]
        };
        let target = &mut current[..stride];
        unfilter(filter, line, previous, target, channels * sample)?;
    }
    // В RGBA.
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for index in 0..(width * height) as usize {
        let base = index * channels * sample;
        let pixel: Vec<u8> = (0..channels)
            .map(|channel| pixels[base + channel * sample])
            .collect();
        let out = match color {
            0 => [pixel[0], pixel[0], pixel[0], 255],
            2 => [pixel[0], pixel[1], pixel[2], 255],
            3 => {
                let entry = palette
                    .get(pixel[0] as usize)
                    .ok_or_else(|| "PNG: индекс палитры вне PLTE".to_string())?;
                let alpha = alpha_palette.get(pixel[0] as usize).copied().unwrap_or(255);
                [entry[0], entry[1], entry[2], alpha]
            }
            4 => [pixel[0], pixel[0], pixel[0], pixel[1]],
            _ => [pixel[0], pixel[1], pixel[2], pixel[3]],
        };
        rgba.extend_from_slice(&out);
    }
    Ok(Image {
        width,
        height,
        rgba,
    })
}

fn unfilter(
    filter: u8,
    line: &[u8],
    previous: &[u8],
    target: &mut [u8],
    bpp: usize,
) -> Result<(), String> {
    for index in 0..line.len() {
        let left = if index >= bpp { target[index - bpp] } else { 0 };
        let up = previous[index];
        let up_left = if index >= bpp {
            previous[index - bpp]
        } else {
            0
        };
        target[index] = match filter {
            0 => line[index],
            1 => line[index].wrapping_add(left),
            2 => line[index].wrapping_add(up),
            3 => line[index].wrapping_add(((left as u16 + up as u16) / 2) as u8),
            4 => {
                // Фильтр Paeth: берём ближайшего из трёх соседей.
                let p = left as i16 + up as i16 - up_left as i16;
                let pa = (p - left as i16).abs();
                let pb = (p - up as i16).abs();
                let pc = (p - up_left as i16).abs();
                let pred = if pa <= pb && pa <= pc {
                    left
                } else if pb <= pc {
                    up
                } else {
                    up_left
                };
                line[index].wrapping_add(pred)
            }
            other => return Err(format!("PNG: фильтр строки {other} неизвестен")),
        };
    }
    Ok(())
}

// --- zlib / deflate ---------------------------------------------------------

pub fn inflate(zlib: &[u8]) -> Result<Vec<u8>, String> {
    if zlib.len() < 2 {
        return Err("zlib: слишком короткий поток".to_string());
    }
    let cmf = zlib[0];
    let flg = zlib[1];
    if cmf & 0x0f != 8 {
        return Err("zlib: это не deflate".to_string());
    }
    if (cmf as u16 * 256 + flg as u16) % 31 != 0 {
        return Err("zlib: неверная контрольная сумма заголовка".to_string());
    }
    if flg & 0x20 != 0 {
        return Err("zlib: заранее заданный словарь не поддержан".to_string());
    }
    let mut bits = BitReader::new(&zlib[2..]);
    let mut out = Vec::new();
    loop {
        let last = bits.bit()?;
        let kind = bits.bits(2)?;
        match kind {
            0 => {
                // stored: выравниваемся на байт и копируем как есть
                bits.align();
                let len = bits.bits(16)? as usize;
                let nlen = bits.bits(16)? as usize;
                if len != (!nlen & 0xffff) {
                    return Err("deflate: длина stored-блока не сходится".to_string());
                }
                for _ in 0..len {
                    out.push(bits.byte()?);
                }
            }
            1 => {
                let (literals, distances) = fixed_tables();
                blocks(&mut bits, &mut out, &literals, &distances)?;
            }
            2 => {
                let (literals, distances) = dynamic_tables(&mut bits)?;
                blocks(&mut bits, &mut out, &literals, &distances)?;
            }
            _ => return Err("deflate: недопустимый тип блока".to_string()),
        }
        if last == 1 {
            break;
        }
    }
    Ok(out)
}

struct BitReader<'a> {
    data: &'a [u8],
    pos: usize,
    holding: u32,
    count: u32,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            holding: 0,
            count: 0,
        }
    }

    fn bit(&mut self) -> Result<u32, String> {
        if self.count == 0 {
            let byte = *self
                .data
                .get(self.pos)
                .ok_or_else(|| "deflate: поток кончился раньше времени".to_string())?;
            self.pos += 1;
            self.holding = byte as u32;
            self.count = 8;
        }
        let bit = self.holding & 1;
        self.holding >>= 1;
        self.count -= 1;
        Ok(bit)
    }

    fn bits(&mut self, count: u32) -> Result<u32, String> {
        let mut value = 0u32;
        for index in 0..count {
            value |= self.bit()? << index;
        }
        Ok(value)
    }

    fn align(&mut self) {
        self.count = 0;
    }

    fn byte(&mut self) -> Result<u8, String> {
        let byte = *self
            .data
            .get(self.pos)
            .ok_or_else(|| "deflate: поток кончился в stored-блоке".to_string())?;
        self.pos += 1;
        Ok(byte)
    }
}

/// Код Хаффмана: сколько бит и сам код.
#[derive(Clone, Copy, Default)]
struct Code {
    count: u8,
    value: u16,
}

struct Huffman {
    codes: Vec<Code>,
}

impl Huffman {
    /// Канонические коды по длинам, как в RFC 1951.
    fn new(lengths: &[u8]) -> Result<Self, String> {
        let mut codes = vec![Code::default(); lengths.len()];
        let max = *lengths.iter().max().unwrap_or(&0) as usize;
        if max == 0 {
            return Ok(Self { codes });
        }
        let mut bl_count = vec![0u16; max + 1];
        for &len in lengths {
            if len > 0 {
                bl_count[len as usize] += 1;
            }
        }
        let mut next_code = vec![0u16; max + 2];
        let mut code = 0u16;
        for bits in 1..=max {
            code = (code + bl_count[bits - 1]) << 1;
            next_code[bits] = code;
        }
        for (symbol, &len) in lengths.iter().enumerate() {
            if len == 0 {
                continue;
            }
            let len = len as usize;
            let value = next_code[len];
            next_code[len] += 1;
            // Канонический код уже в порядке «старший бит вперёд» — так его и
            // читает декодер (code = code << 1 | bit). Разворачивать не нужно.
            codes[symbol] = Code {
                count: len as u8,
                value,
            };
        }
        Ok(Self { codes })
    }

    fn decode(&self, bits: &mut BitReader) -> Result<usize, String> {
        let mut code = 0u16;
        for len in 1..=15u8 {
            code = (code << 1) | bits.bit()? as u16;
            for (symbol, entry) in self.codes.iter().enumerate() {
                if entry.count == len && entry.value == code {
                    return Ok(symbol);
                }
            }
        }
        Err("deflate: код Хаффмана не найден".to_string())
    }
}

fn fixed_tables() -> (Huffman, Huffman) {
    let mut lengths = vec![0u8; 288];
    for (symbol, slot) in lengths.iter_mut().enumerate() {
        *slot = match symbol {
            0..=143 => 8,
            144..=255 => 9,
            256..=279 => 7,
            _ => 8,
        };
    }
    let literals = Huffman::new(&lengths).expect("fixed-таблица корректна");
    let distances = Huffman::new(&vec![5u8; 30]).expect("fixed-дистанции корректны");
    (literals, distances)
}

fn dynamic_tables(bits: &mut BitReader) -> Result<(Huffman, Huffman), String> {
    let hlit = bits.bits(5)? as usize + 257;
    let hdist = bits.bits(5)? as usize + 1;
    let hclen = bits.bits(4)? as usize + 4;
    const ORDER: [usize; 19] = [
        16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
    ];
    let mut code_lengths = vec![0u8; 19];
    for index in 0..hclen {
        code_lengths[ORDER[index]] = bits.bits(3)? as u8;
    }
    let code_huffman = Huffman::new(&code_lengths)?;
    let mut lengths = Vec::with_capacity(hlit + hdist);
    while lengths.len() < hlit + hdist {
        let symbol = code_huffman.decode(bits)?;
        match symbol {
            0..=15 => lengths.push(symbol as u8),
            16 => {
                let previous = *lengths
                    .last()
                    .ok_or_else(|| "deflate: повтор без предыдущей длины".to_string())?;
                let repeat = 3 + bits.bits(2)? as usize;
                for _ in 0..repeat {
                    lengths.push(previous);
                }
            }
            17 => {
                let repeat = 3 + bits.bits(3)? as usize;
                for _ in 0..repeat {
                    lengths.push(0);
                }
            }
            18 => {
                let repeat = 11 + bits.bits(7)? as usize;
                for _ in 0..repeat {
                    lengths.push(0);
                }
            }
            other => return Err(format!("deflate: символ {other} в таблице длин")),
        }
    }
    let literals = Huffman::new(&lengths[..hlit])?;
    let distances = Huffman::new(&lengths[hlit..hlit + hdist])?;
    Ok((literals, distances))
}

const LENGTH_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LENGTH_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const DIST_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DIST_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];

fn blocks(
    bits: &mut BitReader,
    out: &mut Vec<u8>,
    literals: &Huffman,
    distances: &Huffman,
) -> Result<(), String> {
    loop {
        let symbol = literals.decode(bits)?;
        match symbol {
            0..=255 => out.push(symbol as u8),
            256 => return Ok(()),
            257..=285 => {
                let index = symbol - 257;
                let length =
                    LENGTH_BASE[index] as usize + bits.bits(LENGTH_EXTRA[index] as u32)? as usize;
                let dist_symbol = distances.decode(bits)?;
                if dist_symbol >= DIST_BASE.len() {
                    return Err("deflate: недопустимое расстояние".to_string());
                }
                let distance = DIST_BASE[dist_symbol] as usize
                    + bits.bits(DIST_EXTRA[dist_symbol] as u32)? as usize;
                if distance > out.len() {
                    return Err("deflate: ссылка назад за начало данных".to_string());
                }
                let start = out.len() - distance;
                for offset in 0..length {
                    let byte = out[start + offset];
                    out.push(byte);
                }
            }
            other => return Err(format!("deflate: недопустимый литерал {other}")),
        }
    }
}

/// CRC32 (полином 0xEDB88320) — нужен и для чтения, и для записи PNG.
pub fn crc32(data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (index, slot) in table.iter_mut().enumerate() {
        let mut value = index as u32;
        for _ in 0..8 {
            value = if value & 1 != 0 {
                0xedb8_8320 ^ (value >> 1)
            } else {
                value >> 1
            };
        }
        *slot = value;
    }
    let mut crc = 0xffff_ffffu32;
    for &byte in data {
        crc = table[((crc ^ byte as u32) & 0xff) as usize] ^ (crc >> 8);
    }
    crc ^ 0xffff_ffff
}

/// Adler32 — контрольная сумма zlib-потока.
pub fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in data {
        a = (a + byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

impl Image {
    /// Уменьшение «ближайшим соседом» до вписанного в `limit` размера.
    pub fn fit(&self, limit: u32) -> Image {
        if self.width <= limit && self.height <= limit {
            return self.clone();
        }
        let width = self.width.min(limit);
        let height = self.height.min(limit);
        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            let source_y = (y as u64 * self.height as u64 / height as u64) as u32;
            for x in 0..width {
                let source_x = (x as u64 * self.width as u64 / width as u64) as u32;
                rgba.extend_from_slice(&self.pixel(source_x, source_y));
            }
        }
        Image {
            width,
            height,
            rgba,
        }
    }
}

/// Записать RGBA8 в PNG. Сжатие — несжатые deflate-блоки (stored): файл больше,
/// зато кодировщик не тянет zlib и не может ошибиться в Хаффмане.
pub fn encode_rgba(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    let chunk = |out: &mut Vec<u8>, kind: &[u8], body: &[u8]| {
        out.extend_from_slice(&(body.len() as u32).to_be_bytes());
        out.extend_from_slice(kind);
        out.extend_from_slice(body);
        let crc = crc32(&[kind, body].concat());
        out.extend_from_slice(&crc.to_be_bytes());
    };
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);
    // строки с фильтром 0
    let mut raw = Vec::with_capacity(height as usize * (width as usize * 4 + 1));
    for y in 0..height as usize {
        raw.push(0);
        let line = &rgba[y * width as usize * 4..(y + 1) * width as usize * 4];
        raw.extend_from_slice(line);
    }
    let mut zlib = vec![0x78, 0x01];
    let mut rest = raw.as_slice();
    let mut first = true;
    while !rest.is_empty() || first {
        first = false;
        let take = rest.len().min(65535);
        let (block, tail) = rest.split_at(take);
        let last = tail.is_empty();
        zlib.push(if last { 1 } else { 0 });
        zlib.extend_from_slice(&(block.len() as u16).to_le_bytes());
        zlib.extend_from_slice(&(!(block.len() as u16)).to_le_bytes());
        zlib.extend_from_slice(block);
        rest = tail;
    }
    zlib.extend_from_slice(&adler32(&raw).to_be_bytes());
    chunk(&mut out, b"IDAT", &zlib);
    chunk(&mut out, b"IEND", &[]);
    out
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// Собрать простой RGBA-PNG без внешних инструментов: для тестов иконок.
    pub fn rgba_png(width: u32, height: u32, color: [u8; 4]) -> Vec<u8> {
        let mut out = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        let mut chunk = |kind: &[u8], body: &[u8]| {
            out.extend_from_slice(&(body.len() as u32).to_be_bytes());
            out.extend_from_slice(kind);
            out.extend_from_slice(body);
            let crc = crc32(&[kind, body].concat());
            out.extend_from_slice(&crc.to_be_bytes());
        };
        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&width.to_be_bytes());
        ihdr.extend_from_slice(&height.to_be_bytes());
        ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
        chunk(b"IHDR", &ihdr);
        let mut raw = Vec::new();
        for _ in 0..height {
            raw.push(0);
            for _ in 0..width {
                raw.extend_from_slice(&color);
            }
        }
        let mut zlib = vec![0x78, 0x01];
        let mut rest = raw.as_slice();
        while !rest.is_empty() {
            let take = rest.len().min(64);
            let (block, tail) = rest.split_at(take);
            zlib.push(1);
            zlib.extend_from_slice(&(block.len() as u16).to_le_bytes());
            zlib.extend_from_slice(&(!(block.len() as u16)).to_le_bytes());
            zlib.extend_from_slice(block);
            rest = tail;
        }
        zlib.extend_from_slice(&adler32(&raw).to_be_bytes());
        chunk(b"IDAT", &zlib);
        chunk(b"IEND", &[]);
        out
    }

    /// Готовый PNG 2×2, собранный из несжатых zlib-блоков: проверяем всю цепочку
    /// без внешних инструментов.
    fn tiny_png() -> Vec<u8> {
        let signature: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        let mut out = signature.to_vec();
        let mut chunk = |kind: &[u8], body: &[u8]| {
            out.extend_from_slice(&(body.len() as u32).to_be_bytes());
            out.extend_from_slice(kind);
            out.extend_from_slice(body);
            let crc = crc32(&[kind, body].concat());
            out.extend_from_slice(&crc.to_be_bytes());
        };
        // 2×2, 8 бит, RGBA
        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&2u32.to_be_bytes());
        ihdr.extend_from_slice(&2u32.to_be_bytes());
        ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
        chunk(b"IHDR", &ihdr);
        // строки: фильтр 0 + 4 пикселя
        let raw: Vec<u8> = vec![
            0, 255, 0, 0, 255, 0, 255, 0, 255, //
            0, 0, 0, 255, 255, 255, 255, 255, 128,
        ];
        let mut zlib = vec![0x78, 0x01];
        let mut rest = raw.as_slice();
        while !rest.is_empty() {
            let take = rest.len().min(64);
            let (block, tail) = rest.split_at(take);
            zlib.push(1); // последний блок
            zlib.extend_from_slice(&(block.len() as u16).to_le_bytes());
            zlib.extend_from_slice(&(!(block.len() as u16)).to_le_bytes());
            zlib.extend_from_slice(block);
            rest = tail;
        }
        zlib.extend_from_slice(&adler32(&raw).to_be_bytes());
        chunk(b"IDAT", &zlib);
        chunk(b"IEND", &[]);
        out
    }

    #[test]
    fn encoder_round_trips_through_decoder() {
        let rgba: Vec<u8> = (0..(4 * 4 * 4u32)).map(|index| (index * 7) as u8).collect();
        let png = encode_rgba(4, 4, &rgba);
        let image = decode(&png).expect("свой PNG читается своим декодером");
        assert_eq!((image.width, image.height), (4, 4));
        assert_eq!(image.rgba, rgba);
    }

    #[test]
    fn fit_keeps_aspect_less_than_limit() {
        let image = Image {
            width: 100,
            height: 50,
            rgba: vec![255; 100 * 50 * 4],
        };
        let small = image.fit(48);
        assert_eq!((small.width, small.height), (48, 48));
    }

    #[test]
    fn decodes_rgba_png() {
        let png = tiny_png();
        let image = decode(&png).expect("PNG должен разбираться");
        assert_eq!((image.width, image.height), (2, 2));
        assert_eq!(image.pixel(0, 0), [255, 0, 0, 255]);
        assert_eq!(image.pixel(1, 0), [0, 255, 0, 255]);
        assert_eq!(image.pixel(0, 1), [0, 0, 255, 255]);
        assert_eq!(image.pixel(1, 1), [255, 255, 255, 128]);
    }

    #[test]
    fn rejects_something_that_is_not_png() {
        let error = decode(b"not a png at all").unwrap_err();
        assert!(error.contains("нет сигнатуры"), "{error}");
    }

    #[test]
    fn inflate_reads_stored_blocks() {
        let raw = b"lazr".to_vec();
        let mut zlib = vec![0x78, 0x01, 1];
        zlib.extend_from_slice(&(raw.len() as u16).to_le_bytes());
        zlib.extend_from_slice(&(!(raw.len() as u16)).to_le_bytes());
        zlib.extend_from_slice(&raw);
        zlib.extend_from_slice(&adler32(&raw).to_be_bytes());
        assert_eq!(inflate(&zlib).expect("stored"), raw);
    }
}
