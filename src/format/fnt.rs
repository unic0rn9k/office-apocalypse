use glam::*;

#[derive(Debug)]
pub struct FontGlyph {
    id: char,
    position: UVec2,
    size: UVec2,
    offset: IVec2,
}

#[derive(Debug)]
pub struct FontFace {
    width: usize,
    height: usize,
    line_height: u32,
    base: u32,
    glyphs: Vec<FontGlyph>,
}

pub fn parse(bytes: &[u8]) -> FontFace {
    let ident = |s: &str| {
        s.chars()
            .take_while(|c| c.is_alphabetic())
            .collect::<String>()
    };

    let kv = |s: &str| {
        let key = ident(s);
        assert_eq!(&s[key.len()..key.len() + 1], "=");
        let value: String = s[key.len() + 1..]
            .chars()
            .take_while(|c| !c.is_whitespace())
            .collect();

        (key, value)
    };

    let mut width = None;
    let mut height = None;
    let mut line_height = None;
    let mut base = None;

    let mut glyphs = Vec::default();

    for line in std::str::from_utf8(bytes).unwrap().lines() {
        match line {
            line if line.starts_with("info") => {}
            line if line.starts_with("common") => {
                for (key, value) in line.split_whitespace().skip(1).map(kv) {
                    match key.as_str() {
                        "lineHeight" => line_height = value.parse().ok(),
                        "base" => base = value.parse().ok(),
                        "scaleW" => width = value.parse().ok(),
                        "scaleH" => height = value.parse().ok(),
                        _ => {}
                    }
                }
            }
            line if line.starts_with("chars") => {
                let (_, value) = line
                    .split_whitespace()
                    .skip(1)
                    .map(kv)
                    .find(|(key, _)| key == "count")
                    .unwrap();

                glyphs.reserve_exact(value.parse().unwrap());
            }
            line if line.starts_with("char") => {
                let mut id = None;
                let mut x = None;
                let mut y = None;
                let mut width = None;
                let mut height = None;
                let mut xoffset = None;
                let mut yoffset = None;

                for (key, value) in line.split_whitespace().skip(1).map(kv) {
                    match key.as_str() {
                        "id" => {
                            id = value
                                .parse::<u32>()
                                .map(|c| char::from_u32(c).unwrap())
                                .ok()
                        }
                        "x" => x = value.parse::<u32>().ok(),
                        "y" => y = value.parse::<u32>().ok(),
                        "width" => width = value.parse::<u32>().ok(),
                        "height" => height = value.parse::<u32>().ok(),
                        "xoffset" => xoffset = value.parse::<i32>().ok(),
                        "yoffset" => yoffset = value.parse::<i32>().ok(),
                        _ => {}
                    }
                }

                glyphs.push(FontGlyph {
                    id: id.unwrap(),
                    position: uvec2(x.unwrap(), y.unwrap()),
                    size: uvec2(width.unwrap(), height.unwrap()),
                    offset: ivec2(xoffset.unwrap(), yoffset.unwrap()),
                });
            }
            _ => {}
        }
    }

    FontFace {
        width: width.unwrap(),
        height: height.unwrap(),
        line_height: line_height.unwrap(),
        base: base.unwrap(),
        glyphs,
    }
}
